//! 应用配置：持久化用户自定义的全局快捷键与输入源切换设置。
//! 配置文件：%APPDATA%\ProjectionSwitcher.json（损坏时回退默认值；
//! 旧版本配置缺少新字段时由 serde 默认值填充）。

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 默认全局快捷键（触发"一键切换"投影模式）。
pub const DEFAULT_SHORTCUT: &str = "Ctrl+Alt+D";
/// 默认全局快捷键（触发"切换输入源"）。
pub const DEFAULT_INPUT_SHORTCUT: &str = "Ctrl+Alt+I";
/// 默认目标显示器（EDID Monitor Name，迁移自旧脚本配置）。
pub const DEFAULT_INPUT_MONITOR: &str = "XMI27B1";
/// 默认输入源切换值列表（VCP 0x60 的值）。注意：ControlMyMonitor 的
/// VCP code 为十六进制（"60"=0x60），但 value 参数为**十进制**——
/// 迁移自旧脚本的 "18 15" 即 18(0x12, HDMI) 与 15(0x0F, DisplayPort)。
pub const DEFAULT_INPUT_VALUES: [u32; 2] = [18, 15];
/// 前一版本误将脚本值按十六进制解析（"18"->0x18=24, "15"->0x15=21），
/// 检测到该错误默认值时自动迁移为正确的十进制语义。
const LEGACY_MISREAD_VALUES: [u32; 2] = [24, 21];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub shortcut: String,
    #[serde(default = "default_input_shortcut")]
    pub input_shortcut: String,
    #[serde(default = "default_input_monitor")]
    pub input_monitor: String,
    #[serde(default = "default_input_values")]
    pub input_values: Vec<u32>,
}

fn default_input_shortcut() -> String {
    DEFAULT_INPUT_SHORTCUT.to_string()
}

fn default_input_monitor() -> String {
    DEFAULT_INPUT_MONITOR.to_string()
}

fn default_input_values() -> Vec<u32> {
    DEFAULT_INPUT_VALUES.to_vec()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            shortcut: DEFAULT_SHORTCUT.to_string(),
            input_shortcut: default_input_shortcut(),
            input_monitor: default_input_monitor(),
            input_values: default_input_values(),
        }
    }
}

fn config_path() -> Option<PathBuf> {
    std::env::var("APPDATA")
        .ok()
        .map(|appdata| PathBuf::from(appdata).join("ProjectionSwitcher.json"))
}

pub fn load() -> Config {
    let mut config = config_path()
        .and_then(|path| std::fs::read_to_string(path).ok())
        .and_then(|text| serde_json::from_str::<Config>(&text).ok())
        .unwrap_or_default();
    // 一次性迁移：早期版本误读进制导致的错误默认值。
    if config.input_values == LEGACY_MISREAD_VALUES {
        config.input_values = DEFAULT_INPUT_VALUES.to_vec();
        let _ = save(&config);
    }
    config
}

pub fn save(config: &Config) -> Result<(), String> {
    let Some(path) = config_path() else {
        return Err("无法定位 %APPDATA% 目录".to_string());
    };
    let text = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    std::fs::write(&path, text).map_err(|e| format!("写入配置文件失败：{e}"))
}
