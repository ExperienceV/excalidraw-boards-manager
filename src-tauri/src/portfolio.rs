use serde::{Deserialize, Serialize};
use std::{
  fs,
  path::PathBuf,
  sync::Mutex,
  time::{SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Manager};

const STORE_FILE: &str = "portfolios.json";
static STORE_LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Board {
  pub id: String,
  pub name: String,
  #[serde(default)]
  pub scene: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Portfolio {
  pub id: String,
  pub name: String,
  pub boards: Vec<Board>,
}

fn store_path(app: &AppHandle) -> Result<PathBuf, String> {
  let directory = app
    .path()
    .app_data_dir()
    .map_err(|error| error.to_string())?;

  fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
  Ok(directory.join(STORE_FILE))
}

fn read_portfolios(app: &AppHandle) -> Result<Vec<Portfolio>, String> {
  let path = store_path(app)?;

  if !path.exists() {
    return Ok(Vec::new());
  }

  let content = fs::read_to_string(path).map_err(|error| error.to_string())?;
  serde_json::from_str(&content).map_err(|error| error.to_string())
}

fn write_portfolios(app: &AppHandle, portfolios: &[Portfolio]) -> Result<(), String> {
  let path = store_path(app)?;
  let content = serde_json::to_string_pretty(portfolios).map_err(|error| error.to_string())?;
  let temporary_path = path.with_extension("json.tmp");

  fs::write(&temporary_path, content).map_err(|error| error.to_string())?;
  fs::rename(temporary_path, path).map_err(|error| error.to_string())
}

fn new_id(prefix: &str) -> String {
  let timestamp = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap_or_default()
    .as_millis();

  format!("{prefix}-{timestamp}")
}

#[tauri::command]
pub fn list_portfolios(app: AppHandle) -> Result<Vec<Portfolio>, String> {
  read_portfolios(&app)
}

#[tauri::command]
pub fn create_portfolio(app: AppHandle, name: String) -> Result<Portfolio, String> {
  let name = name.trim();

  if name.is_empty() {
    return Err("The portfolio name cannot be empty".into());
  }

  let _lock = STORE_LOCK.lock().map_err(|error| error.to_string())?;
  let mut portfolios = read_portfolios(&app)?;
  let portfolio = Portfolio {
    id: new_id("portfolio"),
    name: name.to_string(),
    boards: Vec::new(),
  };

  portfolios.push(portfolio.clone());
  write_portfolios(&app, &portfolios)?;
  Ok(portfolio)
}

#[tauri::command]
pub fn create_board(
  app: AppHandle,
  portfolio_id: String,
  name: String,
) -> Result<Board, String> {
  let name = name.trim();

  if name.is_empty() {
    return Err("The board name cannot be empty".into());
  }

  let _lock = STORE_LOCK.lock().map_err(|error| error.to_string())?;
  let mut portfolios = read_portfolios(&app)?;
  let portfolio = portfolios
    .iter_mut()
    .find(|portfolio| portfolio.id == portfolio_id)
    .ok_or_else(|| "Portfolio not found".to_string())?;

  let board = Board {
    id: new_id("board"),
    name: name.to_string(),
    scene: None,
  };

  portfolio.boards.push(board.clone());
  write_portfolios(&app, &portfolios)?;
  Ok(board)
}

#[tauri::command]
pub fn get_board_scene(
  app: AppHandle,
  board_id: String,
) -> Result<Option<serde_json::Value>, String> {
  let portfolios = read_portfolios(&app)?;
  let board = portfolios
    .iter()
    .flat_map(|portfolio| &portfolio.boards)
    .find(|board| board.id == board_id)
    .ok_or_else(|| "Board not found".to_string())?;

  Ok(board.scene.clone())
}

#[tauri::command]
pub fn save_board_scene(
  app: AppHandle,
  board_id: String,
  scene: serde_json::Value,
) -> Result<(), String> {
  let _lock = STORE_LOCK.lock().map_err(|error| error.to_string())?;
  let mut portfolios = read_portfolios(&app)?;
  let board = portfolios
    .iter_mut()
    .flat_map(|portfolio| &mut portfolio.boards)
    .find(|board| board.id == board_id)
    .ok_or_else(|| "Board not found".to_string())?;

  board.scene = Some(scene);
  write_portfolios(&app, &portfolios)
}
