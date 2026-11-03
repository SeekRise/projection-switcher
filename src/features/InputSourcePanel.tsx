import { ArrowLeftRight, Loader2, MonitorUp } from "lucide-react";
import { ShortcutRow } from "../components/ShortcutRow";
import { hex, type AppConfig, type MonitorInfo } from "../lib/api";

interface InputSourcePanelProps {
  monitors: MonitorInfo[] | null;
  config: AppConfig | null;
  switching: boolean;
  recording: boolean;
  onSwitch: () => void;
  onSelectMonitor: (name: string) => void;
  onToggleRecording: () => void;
}

export function InputSourcePanel({
  monitors,
  config,
  switching,
  recording,
  onSwitch,
  onSelectMonitor,
  onToggleRecording,
}: InputSourcePanelProps) {
  const target = monitors?.find((m) => m.name === config?.inputMonitor) ?? null;
  const configName = config?.inputMonitor ?? "";

  const statusTitle =
    monitors === null
      ? "读取中…"
      : target === null
        ? "未连接"
        : target.inputSource == null
          ? "读取失败"
          : hex(target.inputSource);

  const statusHint =
    monitors === null
      ? "正在枚举显示器…"
      : target === null
        ? `${configName} 未在当前活动显示器中（显示器可能正显示其他输入源，切回后点一下窗口即可刷新）`
        : target.inputSource == null
          ? `${target.name} 未响应 DDC/CI（显示器可能正显示其他输入源；仍可尝试切换）`
          : `${target.name} · 在配置值列表中循环切换`;

  return (
    <div className="flex flex-col gap-5">
      <header>
        <h2 className="text-lg font-semibold tracking-tight">输入源</h2>
        <p className="mt-0.5 text-xs text-muted">
          通过 DDC/CI 切换外接显示器输入源（需显示器支持）
        </p>
      </header>

      <section>
        <div className="mb-1.5 text-xs text-muted">当前输入源</div>
        <div className="flex items-center gap-2.5">
          <span
            className={
              target?.inputSource != null
                ? "size-2 rounded-full bg-ok"
                : "size-2 rounded-full bg-warn"
            }
          />
          <span className="text-lg font-semibold tracking-tight">
            {statusTitle}
          </span>
        </div>
        <div className="mt-1.5 text-xs leading-5 text-muted">{statusHint}</div>
      </section>

      <section className="flex flex-col gap-2.5">
        <button
          type="button"
          onClick={onSwitch}
          disabled={switching || monitors === null}
          className="flex h-11 items-center justify-center gap-2 rounded-md bg-accent text-[13px] font-medium text-white transition-colors hover:bg-accent-hover disabled:cursor-not-allowed disabled:opacity-60 focus-visible:outline-2 focus-visible:outline-accent focus-visible:outline-offset-2"
        >
          {switching ? (
            <Loader2 size={16} className="animate-spin" />
          ) : (
            <ArrowLeftRight size={16} />
          )}
          {switching ? "正在切换…" : "切换输入源"}
        </button>
        <div className="flex items-center gap-2">
          <MonitorUp size={14} className="shrink-0 text-muted" />
          <select
            value={config?.inputMonitor ?? ""}
            onChange={(e) => onSelectMonitor(e.target.value)}
            disabled={monitors === null || monitors.length === 0}
            title="目标显示器（EDID 名称）"
            className="h-8 min-w-0 flex-1 rounded-md border border-border bg-panel px-2 text-xs text-text focus-visible:outline-2 focus-visible:outline-accent disabled:opacity-60"
          >
            {monitors === null ? (
              <option value="">枚举中…</option>
            ) : monitors.length === 0 ? (
              <option value="">未找到物理显示器</option>
            ) : (
              <>
                {target === null && configName !== "" && (
                  <option value={configName}>
                    {configName}（未检测到，正显示其他输入源？）
                  </option>
                )}
                {monitors.map((m) => (
                  <option key={m.name} value={m.name}>
                    {m.name}
                    {m.inputSource === null ? "" : `（${hex(m.inputSource)}）`}
                  </option>
                ))}
              </>
            )}
          </select>
        </div>
      </section>

      <section className="border-t border-border pt-4">
        <ShortcutRow
          label="切换快捷键"
          value={config?.inputShortcut}
          recording={recording}
          onToggleRecording={onToggleRecording}
        />
      </section>

      <section className="mt-auto border-t border-border pt-4">
        <div className="mb-2 text-xs text-muted">切换值列表（VCP 0x60）</div>
        <div className="font-mono text-xs leading-5 text-text">
          {(config?.inputValues ?? []).map(hex).join("  ⇄  ") || "未配置"}
        </div>
        <p className="mt-2.5 text-[11px] leading-4 text-muted">
          当前值在列表中则切到下一个（末尾回绕），否则切到第一个值；
          可编辑 %APPDATA%\ProjectionSwitcher.json 的 inputValues 增删输入源
        </p>
      </section>
    </div>
  );
}
