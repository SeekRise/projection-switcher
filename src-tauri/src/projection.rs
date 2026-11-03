//! 投影模式切换核心逻辑。
//!
//! 工作原理（分辨率签名 + 状态记忆）：
//! 1. 通过 Win32 GDI 枚举第一个活动显示输出，取其当前分辨率作为"模式签名"；
//! 2. 与已学习的内置/外接签名比对，判断当前处于"仅电脑屏幕"还是"仅第二屏幕"；
//! 3. 签名无法判定时（首次运行 / 复制 / 扩展 / 改过分辨率），按上次切换记录取反；
//! 4. 每次切换到 internal/external 后轮询签名变化并重新学习。
//!
//! 投影切换直接调用 Win32 Display Configuration API 的 SetDisplayConfig——
//! 与系统 Win+P 面板相同的底层调用，无外部进程。
//!
//! 状态文件：%APPDATA%\DisplaySwitchToggle.state

use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use serde::Serialize;
use windows_sys::Win32::Devices::Display::{
    SetDisplayConfig, SDC_APPLY, SDC_TOPOLOGY_CLONE, SDC_TOPOLOGY_EXTEND, SDC_TOPOLOGY_EXTERNAL,
    SDC_TOPOLOGY_INTERNAL,
};
use windows_sys::Win32::Foundation::ERROR_SUCCESS;
use windows_sys::Win32::Graphics::Gdi::{
    EnumDisplayDevicesW, EnumDisplaySettingsW, DEVMODEW, DISPLAY_DEVICEW, DISPLAY_DEVICE_ACTIVE,
    ENUM_CURRENT_SETTINGS,
};

/// 首次运行（从未切换过）时默认切到哪边。
const DEFAULT_FIRST: &str = "external";
/// 切换后等待拓扑生效的上限毫秒数（签名一旦变化即提前返回）。
const SETTLE_TIMEOUT_MS: u64 = 3000;
/// 等待拓扑生效的轮询间隔毫秒数。
const SETTLE_POLL_MS: u64 = 200;

const VALID_MODES: [&str; 4] = ["internal", "clone", "extend", "external"];

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToggleState {
    pub last: String,
    pub sig_int: String,
    pub sig_ext: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectionStatus {
    pub current_signature: String,
    pub state: ToggleState,
    /// "internal" | "external" | "unknown"
    pub detected: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SwitchOutcome {
    pub switched_to: String,
    pub status: ProjectionStatus,
}

#[derive(Debug)]
pub enum SwitchError {
    InvalidMode(String),
    Exec(String),
    Io(String),
}

impl std::fmt::Display for SwitchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SwitchError::InvalidMode(m) => write!(f, "无效的投影模式：{m}"),
            SwitchError::Exec(e) => write!(f, "{e}"),
            SwitchError::Io(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for SwitchError {}

/// 读取当前状态（含当前签名与模式判定）。
pub fn status() -> ProjectionStatus {
    let state = read_state();
    let cur = current_signature().unwrap_or_default();
    ProjectionStatus {
        current_signature: cur.clone(),
        detected: detect(&state, &cur).to_string(),
        state,
    }
}

/// 一键切换：在"仅电脑屏幕 ⇄ 仅第二屏幕"之间智能取反。
pub async fn toggle() -> Result<SwitchOutcome, SwitchError> {
    let state = read_state();
    let cur = current_signature().unwrap_or_default();
    let target = decide_target(&state, &cur);
    switch_to(target).await
}

/// 显式切换到指定模式，等待生效后学习签名并保存状态。
pub async fn switch_to(mode: &str) -> Result<SwitchOutcome, SwitchError> {
    let mode: &str = VALID_MODES
        .iter()
        .find(|m| **m == mode)
        .ok_or_else(|| SwitchError::InvalidMode(mode.to_string()))?;

    let previous = current_signature().unwrap_or_default();
    apply_topology(mode)?;
    let cur = wait_for_settle(&previous).await;

    let mut state = read_state();
    state.last = mode.to_string();
    // 仅 internal/external 的签名是可靠的单屏输出，用于学习；复制/扩展不学习。
    if (mode == "internal" || mode == "external") && !cur.is_empty() {
        if mode == "internal" {
            state.sig_int = cur.clone();
        } else {
            state.sig_ext = cur.clone();
        }
    }
    write_state(&state)?;

    Ok(SwitchOutcome {
        switched_to: mode.to_string(),
        status: ProjectionStatus {
            detected: detect(&state, &cur).to_string(),
            current_signature: cur,
            state,
        },
    })
}

/// 删除状态文件，清空已学习的签名（下次切换重新学习）。
pub fn reset_state() -> Result<ProjectionStatus, SwitchError> {
    if let Some(path) = state_path() {
        if path.exists() {
            fs::remove_file(&path)
                .map_err(|e| SwitchError::Io(format!("删除状态文件失败：{e}")))?;
        }
    }
    Ok(status())
}

/// 判定当前模式：签名匹配则精确判定，否则 unknown。
fn detect(state: &ToggleState, cur: &str) -> &'static str {
    if !state.sig_int.is_empty() && state.sig_int != state.sig_ext {
        if cur == state.sig_int {
            return "internal";
        }
        if cur == state.sig_ext && !state.sig_ext.is_empty() {
            return "external";
        }
    }
    "unknown"
}

/// 决定本次切换到哪边：签名可判定则精确取反，否则按上次记录取反。
fn decide_target(state: &ToggleState, cur: &str) -> &'static str {
    if !state.sig_int.is_empty() && state.sig_int != state.sig_ext {
        if cur == state.sig_int {
            return "external"; // 签名确认当前=仅电脑屏幕 -> 切到仅第二屏幕
        }
        if !state.sig_ext.is_empty() && cur == state.sig_ext {
            return "internal"; // 签名确认当前=仅第二屏幕 -> 切到仅电脑屏幕
        }
    }
    match state.last.as_str() {
        "internal" => "external", // 按上次记录取反
        "external" => "internal",
        _ => DEFAULT_FIRST, // 首次运行 -> 默认切仅第二屏幕
    }
}

/// 模式名 -> SDC 拓扑标志（纯函数，便于测试）。
fn topology_flags(mode: &str) -> Option<u32> {
    match mode {
        "internal" => Some(SDC_TOPOLOGY_INTERNAL),
        "clone" => Some(SDC_TOPOLOGY_CLONE),
        "extend" => Some(SDC_TOPOLOGY_EXTEND),
        "external" => Some(SDC_TOPOLOGY_EXTERNAL),
        _ => None,
    }
}

/// 应用投影拓扑：直接调用 SetDisplayConfig——与系统 Win+P 完全相同的
/// 底层调用（空路径数组 + 拓扑标志 + SDC_APPLY，由系统从显示配置
/// 数据库挑选该类拓扑的持久化配置）。
/// 注意不能附加 SDC_SAVE_TO_DATABASE：该标志要求调用方提供可保存的配置，
/// 与空路径数组组合会得到 ERROR_INVALID_PARAMETER(87)。
fn apply_topology(mode: &str) -> Result<(), SwitchError> {
    let Some(topology) = topology_flags(mode) else {
        return Err(SwitchError::InvalidMode(mode.to_string()));
    };
    let flags = topology | SDC_APPLY;
    // API 同步快速返回，应用在后台完成，随后由 wait_for_settle 等待生效。
    let code = unsafe { SetDisplayConfig(0, std::ptr::null(), 0, std::ptr::null(), flags) };
    if code != ERROR_SUCCESS as i32 {
        return Err(SwitchError::Exec(format!(
            "SetDisplayConfig 失败（Win32 错误码 {code:#010x}）"
        )));
    }
    Ok(())
}

/// 轮询等待新拓扑生效：当前签名相对切换前发生变化即认为生效并返回新签名；
/// 超过上限仍未变化（如目标模式与当前一致）则返回当前值兜底。
async fn wait_for_settle(previous: &str) -> String {
    let deadline = Instant::now() + Duration::from_millis(SETTLE_TIMEOUT_MS);
    loop {
        let cur = current_signature().unwrap_or_default();
        if !cur.is_empty() && cur != previous {
            return cur;
        }
        if Instant::now() >= deadline {
            return cur;
        }
        tokio::time::sleep(Duration::from_millis(SETTLE_POLL_MS)).await;
    }
}

fn state_path() -> Option<PathBuf> {
    std::env::var("APPDATA")
        .ok()
        .map(|appdata| PathBuf::from(appdata).join("DisplaySwitchToggle.state"))
}

fn read_state() -> ToggleState {
    let Some(path) = state_path() else {
        return ToggleState::default();
    };
    match fs::read_to_string(&path) {
        Ok(text) => parse_state(&text),
        Err(_) => ToggleState::default(),
    }
}

fn write_state(state: &ToggleState) -> Result<(), SwitchError> {
    let Some(path) = state_path() else {
        return Err(SwitchError::Io("无法定位 %APPDATA% 目录".to_string()));
    };
    fs::write(&path, render_state(state))
        .map_err(|e| SwitchError::Io(format!("写入状态文件失败：{e}")))
}

/// 状态文件格式：last= / sigInt= / sigExt= 三行文本。
fn parse_state(text: &str) -> ToggleState {
    let mut state = ToggleState::default();
    for line in text.lines() {
        let line = line.trim();
        if let Some(v) = line.strip_prefix("last=") {
            state.last = v.to_string();
        } else if let Some(v) = line.strip_prefix("sigInt=") {
            state.sig_int = v.to_string();
        } else if let Some(v) = line.strip_prefix("sigExt=") {
            state.sig_ext = v.to_string();
        }
    }
    state
}

fn render_state(state: &ToggleState) -> String {
    format!(
        "last={}\nsigInt={}\nsigExt={}\n",
        state.last, state.sig_int, state.sig_ext
    )
}

/// 取第一个活动显示输出的当前分辨率作为模式签名，如 "2560x1600"
/// （跳过无活动输出的设备：虚拟显卡 / 已关闭的内置屏）。
pub fn current_signature() -> Option<String> {
    unsafe {
        let mut index: u32 = 0;
        loop {
            let mut device: DISPLAY_DEVICEW = std::mem::zeroed();
            device.cb = std::mem::size_of::<DISPLAY_DEVICEW>() as u32;
            if EnumDisplayDevicesW(std::ptr::null(), index, &mut device, 0) == 0 {
                break; // 枚举结束
            }
            index += 1;
            if device.StateFlags & DISPLAY_DEVICE_ACTIVE == 0 {
                continue;
            }
            let mut mode: DEVMODEW = std::mem::zeroed();
            mode.dmSize = std::mem::size_of::<DEVMODEW>() as u16;
            if EnumDisplaySettingsW(device.DeviceName.as_ptr(), ENUM_CURRENT_SETTINGS, &mut mode)
                != 0
                && mode.dmPelsWidth > 0
                && mode.dmPelsHeight > 0
            {
                return Some(format!("{}x{}", mode.dmPelsWidth, mode.dmPelsHeight));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(last: &str, sig_int: &str, sig_ext: &str) -> ToggleState {
        ToggleState {
            last: last.to_string(),
            sig_int: sig_int.to_string(),
            sig_ext: sig_ext.to_string(),
        }
    }

    #[test]
    fn parse_state_reads_state_file_format() {
        let s = parse_state("last=external\r\nsigInt=2560x1600\r\nsigExt=3840x2160\r\n");
        assert_eq!(s.last, "external");
        assert_eq!(s.sig_int, "2560x1600");
        assert_eq!(s.sig_ext, "3840x2160");
    }

    #[test]
    fn parse_state_handles_missing_lines_and_garbage() {
        let s = parse_state("hello\nsigExt=1920x1080\n");
        assert_eq!(s.last, "");
        assert_eq!(s.sig_int, "");
        assert_eq!(s.sig_ext, "1920x1080");
    }

    #[test]
    fn render_parse_round_trip() {
        let s = state("internal", "2560x1600", "3840x2160");
        assert_eq!(parse_state(&render_state(&s)), s);
    }

    #[test]
    fn decide_target_signature_confirmed() {
        // 当前签名=内置签名 -> 切到外接
        let s = state("internal", "2560x1600", "3840x2160");
        assert_eq!(decide_target(&s, "2560x1600"), "external");
        // 当前签名=外接签名 -> 切到内置
        assert_eq!(decide_target(&s, "3840x2160"), "internal");
    }

    #[test]
    fn decide_target_falls_back_to_last() {
        // 签名不匹配任何已学习签名 -> 按上次记录取反
        assert_eq!(
            decide_target(&state("internal", "2560x1600", "3840x2160"), "1920x1080"),
            "external"
        );
        assert_eq!(
            decide_target(&state("external", "2560x1600", "3840x2160"), "1920x1080"),
            "internal"
        );
    }

    #[test]
    fn decide_target_identical_signatures_are_unreliable() {
        // 两签名相同（不可区分）-> 忽略签名，按上次记录取反
        let s = state("internal", "2560x1600", "2560x1600");
        assert_eq!(decide_target(&s, "2560x1600"), "external");
    }

    #[test]
    fn decide_target_first_run_defaults_to_external() {
        assert_eq!(
            decide_target(&ToggleState::default(), "2560x1600"),
            "external"
        );
    }

    #[test]
    fn detect_reports_known_and_unknown() {
        let s = state("internal", "2560x1600", "3840x2160");
        assert_eq!(detect(&s, "2560x1600"), "internal");
        assert_eq!(detect(&s, "3840x2160"), "external");
        assert_eq!(detect(&s, "1920x1080"), "unknown");
        assert_eq!(detect(&state("", "", ""), "2560x1600"), "unknown");
    }

    #[test]
    fn topology_flags_map_all_four_modes() {
        assert_eq!(topology_flags("internal"), Some(SDC_TOPOLOGY_INTERNAL));
        assert_eq!(topology_flags("clone"), Some(SDC_TOPOLOGY_CLONE));
        assert_eq!(topology_flags("extend"), Some(SDC_TOPOLOGY_EXTEND));
        assert_eq!(topology_flags("external"), Some(SDC_TOPOLOGY_EXTERNAL));
        assert_eq!(topology_flags("Internal"), None); // 大小写敏感，白名单外拒绝
        assert_eq!(topology_flags("none"), None);
    }
}
