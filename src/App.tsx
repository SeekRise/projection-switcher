import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { Sidebar, type ModuleId } from "./components/Sidebar";
import { Toast, type ToastState } from "./components/Toast";
import { ProjectionPanel } from "./features/ProjectionPanel";
import { InputSourcePanel } from "./features/InputSourcePanel";
import { SettingsPanel } from "./features/SettingsPanel";
import {
  MODE_LABEL,
  autostartApi,
  configApi,
  hex,
  monitorApi,
  projectionApi,
  type AppConfig,
  type InputSwitchOutcome,
  type MonitorInfo,
  type ProjectionMode,
  type ProjectionStatus,
  type ShortcutKind,
  type SwitchOutcome,
} from "./lib/api";

/** 从 KeyboardEvent.code 提取快捷键主键名（Tauri Shortcut 格式），不支持的键返回 null。 */
function mainKeyFromCode(code: string): string | null {
  if (/^Key[A-Z]$/.test(code)) return code.slice(3);
  if (/^Digit\d$/.test(code)) return code.slice(5);
  if (/^F([1-9]|1[0-2])$/.test(code)) return code;
  const named: Record<string, string> = {
    ArrowUp: "Up",
    ArrowDown: "Down",
    ArrowLeft: "Left",
    ArrowRight: "Right",
    Space: "Space",
    Minus: "Minus",
    Equal: "Equal",
    BracketLeft: "BracketLeft",
    BracketRight: "BracketRight",
    Semicolon: "Semicolon",
    Quote: "Quote",
    Comma: "Comma",
    Period: "Period",
    Slash: "Slash",
  };
  return named[code] ?? null;
}

export default function App() {
  const [active, setActive] = useState<ModuleId>("projection");
  const [status, setStatus] = useState<ProjectionStatus | null>(null);
  const [config, setConfig] = useState<AppConfig | null>(null);
  const [autostart, setAutostart] = useState<boolean | null>(null);
  const [monitors, setMonitors] = useState<MonitorInfo[] | null>(null);
  const [switchingInput, setSwitchingInput] = useState(false);
  const [pending, setPending] = useState<"toggle" | ProjectionMode | null>(null);
  const [recording, setRecording] = useState<ShortcutKind | null>(null);
  const [toast, setToast] = useState<ToastState | null>(null);
  const [dark, setDark] = useState(
    document.documentElement.dataset.theme === "dark",
  );
  const toastTimer = useRef<number | undefined>(undefined);

  const showToast = useCallback((kind: "success" | "error", message: string) => {
    window.clearTimeout(toastTimer.current);
    setToast({ id: Date.now(), kind, message });
    toastTimer.current = window.setTimeout(() => setToast(null), 3000);
  }, []);

  const refreshMonitors = useCallback(() => {
    monitorApi
      .getMonitors()
      .then(setMonitors)
      .catch((e: unknown) => showToast("error", `枚举显示器失败：${String(e)}`));
  }, [showToast]);

  useEffect(() => {
    projectionApi
      .getStatus()
      .then(setStatus)
      .catch((e: unknown) => showToast("error", `读取状态失败：${String(e)}`));
    configApi
      .getConfig()
      .then(setConfig)
      .catch((e: unknown) => showToast("error", `读取配置失败：${String(e)}`));
    autostartApi
      .getAutostart()
      .then(setAutostart)
      .catch(() => setAutostart(false));
    refreshMonitors();
  }, [showToast, refreshMonitors]);

  // 窗口重新获得焦点/从托盘恢复时刷新显示器列表（如从另一台电脑切回输入源后）。
  useEffect(() => {
    const onFocus = () => refreshMonitors();
    const onVisible = () => {
      if (document.visibilityState === "visible") refreshMonitors();
    };
    window.addEventListener("focus", onFocus);
    document.addEventListener("visibilitychange", onVisible);
    return () => {
      window.removeEventListener("focus", onFocus);
      document.removeEventListener("visibilitychange", onVisible);
    };
  }, [refreshMonitors]);

  // 全局快捷键触发后，后端推送事件刷新界面。
  useEffect(() => {
    const unlistenOk = listen<SwitchOutcome>("projection-switched", (e) => {
      setStatus(e.payload.status);
      showToast(
        "success",
        `已切换到：${MODE_LABEL[e.payload.switchedTo] ?? e.payload.switchedTo}`,
      );
    });
    const unlistenErr = listen<string>("projection-switch-error", (e) => {
      showToast("error", `切换失败：${e.payload}`);
    });
    const unlistenInput = listen<InputSwitchOutcome>("input-switched", (e) => {
      showToast(
        "success",
        `${e.payload.monitor} 输入源已切换：${
          e.payload.from === null ? "未知" : hex(e.payload.from)
        } → ${hex(e.payload.to)}`,
      );
      refreshMonitors();
    });
    const unlistenInputErr = listen<string>("input-switch-error", (e) => {
      showToast("error", `输入源切换失败：${e.payload}`);
    });
    return () => {
      unlistenOk.then((f) => f());
      unlistenErr.then((f) => f());
      unlistenInput.then((f) => f());
      unlistenInputErr.then((f) => f());
    };
  }, [showToast, refreshMonitors]);

  const toggleTheme = () => {
    const next = !dark;
    setDark(next);
    const name = next ? "dark" : "light";
    document.documentElement.dataset.theme = name;
    try {
      localStorage.setItem("theme", name);
    } catch {
      /* 浏览器存储不可用时忽略 */
    }
  };

  const runSwitch = useCallback(
    async (action: "toggle" | ProjectionMode) => {
      if (pending) return;
      setPending(action === "toggle" ? "toggle" : action);
      try {
        const outcome =
          action === "toggle"
            ? await projectionApi.toggleProjection()
            : await projectionApi.switchTo(action);
        setStatus(outcome.status);
        showToast(
          "success",
          `已切换到：${MODE_LABEL[outcome.switchedTo] ?? outcome.switchedTo}`,
        );
      } catch (e: unknown) {
        showToast("error", `切换失败：${String(e)}`);
        projectionApi
          .getStatus()
          .then(setStatus)
          .catch(() => {});
      } finally {
        setPending(null);
      }
    },
    [pending, showToast],
  );

  const applyShortcut = useCallback(
    async (kind: ShortcutKind, combo: string) => {
      try {
        const cfg = await configApi.setShortcut(kind, combo);
        setConfig(cfg);
        showToast("success", `全局快捷键已更新为 ${combo}`);
      } catch (e: unknown) {
        showToast("error", `快捷键设置失败：${String(e)}`);
      }
    },
    [showToast],
  );

  const runSwitchInput = useCallback(async () => {
    if (switchingInput) return;
    setSwitchingInput(true);
    try {
      const outcome = await monitorApi.switchInput();
      showToast(
        "success",
        `${outcome.monitor} 输入源已切换：${
          outcome.from === null ? "未知" : hex(outcome.from)
        } → ${hex(outcome.to)}`,
      );
      refreshMonitors();
    } catch (e: unknown) {
      showToast("error", `输入源切换失败：${String(e)}`);
    } finally {
      setSwitchingInput(false);
    }
  }, [switchingInput, showToast, refreshMonitors]);

  const applyInputMonitor = useCallback(
    async (name: string) => {
      try {
        const cfg = await configApi.setInputMonitor(name);
        setConfig(cfg);
      } catch (e: unknown) {
        showToast("error", `保存显示器选择失败：${String(e)}`);
      }
    },
    [showToast],
  );

  const applyAutostart = useCallback(
    async (enabled: boolean) => {
      setAutostart(enabled);
      try {
        const actual = await autostartApi.setAutostart(enabled);
        setAutostart(actual);
        showToast(
          "success",
          actual ? "已开启开机自启动（静默后台启动）" : "已关闭开机自启动",
        );
      } catch (e: unknown) {
        setAutostart(!enabled);
        showToast("error", `设置开机自启动失败：${String(e)}`);
      }
    },
    [showToast],
  );

  const handleResetLearning = useCallback(async () => {
    try {
      setStatus(await projectionApi.resetState());
      showToast("success", "已重置学习状态，将在下次切换时重新学习");
    } catch (e: unknown) {
      showToast("error", `重置失败：${String(e)}`);
    }
  }, [showToast]);

  // 录入模式下捕获按键组合。
  useEffect(() => {
    if (!recording) return;
    const onKeyDown = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      if (e.code === "Escape") {
        setRecording(null);
        return;
      }
      if (e.key === "Meta" || e.metaKey) {
        showToast("error", "不支持包含 Win 键的组合，请重试");
        return;
      }
      const main = mainKeyFromCode(e.code);
      if (!main) return; // 仅修饰键，继续等待主键
      const mods: string[] = [];
      if (e.ctrlKey) mods.push("Ctrl");
      if (e.altKey) mods.push("Alt");
      if (e.shiftKey) mods.push("Shift");
      if (mods.length === 0) {
        showToast("error", "需要至少一个修饰键（Ctrl / Alt / Shift）");
        return;
      }
      const kind = recording;
      setRecording(null);
      void applyShortcut(kind, [...mods, main].join("+"));
    };
    window.addEventListener("keydown", onKeyDown, { capture: true });
    return () =>
      window.removeEventListener("keydown", onKeyDown, { capture: true });
  }, [recording, applyShortcut, showToast]);

  const handleSelectModule = (id: ModuleId) => {
    setRecording(null); // 切换模块时退出快捷键录入态
    setActive(id);
  };

  return (
    <div className="flex h-full">
      <Sidebar
        active={active}
        onSelect={handleSelectModule}
        dark={dark}
        onToggleTheme={toggleTheme}
      />
      <main className="min-w-0 flex-1 overflow-y-auto px-6 py-6">
        {active === "projection" && (
          <ProjectionPanel
            status={status}
            config={config}
            pending={pending}
            recording={recording === "toggle"}
            onToggle={() => void runSwitch("toggle")}
            onSwitchTo={(mode) => void runSwitch(mode)}
            onToggleRecording={() =>
              setRecording(recording === "toggle" ? null : "toggle")
            }
            onResetLearning={() => void handleResetLearning()}
          />
        )}
        {active === "input" && (
          <InputSourcePanel
            monitors={monitors}
            config={config}
            switching={switchingInput}
            recording={recording === "input"}
            onSwitch={() => void runSwitchInput()}
            onSelectMonitor={(name) => void applyInputMonitor(name)}
            onToggleRecording={() =>
              setRecording(recording === "input" ? null : "input")
            }
          />
        )}
        {active === "settings" && (
          <SettingsPanel
            autostart={autostart}
            onToggleAutostart={(next) => void applyAutostart(next)}
          />
        )}
      </main>
      <Toast toast={toast} />
    </div>
  );
}
