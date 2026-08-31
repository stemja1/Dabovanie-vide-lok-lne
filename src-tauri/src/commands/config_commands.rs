use crate::config::app_config::AppConfig;
use std::sync::Mutex;
use tauri::State;

pub struct ConfigState(pub Mutex<AppConfig>);

#[tauri::command]
pub fn get_config(state: State<'_, ConfigState>) -> Result<AppConfig, String> {
    let cfg = state.0.lock().map_err(|e| e.to_string())?;
    Ok(cfg.clone())
}

#[tauri::command]
pub fn save_config(new_config: AppConfig, state: State<'_, ConfigState>) -> Result<(), String> {
    let default_path = AppConfig::get_default_config_path();
    new_config
        .save_to_file(&default_path)
        .map_err(|e| e.to_string())?;
    let mut cfg = state.0.lock().map_err(|e| e.to_string())?;
    *cfg = new_config;
    Ok(())
}

#[tauri::command]
pub fn reset_config_to_default(state: State<'_, ConfigState>) -> Result<AppConfig, String> {
    let default_cfg = AppConfig::default();
    let default_path = AppConfig::get_default_config_path();
    default_cfg
        .save_to_file(&default_path)
        .map_err(|e| e.to_string())?;
    let mut cfg = state.0.lock().map_err(|e| e.to_string())?;
    *cfg = default_cfg.clone();
    Ok(default_cfg)
}

#[tauri::command]
pub fn export_config_toml(state: State<'_, ConfigState>) -> Result<String, String> {
    let cfg = state.0.lock().map_err(|e| e.to_string())?;
    cfg.to_toml_string().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn import_config_toml(
    toml_str: String,
    state: State<'_, ConfigState>,
) -> Result<AppConfig, String> {
    let parsed = AppConfig::from_toml_string(&toml_str).map_err(|e| e.to_string())?;
    let default_path = AppConfig::get_default_config_path();
    parsed
        .save_to_file(&default_path)
        .map_err(|e| e.to_string())?;
    let mut cfg = state.0.lock().map_err(|e| e.to_string())?;
    *cfg = parsed.clone();
    Ok(parsed)
}
