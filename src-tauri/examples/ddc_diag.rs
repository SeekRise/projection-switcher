//! DDC/CI 诊断（只读）：逐层打印物理监视器 API 的返回值与 GetLastError，
//! 用于定位"读取不到输入源"的具体环节。不做任何 SetVCPFeature 写操作。

use windows_sys::Win32::Devices::Display::{
    DestroyPhysicalMonitor, GetNumberOfPhysicalMonitorsFromHMONITOR,
    GetPhysicalMonitorsFromHMONITOR, GetVCPFeatureAndVCPFeatureReply, MC_VCP_CODE_TYPE,
    PHYSICAL_MONITOR,
};
use windows_sys::Win32::Foundation::{GetLastError, HANDLE, RECT};
use windows_sys::Win32::Graphics::Gdi::{EnumDisplayMonitors, HDC, HMONITOR};

unsafe extern "system" fn enum_proc(
    hmonitor: HMONITOR,
    _hdc: HDC,
    _rect: *mut RECT,
    data: isize,
) -> i32 {
    let monitors = unsafe { &mut *(data as *mut Vec<HMONITOR>) };
    monitors.push(hmonitor);
    1
}

fn read_vcp(handle: HANDLE, code: u8, label: &str) {
    unsafe {
        for attempt in 1..=3 {
            let mut vcp_type: MC_VCP_CODE_TYPE = std::mem::zeroed();
            let mut current: u32 = 0;
            let mut maximum: u32 = 0;
            let ok = GetVCPFeatureAndVCPFeatureReply(
                handle,
                code,
                &mut vcp_type,
                &mut current,
                &mut maximum,
            );
            println!(
                "      read {label} (attempt {attempt}): ok={ok} cur={current:#04x} max={maximum:#04x} last_err={:#x}",
                GetLastError()
            );
            if ok != 0 {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(300));
        }
    }
}

fn main() {
    // 库层视角：EDID 名 + 输入源（与 GUI 展示一致）
    for m in projection_switcher_lib::monitor_input::list_monitors() {
        println!(
            "[lib] monitor=\"{}\" input_source={}",
            m.name,
            match m.input_source {
                Some(v) => format!("{v:#04x}"),
                None => "不支持".to_string(),
            }
        );
    }

    unsafe {
        let mut hmonitors: Vec<HMONITOR> = Vec::new();
        let enum_ok = EnumDisplayMonitors(
            std::ptr::null_mut(),
            std::ptr::null(),
            Some(enum_proc),
            &mut hmonitors as *mut Vec<HMONITOR> as isize,
        );
        println!(
            "EnumDisplayMonitors: ok={enum_ok} count={} last_err={:#x}",
            hmonitors.len(),
            GetLastError()
        );

        for (i, hmonitor) in hmonitors.iter().enumerate() {
            println!("[monitor {i}]");
            let mut count: u32 = 0;
            let okn = GetNumberOfPhysicalMonitorsFromHMONITOR(*hmonitor, &mut count);
            println!(
                "  GetNumberOfPhysicalMonitors: ok={okn} count={count} last_err={:#x}",
                GetLastError()
            );
            if okn == 0 || count == 0 {
                continue;
            }
            let mut array: Vec<PHYSICAL_MONITOR> = Vec::with_capacity(count as usize);
            for _ in 0..count {
                array.push(std::mem::zeroed());
            }
            let okp = GetPhysicalMonitorsFromHMONITOR(*hmonitor, count, array.as_mut_ptr());
            println!(
                "  GetPhysicalMonitors: ok={okp} last_err={:#x}",
                GetLastError()
            );
            if okp == 0 {
                continue;
            }
            for pm in &array {
                let description = pm.szPhysicalMonitorDescription;
                let len = description
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(description.len());
                let name = String::from_utf16_lossy(&description[..len]);
                println!("  physical monitor name = \"{name}\"");
                read_vcp(pm.hPhysicalMonitor, 0x60, "VCP 0x60 (input source)");
                DestroyPhysicalMonitor(pm.hPhysicalMonitor);
            }
        }
        println!("done");
    }
}
