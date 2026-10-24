use std::fs::{read_dir, read_to_string, File, OpenOptions};
use std::io::Read;
use std::os::fd::AsRawFd;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use serde_json::json;
use tauri::{Emitter, Manager, WebviewWindow};

const PRESSURE_SCRIPT_KEY: &str = "__EXCALIDRAW_NATIVE_POINTER_PRESSURE__";
const ABS_PRESSURE: u16 = 24;
const EV_ABS: u16 = 3;
const PRESSURE_FRESHNESS_MS: u64 = 100;
const XENX_MAX_PRESSURE: f64 = 8191.0;

#[repr(C)]
struct LinuxInputEvent {
  _time: libc::timeval,
  event_type: u16,
  code: u16,
  value: i32,
}

#[repr(C)]
#[derive(Default)]
struct LinuxInputAbsInfo {
  value: i32,
  minimum: i32,
  maximum: i32,
  _fuzz: i32,
  _flat: i32,
  _resolution: i32,
}

struct DriverPressure {
  pressure_bits: AtomicU32,
  last_update_ms: AtomicU64,
  started: Instant,
}

impl DriverPressure {
  fn new() -> Arc<Self> {
    Arc::new(Self {
      pressure_bits: AtomicU32::new(0.5f32.to_bits()),
      last_update_ms: AtomicU64::new(0),
      started: Instant::now(),
    })
  }

  fn update(&self, pressure: f64) {
    self
      .pressure_bits
      .store((pressure.clamp(0.0, 1.0) as f32).to_bits(), Ordering::Relaxed);
    self.last_update_ms.store(
      self.started.elapsed().as_millis() as u64,
      Ordering::Release,
    );
  }

  fn current(&self) -> Option<f64> {
    let now = self.started.elapsed().as_millis() as u64;
    let last_update = self.last_update_ms.load(Ordering::Acquire);
    if now.saturating_sub(last_update) > PRESSURE_FRESHNESS_MS {
      return None;
    }

    Some(f32::from_bits(self.pressure_bits.load(Ordering::Relaxed)) as f64)
  }
}

fn pressure_ioctl_request() -> libc::c_ulong {
  // Linux: EVIOCGABS(abs) = _IOR('E', 0x40 + abs, struct input_absinfo)
  (2 << 30)
    | ((std::mem::size_of::<LinuxInputAbsInfo>() as libc::c_ulong) << 16)
    | ((b'E' as libc::c_ulong) << 8)
    | (0x40 + ABS_PRESSURE as libc::c_ulong)
}

fn pressure_range(file: &File) -> Option<(i32, i32)> {
  let mut info = LinuxInputAbsInfo::default();
  let result = unsafe { libc::ioctl(file.as_raw_fd(), pressure_ioctl_request(), &mut info) };
  if result < 0 || info.maximum <= info.minimum {
    return None;
  }
  Some((info.minimum, info.maximum))
}

fn tablet_event_devices() -> Vec<(PathBuf, (i32, i32))> {
  let Ok(entries) = read_dir("/sys/class/input") else {
    return Vec::new();
  };

  entries
    .filter_map(Result::ok)
    .filter_map(|entry| {
      let event_name = entry.file_name().to_string_lossy().to_string();
      if !event_name.starts_with("event") {
        return None;
      }

      let name = read_to_string(entry.path().join("device/name")).ok()?;
      let name = name.trim().to_ascii_lowercase();
      let is_tablet = ["tablet", "stylus", "pen", "wacom", "huion", "xppen", "xp-pen"]
        .iter()
        .any(|term| name.contains(term));
      if !is_tablet {
        return None;
      }

      let path = PathBuf::from("/dev/input").join(&event_name);
      let file = OpenOptions::new().read(true).open(&path).ok()?;
      let range = pressure_range(&file)?;
      Some((path, range))
    })
    .collect()
}

fn start_driver_reader(driver_pressure: &Arc<DriverPressure>) {
  let devices = tablet_event_devices();
  log::info!("Tablet evdev pressure devices: {}", devices.len());

  for (path, (minimum, maximum)) in devices {
    let driver_pressure = Arc::clone(driver_pressure);
    log::info!("Starting evdev pressure reader: {:?}", path);
    let _ = std::thread::Builder::new()
      .name("tablet-pressure".to_string())
      .spawn(move || {
        let Ok(mut file) = OpenOptions::new().read(true).open(path) else {
          return;
        };
        let mut event = LinuxInputEvent {
          _time: libc::timeval {
            tv_sec: 0,
            tv_usec: 0,
          },
          event_type: 0,
          code: 0,
          value: 0,
        };
        let event_bytes = std::mem::size_of::<LinuxInputEvent>();

        loop {
          let bytes = unsafe {
            std::slice::from_raw_parts_mut(
              (&mut event as *mut LinuxInputEvent).cast::<u8>(),
              event_bytes,
            )
          };
          if file.read_exact(bytes).is_err() {
            break;
          }

          if event.event_type == EV_ABS && event.code == ABS_PRESSURE {
            let pressure =
              (event.value - minimum) as f64 / (maximum - minimum) as f64;
            driver_pressure.update(pressure);
          }
        }
      });
  }
}

fn hidraw_devices() -> Vec<PathBuf> {
  let Ok(entries) = read_dir("/dev/input/by-id") else {
    return Vec::new();
  };

  entries
    .filter_map(Result::ok)
    .filter_map(|entry| {
      let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
      if !name.contains("hidraw") || name.contains("if01") {
        return None;
      }

      let is_tablet = ["tablet", "stylus", "wacom", "huion", "xenx", "xppen", "xp-pen"]
        .iter()
        .any(|term| name.contains(term));
      is_tablet.then(|| entry.path())
    })
    .collect()
}

fn start_hidraw_reader<R: tauri::Runtime>(
  driver_pressure: &Arc<DriverPressure>,
  app: &tauri::AppHandle<R>,
) {
  let devices = hidraw_devices();
  log::info!("Tablet HID pressure devices: {:?}", devices);

  for path in devices {
    let driver_pressure = Arc::clone(driver_pressure);
    let app = app.clone();
    log::info!("Starting HID pressure reader: {:?}", path);
    let _ = std::thread::Builder::new()
      .name("tablet-hidraw-pressure".to_string())
      .spawn(move || {
        let Ok(mut file) = OpenOptions::new().read(true).open(&path) else {
          log::warn!("Could not open tablet HID device: {:?}", path);
          return;
        };
        let mut report = [0_u8; 128];
        let mut last_emitted_pressure = -1.0;
        let mut last_emitted_at = Instant::now();
        let mut logged_first_report = false;

        loop {
          let Ok(length) = file.read(&mut report) else {
            break;
          };

          // XENX X1-640 report format (OpenTabletDriver):
          // report[0] = report id, report[1] = in-range flags,
          // report[6..8] = little-endian pen pressure (0..8191).
          if length >= 8 && report[0] == 0x01 && report[1] != 0 {
            let pressure = u16::from_le_bytes([report[6], report[7]]) as f64
              / XENX_MAX_PRESSURE;
            driver_pressure.update(pressure);

            if !logged_first_report {
              log::info!(
                "Received tablet HID pressure report: raw={}, normalized={:.3}",
                u16::from_le_bytes([report[6], report[7]]),
                pressure,
              );
              logged_first_report = true;
            }

            if last_emitted_at.elapsed().as_millis() >= 4
              || (pressure - last_emitted_pressure).abs() >= 0.005
            {
              if let Err(error) = app.emit("tablet:pressure", pressure) {
                log::warn!("Could not emit tablet pressure event: {error}");
              }
              last_emitted_pressure = pressure;
              last_emitted_at = Instant::now();
            }
          }
        }
      });
  }
}

fn publish_pressure(
  webview: &webkit2gtk::WebView,
  position: (f64, f64),
  time: u32,
  pressure: Option<f64>,
  phase: &str,
) {
  use webkit2gtk::WebViewExt;

  let value = json!({
    "x": position.0,
    "y": position.1,
    "time": time,
    "phase": phase,
    "pressure": pressure.filter(|value| value.is_finite()).map(|value| value.clamp(0.0, 1.0)),
  });
  let script = format!("window.{PRESSURE_SCRIPT_KEY}={value};");

  webview.evaluate_javascript(
    &script,
    None,
    None,
    None::<&webkit2gtk::gio::Cancellable>,
    |_| {},
  );
}

pub fn install<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> tauri::Result<()> {
  let Some(window): Option<WebviewWindow<R>> = app.get_webview_window("main") else {
    return Ok(());
  };
  let driver_pressure = DriverPressure::new();
  start_driver_reader(&driver_pressure);
  start_hidraw_reader(&driver_pressure, app);

  window.with_webview(move |webview| {
    use gtk::gdk::{AxisUse, EventMask};
    use gtk::glib::Propagation;
    use gtk::prelude::*;

    let view = webview.inner();
    view.add_events(
      EventMask::POINTER_MOTION_MASK
        | EventMask::BUTTON_PRESS_MASK
        | EventMask::BUTTON_RELEASE_MASK,
    );

    let button_press_driver = Arc::clone(&driver_pressure);
    view.connect_button_press_event(move |view, event| {
      publish_pressure(
        view,
        event.position(),
        event.time(),
        button_press_driver
          .current()
          .or_else(|| event.axis(AxisUse::Pressure)),
        "down",
      );
      Propagation::Proceed
    });

    let motion_driver = Arc::clone(&driver_pressure);
    view.connect_motion_notify_event(move |view, event| {
      let pressure = motion_driver
        .current()
        .or_else(|| event.axis(AxisUse::Pressure));
      if pressure.is_some() {
        publish_pressure(view, event.position(), event.time(), pressure, "move");
      }
      Propagation::Proceed
    });

    let button_release_driver = Arc::clone(&driver_pressure);
    view.connect_button_release_event(move |view, event| {
      publish_pressure(
        view,
        event.position(),
        event.time(),
        button_release_driver
          .current()
          .or_else(|| event.axis(AxisUse::Pressure)),
        "up",
      );
      Propagation::Proceed
    });
  })?;

  Ok(())
}
