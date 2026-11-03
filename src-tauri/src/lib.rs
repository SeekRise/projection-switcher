mod config;
pub mod monitor_input; // pub 供 examples/ddc_diag 诊断工具复用
mod projection;

use std::sync::atomic::{AtomicBool, Ordering};

use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Emitter, Manager};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutEvent, ShortcutState};

use projection::{ProjectionStatus, SwitchOutcome};

/// 一键切换进行中标记：防止快捷键/托盘连按导致并发切换（切换判定依赖生效后的签名）。
static SWITCHING: AtomicBool = AtomicBool::new(false);

#[tauri::command]
fn get_status() -> ProjectionStatus {
    projection::status()
}

#[tauri::command]
async fn toggle_projection() -> Result<SwitchOutcome, String> {
    do_toggle().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn switch_to(mode: String) -> Result<SwitchOutcome, String> {
    projection::switch_to(&mode)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn reset_state() -> Result<ProjectionStatus, String> {
    projection::reset_state().map_err(|e| e.to_string())
}

#[tauri::command]
fn get_config() -> config::Config {
    config::load()
}

/// 快捷键类别：一键切换投影 / 切换输入源。
fn shortcut_kind(kind: &str) -> Result<&'static str, String> {
    match kind {
        "toggle" => Ok("toggle"),
        "input" => Ok("input"),
        other => Err(format!("未知的快捷键类别：{other}")),
    }
}

/// 修改全局快捷键：校验并换绑，注册失败时回滚原快捷键并报错。
#[tauri::command]
fn set_shortcut(
    app: tauri::AppHandle,
    kind: String,
    shortcut: String,
) -> Result<config::Config, String> {
    let action = shortcut_kind(&kind)?;
    let new_shortcut: Shortcut = shortcut
        .parse()
        .map_err(|e| format!("无效的快捷键格式：{e}"))?;
    let gs = app.global_shortcut();
    let mut cfg = config::load();
    let old = match action {
        "toggle" => cfg.shortcut.clone(),
        _ => cfg.input_shortcut.clone(),
    };
    if old == shortcut {
        return Ok(cfg); // 未变化
    }

    if let Ok(old_shortcut) = old.parse::<Shortcut>() {
        let _ = gs.unregister(old_shortcut);
    }
    if let Err(e) = gs.on_shortcut(new_shortcut, on_trigger(action)) {
        // 回滚：尽力恢复原快捷键，保持行为一致。
        if let Ok(old_shortcut) = old.parse::<Shortcut>() {
            let _ = gs.on_shortcut(old_shortcut, on_trigger(action));
        }
        return Err(format!("注册快捷键失败（可能已被其他程序占用）：{e}"));
    }
    match action {
        "toggle" => cfg.shortcut = shortcut,
        _ => cfg.input_shortcut = shortcut,
    }
    config::save(&cfg)?;
    Ok(cfg)
}

/// 修改输入源切换的目标显示器。
#[tauri::command]
fn set_input_monitor(name: String) -> Result<config::Config, String> {
    let mut cfg = config::load();
    cfg.input_monitor = name;
    config::save(&cfg)?;
    Ok(cfg)
}

/// 查询开机自启动是否已启用（状态源为系统，不存本地配置）。
#[tauri::command]
fn get_autostart(app: tauri::AppHandle) -> Result<bool, String> {
    app.autolaunch().is_enabled().map_err(|e| e.to_string())
}

/// 开启/关闭开机自启动（以 --hidden 参数静默拉起，仅托盘运行）。
#[tauri::command]
fn set_autostart(app: tauri::AppHandle, enabled: bool) -> Result<bool, String> {
    let autolaunch = app.autolaunch();
    if enabled {
        autolaunch.enable().map_err(|e| e.to_string())?;
    } else {
        autolaunch.disable().map_err(|e| e.to_string())?;
    }
    autolaunch.is_enabled().map_err(|e| e.to_string())
}

#[tauri::command]
fn get_monitors() -> Vec<monitor_input::MonitorInfo> {
    monitor_input::list_monitors()
}

#[tauri::command]
fn switch_input() -> Result<monitor_input::InputSwitchOutcome, String> {
    let cfg = config::load();
    monitor_input::switch_input(&cfg.input_monitor, &cfg.input_values)
}

/// 全局快捷键按下时的动作分发。
fn on_trigger(action: &'static str) -> impl Fn(&tauri::AppHandle, &Shortcut, ShortcutEvent) {
    move |app, _shortcut, event| {
        if event.state == ShortcutState::Pressed {
            match action {
                "toggle" => spawn_toggle(app.clone()),
                "input" => spawn_input_switch(app.clone()),
                _ => {}
            }
        }
    }
}

/// 异步执行一键切换（投影模式），结果通过事件推送给前端；切换中忽略重复触发。
fn spawn_toggle(app: tauri::AppHandle) {
    if SWITCHING.swap(true, Ordering::SeqCst) {
        return; // 上一次切换尚未完成
    }
    tauri::async_runtime::spawn(async move {
        let result = do_toggle().await;
        SWITCHING.store(false, Ordering::SeqCst);
        match result {
            Ok(outcome) => {
                let _ = app.emit("projection-switched", &outcome);
            }
            Err(e) => {
                let _ = app.emit("projection-switch-error", e.to_string());
            }
        }
    });
}

/// 异步执行输入源切换（快捷键/托盘入口），结果通过事件推送给前端。
fn spawn_input_switch(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        let cfg = config::load();
        let result = monitor_input::switch_input(&cfg.input_monitor, &cfg.input_values);
        match result {
            Ok(outcome) => {
                let _ = app.emit("input-switched", &outcome);
            }
            Err(e) => {
                let _ = app.emit("input-switch-error", e);
            }
        }
    });
}

async fn do_toggle() -> Result<SwitchOutcome, projection::SwitchError> {
    projection::toggle().await
}

/// 显示并聚焦主窗口。
fn show_main_window(app: &tauri::AppHandle) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window("main") {
        window.show()?;
        window.set_focus()?;
        window.unminimize()?;
    }
    Ok(())
}

fn build_tray(app: &tauri::AppHandle) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "打开主窗口", true, None::<&str>)?;
    let toggle = MenuItem::with_id(app, "toggle", "一键切换投影", true, None::<&str>)?;
    let input = MenuItem::with_id(app, "input", "切换输入源", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &toggle, &input, &quit])?;

    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or(tauri::Error::AssetNotFound("app icon".into()))?;

    TrayIconBuilder::with_id("main-tray")
        .icon(icon)
        .tooltip("投影切换：点击打开，右键菜单退出")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => {
                let _ = show_main_window(app);
            }
            "toggle" => spawn_toggle(app.clone()),
            "input" => spawn_input_switch(app.clone()),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let _ = show_main_window(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

/// 启动时注册一个全局快捷键；失败（格式无效/被占用）仅记录，不阻止应用启动。
fn register_shortcut(app: &tauri::AppHandle, shortcut: &str, action: &'static str) {
    match shortcut.parse::<Shortcut>() {
        Ok(parsed) => {
            if let Err(e) = app
                .global_shortcut()
                .on_shortcut(parsed, on_trigger(action))
            {
                eprintln!("全局快捷键 {shortcut} 注册失败（可能被占用）：{e}");
            }
        }
        Err(e) => {
            eprintln!("配置中的快捷键 {shortcut} 无效：{e:?}");
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let hidden = std::env::args().any(|arg| arg == "--hidden");

    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--hidden"]),
        ))
        .setup(move |app| {
            build_tray(app.handle())?;

            let cfg = config::load();
            register_shortcut(app.handle(), &cfg.shortcut, "toggle");
            register_shortcut(app.handle(), &cfg.input_shortcut, "input");

            // 窗口默认隐藏创建：普通启动显示，--hidden（开机自启）保持托盘模式。
            if !hidden {
                if let Some(window) = app.get_webview_window("main") {
                    window.show()?;
                }
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            // 关闭主窗口 = 最小化到托盘，程序继续后台运行（托盘菜单可退出）。
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_status,
            toggle_projection,
            switch_to,
            reset_state,
            get_config,
            set_shortcut,
            set_input_monitor,
            get_autostart,
            set_autostart,
            get_monitors,
            switch_input
        ])
        .run(tauri::generate_context!())
        .expect("failed to run tauri application");
}
