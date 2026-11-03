//! 显示器输入源切换：基于 Windows 物理显示器 API（dxva2.dll，DDC/CI 协议）。
//!
//! 通过 VCP code 0x60（Input Select）读取/设置外接显示器的输入源。
//! 切换语义与 ControlMyMonitor 的 /SwitchValue 一致：读取当前值，
//! 在配置的值列表中循环切换到下一个；当前值不在列表中（或读不到）
//! 时切换到列表第一个值。
//!
//! 显示器标识使用 EDID 型号 ID（如 "XMI27B1"，取自设备 ID
//! MONITOR\XMI27B1\{...}）——与 ControlMyMonitor 的 "Monitor Name" 同体系。
//! 注意 GetPhysicalMonitorsFromHMONITOR 返回的描述是 "Generic PnP Monitor"
//! 这类系统通用名，无法区分具体显示器，仅作取不到 EDID 名时的回退。

use windows_sys::Win32::Devices::Display::{
    DestroyPhysicalMonitor, GetNumberOfPhysicalMonitorsFromHMONITOR,
    GetPhysicalMonitorsFromHMONITOR, GetVCPFeatureAndVCPFeatureReply, SetVCPFeature,
    MC_VCP_CODE_TYPE, PHYSICAL_MONITOR,
};
use windows_sys::Win32::Foundation::{GetLastError, HANDLE, RECT};
use windows_sys::Win32::Graphics::Gdi::{
    EnumDisplayDevicesW, EnumDisplayMonitors, GetMonitorInfoW, DISPLAY_DEVICEW,
    DISPLAY_DEVICE_ACTIVE, HDC, HMONITOR, MONITORINFO, MONITORINFOEXW,
};

/// VCP 0x60：Input Select（输入源选择）。
const VCP_INPUT_SOURCE: u8 = 0x60;

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorInfo {
    /// EDID 型号 ID（如 "XMI27B1"），与 ControlMyMonitor 的 Monitor Name 一致。
    pub name: String,
    /// 当前输入源值；显示器不支持 VCP 0x60（如笔记本内置屏）时为 None。
    pub input_source: Option<u32>,
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InputSwitchOutcome {
    pub monitor: String,
    pub from: Option<u32>,
    pub to: u32,
}

/// 一台物理显示器：EDID 名 + DDC/CI 句柄。
struct PhysicalMonitor {
    name: String,
    handle: HANDLE,
}

unsafe extern "system" fn enum_monitor_proc(
    hmonitor: HMONITOR,
    _hdc: HDC,
    _rect: *mut RECT,
    data: isize, // LPARAM（isize 别名）：回传的 Vec<HMONITOR> 指针
) -> i32 {
    let monitors = unsafe { &mut *(data as *mut Vec<HMONITOR>) };
    monitors.push(hmonitor);
    1
}

/// 取 HMONITOR 对应显示器的 EDID 型号 ID（设备 ID MONITOR\XMI27B1\... 的第二段）。
/// 链路：GetMonitorInfoW 得到 GDI 设备名（\\.\DISPLAY1），再经 EnumDisplayDevicesW
/// 枚举其活动监视器的 DeviceID。
fn edid_monitor_name(hmonitor: HMONITOR) -> Option<String> {
    unsafe {
        let mut info: MONITORINFOEXW = std::mem::zeroed();
        info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
        if GetMonitorInfoW(
            hmonitor,
            &mut info as *mut MONITORINFOEXW as *mut MONITORINFO,
        ) == 0
        {
            return None;
        }
        let mut index: u32 = 0;
        loop {
            let mut device: DISPLAY_DEVICEW = std::mem::zeroed();
            device.cb = std::mem::size_of::<DISPLAY_DEVICEW>() as u32;
            if EnumDisplayDevicesW(info.szDevice.as_ptr(), index, &mut device, 0) == 0 {
                return None; // 枚举结束仍未找到活动监视器
            }
            index += 1;
            if device.StateFlags & DISPLAY_DEVICE_ACTIVE == 0 {
                continue;
            }
            let device_id = wide_to_string(&device.DeviceID);
            let edid = device_id.split('\\').nth(1).unwrap_or_default();
            if !edid.is_empty() {
                return Some(edid.to_string());
            }
        }
    }
}

/// 枚举所有物理显示器（用完必须 DestroyPhysicalMonitor，由调用方负责）。
fn physical_monitors() -> Vec<PhysicalMonitor> {
    let mut hmonitors: Vec<HMONITOR> = Vec::new();
    unsafe {
        EnumDisplayMonitors(
            std::ptr::null_mut(),
            std::ptr::null(),
            Some(enum_monitor_proc),
            &mut hmonitors as *mut Vec<HMONITOR> as isize,
        );
    }
    let mut physical = Vec::new();
    for hmonitor in hmonitors {
        unsafe {
            let mut count: u32 = 0;
            if GetNumberOfPhysicalMonitorsFromHMONITOR(hmonitor, &mut count) == 0 || count == 0 {
                continue;
            }
            let mut array: Vec<PHYSICAL_MONITOR> = Vec::with_capacity(count as usize);
            for _ in 0..count {
                array.push(std::mem::zeroed());
            }
            if GetPhysicalMonitorsFromHMONITOR(hmonitor, count, array.as_mut_ptr()) == 0 {
                continue;
            }
            let edid_name = edid_monitor_name(hmonitor);
            for pm in array {
                // 取不到 EDID 名时回退到物理监视器描述（如 "Generic PnP Monitor"）。
                let name = match &edid_name {
                    Some(edid) => edid.clone(),
                    None => monitor_description(&pm),
                };
                physical.push(PhysicalMonitor {
                    name,
                    handle: pm.hPhysicalMonitor,
                });
            }
        }
    }
    physical
}

fn read_input_source(handle: HANDLE) -> Option<u32> {
    // DDC/CI 为慢速 I2C 总线，首次通信偶发超时，重试提高成功率；
    // 显示器正显示其他输入源时对当前口不响应 DDC，最终仍失败则返回 None。
    for _ in 0..3 {
        unsafe {
            let mut code_type: MC_VCP_CODE_TYPE = std::mem::zeroed();
            let mut current: u32 = 0;
            let mut maximum: u32 = 0;
            if GetVCPFeatureAndVCPFeatureReply(
                handle,
                VCP_INPUT_SOURCE,
                &mut code_type,
                &mut current,
                &mut maximum,
            ) != 0
            {
                return Some(current);
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(250));
    }
    None
}

/// 列出所有物理显示器及其当前输入源。
pub fn list_monitors() -> Vec<MonitorInfo> {
    let monitors = physical_monitors();
    let mut result = Vec::with_capacity(monitors.len());
    for m in &monitors {
        result.push(MonitorInfo {
            name: m.name.clone(),
            input_source: read_input_source(m.handle),
        });
    }
    destroy_all(&monitors);
    result
}

/// 在指定显示器上循环切换输入源（/SwitchValue 语义）。
pub fn switch_input(monitor: &str, values: &[u32]) -> Result<InputSwitchOutcome, String> {
    if values.is_empty() {
        return Err("切换值列表为空，请检查配置文件".to_string());
    }
    let monitors = physical_monitors();
    let Some(target) = monitors.iter().find(|m| m.name == monitor) else {
        let names: Vec<String> = monitors.iter().map(|m| m.name.clone()).collect();
        destroy_all(&monitors);
        let available = if names.is_empty() {
            "（未枚举到任何物理显示器）".to_string()
        } else {
            names.join("、")
        };
        return Err(format!("未找到显示器 \"{monitor}\"，可用：{available}"));
    };

    let from = read_input_source(target.handle);
    let to = next_switch_value(values, from);
    let ok = unsafe { SetVCPFeature(target.handle, VCP_INPUT_SOURCE, to) } != 0;
    let last_err = unsafe { GetLastError() };
    destroy_all(&monitors);
    if !ok {
        return Err(format!(
            "设置输入源失败（os error {last_err:#x}，显示器可能不支持 DDC/CI 或已禁用 OSD 控制）"
        ));
    }
    Ok(InputSwitchOutcome {
        monitor: monitor.to_string(),
        from,
        to,
    })
}

/// 循环切换的纯逻辑：当前值在列表中则取下一个（末尾回绕），否则取第一个。
pub fn next_switch_value(values: &[u32], current: Option<u32>) -> u32 {
    match current {
        Some(cur) => match values.iter().position(|v| *v == cur) {
            Some(i) => values[(i + 1) % values.len()],
            None => values[0],
        },
        None => values[0],
    }
}

fn destroy_all(monitors: &[PhysicalMonitor]) {
    for m in monitors {
        unsafe { DestroyPhysicalMonitor(m.handle) };
    }
}

fn wide_to_string(buf: &[u16]) -> String {
    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..len])
}

/// 取物理监视器的描述名。PHYSICAL_MONITOR 为 packed 结构，字段须先按值拷贝。
fn monitor_description(monitor: &PHYSICAL_MONITOR) -> String {
    let description = monitor.szPhysicalMonitorDescription;
    wide_to_string(&description)
}

#[cfg(test)]
mod tests {
    use super::next_switch_value;

    #[test]
    fn cycles_through_values() {
        let values = [0x18u32, 0x15];
        assert_eq!(next_switch_value(&values, Some(0x18)), 0x15);
        assert_eq!(next_switch_value(&values, Some(0x15)), 0x18);
    }

    #[test]
    fn supports_more_than_two_values() {
        let values = [1u32, 2, 3];
        assert_eq!(next_switch_value(&values, Some(1)), 2);
        assert_eq!(next_switch_value(&values, Some(2)), 3);
        assert_eq!(next_switch_value(&values, Some(3)), 1); // 末尾回绕
    }

    #[test]
    fn unknown_or_unreadable_falls_to_first() {
        let values = [0x18u32, 0x15];
        assert_eq!(next_switch_value(&values, Some(0x12)), 0x18);
        assert_eq!(next_switch_value(&values, None), 0x18);
    }
}
