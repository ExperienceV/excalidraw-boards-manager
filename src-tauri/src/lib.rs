mod portfolio;

#[cfg(target_os = "linux")]
mod pressure;

#[cfg(desktop)]
use tauri::{
  menu::{Menu, MenuBuilder, SubmenuBuilder},
  Emitter,
};

#[cfg(desktop)]
fn build_menu(app: &tauri::AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
  let portfolios = portfolio::list_portfolios(app.clone()).unwrap_or_default();

  let file_menu = SubmenuBuilder::new(app, "File")
    .text("new_board_file", "New board")
    .separator()
    .quit()
    .build()?;

  let mut portfolio_menu = SubmenuBuilder::new(app, "Portfolios")
    .text("portfolio_manager", "Open portfolio manager")
    .text("new_portfolio", "New portfolio")
    .text("new_board", "New board");

  if !portfolios.is_empty() {
    portfolio_menu = portfolio_menu.separator();
  }

  for portfolio in portfolios {
    let mut portfolio_submenu =
      SubmenuBuilder::with_id(app, format!("portfolio:{}", portfolio.id), &portfolio.name);

    for board in portfolio.boards {
      portfolio_submenu =
        portfolio_submenu.text(format!("board:{}", board.id), &board.name);
    }

    portfolio_menu = portfolio_menu.item(&portfolio_submenu.build()?);
  }

  let portfolio_menu = portfolio_menu.build()?;

  MenuBuilder::new(app)
    .item(&file_menu)
    .item(&portfolio_menu)
    .build()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .invoke_handler(tauri::generate_handler![
      portfolio::list_portfolios,
      portfolio::create_portfolio,
      portfolio::create_board,
      portfolio::get_board_scene,
      portfolio::save_board_scene,
      refresh_menu,
    ])
    .setup(|app| {
      #[cfg(desktop)]
      {
        app.set_menu(build_menu(app.handle())?)?;
        app.on_menu_event(|app, event| {
          let id = event.id().0.to_string();

          if let Some(board_id) = id.strip_prefix("board:") {
            let _ = app.emit("portfolio:open-board", board_id.to_string());
          } else {
            let _ = app.emit("portfolio:menu", id);
          }
        });
      }

      if cfg!(debug_assertions) {
        app.handle().plugin(
          tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
          .build(),
        )?;
      }

      #[cfg(target_os = "linux")]
      pressure::install(app.handle())?;

      Ok(())
    })
    .run(tauri::generate_context!())
    .expect("error while building tauri application");
}

#[tauri::command]
fn refresh_menu(app: tauri::AppHandle) -> Result<(), String> {
  #[cfg(desktop)]
  {
    app
      .set_menu(build_menu(&app).map_err(|error| error.to_string())?)
      .map_err(|error| error.to_string())?;
  }

  Ok(())
}
