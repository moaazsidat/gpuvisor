use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub inline_metrics: Vec<InlineMetric>,
    pub inline_gpu_mode: InlineGpuMode,
    pub selected_gpu_index: Option<u32>,
    pub poll_interval_ms: u64,
    pub temp_unit: TempUnit,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum InlineMetric {
    Utilization,
    Temperature,
    FanSpeed,
    MemoryUsage,
    PowerUsage,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum InlineGpuMode {
    All,
    Aggregate,
    Single,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TempUnit {
    C,
    F,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            inline_metrics: vec![InlineMetric::Utilization],
            inline_gpu_mode: InlineGpuMode::All,
            selected_gpu_index: None,
            poll_interval_ms: 1000,
            temp_unit: TempUnit::C,
        }
    }
}

impl Settings {
    fn config_path(app_handle: &tauri::AppHandle) -> PathBuf {
        let config_dir = app_handle
            .path()
            .app_config_dir()
            .expect("Failed to get app config dir");
        fs::create_dir_all(&config_dir).ok();
        config_dir.join("settings.json")
    }

    pub fn load(app_handle: &tauri::AppHandle) -> Self {
        let path = Self::config_path(app_handle);
        if let Ok(data) = fs::read_to_string(&path) {
            serde_json::from_str(&data).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    pub fn save(&self, app_handle: &tauri::AppHandle) -> Result<(), String> {
        let path = Self::config_path(app_handle);
        let data =
            serde_json::to_string_pretty(self).map_err(|e| format!("Serialize error: {}", e))?;
        fs::write(&path, data).map_err(|e| format!("Write error: {}", e))?;
        Ok(())
    }
}

use tauri::Manager;
