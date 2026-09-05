use crate::config::app_config::AppConfig;
use crate::error::{AppError, AppResult};
use std::sync::Mutex;
use tauri::State;

pub struct ConfigState(pub Mutex<AppConfig>);

#[tauri::command]
pub fn get_config(state: State<'_, ConfigState>) -> AppResult<AppConfig> {
    let cfg = state.0.lock().map_err(|_| AppError::LockPoisoned)?;
    Ok(cfg.clone())
}

#[tauri::command]
pub fn save_config(new_config: AppConfig, state: State<'_, ConfigState>) -> AppResult<()> {
    let default_path = AppConfig::get_default_config_path();
    new_config.save_to_file(&default_path)?;
    let mut cfg = state.0.lock().map_err(|_| AppError::LockPoisoned)?;
    *cfg = new_config;
    Ok(())
}

#[tauri::command]
pub fn reset_config_to_default(state: State<'_, ConfigState>) -> AppResult<AppConfig> {
    let default_cfg = AppConfig::default();
    let default_path = AppConfig::get_default_config_path();
    default_cfg.save_to_file(&default_path)?;
    let mut cfg = state.0.lock().map_err(|_| AppError::LockPoisoned)?;
    *cfg = default_cfg.clone();
    Ok(default_cfg)
}

#[tauri::command]
pub fn export_config_toml(state: State<'_, ConfigState>) -> AppResult<String> {
    let cfg = state.0.lock().map_err(|_| AppError::LockPoisoned)?;
    Ok(cfg.to_toml_string()?)
}

#[tauri::command]
pub fn import_config_toml(
    toml_str: String,
    state: State<'_, ConfigState>,
) -> AppResult<AppConfig> {
    let parsed = AppConfig::from_toml_string(&toml_str)?;
    let default_path = AppConfig::get_default_config_path();
    parsed.save_to_file(&default_path)?;
    let mut cfg = state.0.lock().map_err(|_| AppError::LockPoisoned)?;
    *cfg = parsed.clone();
    Ok(parsed)
}
