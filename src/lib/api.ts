import { invoke } from "@tauri-apps/api/core";

export type ProjectionMode = "internal" | "clone" | "extend" | "external";

/** 与 Rust 端 DisplaySwitchToggle.state 对应的学习状态 */
export interface ToggleState {
  last: string;
  sigInt: string;
  sigExt: string;
}

export interface ProjectionStatus {
  currentSignature: string;
  state: ToggleState;
  /** internal | external | unknown */
  detected: string;
}

export interface SwitchOutcome {
  switchedTo: string;
  status: ProjectionStatus;
}

export const projectionApi = {
  getStatus: () => invoke<ProjectionStatus>("get_status"),
  toggleProjection: () => invoke<SwitchOutcome>("toggle_projection"),
  switchTo: (mode: ProjectionMode) => invoke<SwitchOutcome>("switch_to", { mode }),
  resetState: () => invoke<ProjectionStatus>("reset_state"),
};

/** 用户配置（%APPDATA%\ProjectionSwitcher.json） */
export interface AppConfig {
  shortcut: string;
  inputShortcut: string;
  inputMonitor: string;
  inputValues: number[];
}

/** 快捷键类别：一键切换投影 / 切换输入源 */
export type ShortcutKind = "toggle" | "input";

export const configApi = {
  getConfig: () => invoke<AppConfig>("get_config"),
  setShortcut: (kind: ShortcutKind, shortcut: string) =>
    invoke<AppConfig>("set_shortcut", { kind, shortcut }),
  setInputMonitor: (name: string) =>
    invoke<AppConfig>("set_input_monitor", { name }),
};

/** 物理显示器（DDC/CI）信息 */
export interface MonitorInfo {
  name: string;
  inputSource: number | null;
}

export interface InputSwitchOutcome {
  monitor: string;
  from: number | null;
  to: number;
}

export const monitorApi = {
  getMonitors: () => invoke<MonitorInfo[]>("get_monitors"),
  switchInput: () => invoke<InputSwitchOutcome>("switch_input"),
};

/** VCP 值的十六进制展示，如 0x18 */
export function hex(value: number): string {
  return `0x${value.toString(16).toUpperCase().padStart(2, "0")}`;
}

/** 开机自启动（状态源为系统注册表，不存本地配置） */
export const autostartApi = {
  getAutostart: () => invoke<boolean>("get_autostart"),
  setAutostart: (enabled: boolean) => invoke<boolean>("set_autostart", { enabled }),
};

export const MODE_LABEL: Record<string, string> = {
  internal: "仅电脑屏幕",
  clone: "复制",
  extend: "扩展",
  external: "仅第二屏幕",
};

export const DETECTED_LABEL: Record<string, string> = {
  internal: "仅电脑屏幕",
  external: "仅第二屏幕",
  unknown: "未知（复制 / 扩展，或尚未学习）",
};
