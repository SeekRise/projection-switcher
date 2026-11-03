import { useState } from "react";
import { ArrowLeftRight, Copy, Expand, Loader2, Monitor, MonitorUp, RefreshCw } from "lucide-react";
import { ShortcutRow } from "../components/ShortcutRow";
import {
  DETECTED_LABEL,
  type AppConfig,
  type ProjectionMode,
  type ProjectionStatus,
} from "../lib/api";
import { cn } from "../lib/utils";

const MODES: Array<{ id: ProjectionMode; label: string; icon: typeof Monitor }> = [
  { id: "internal", label: "仅电脑屏幕", icon: Monitor },
  { id: "clone", label: "复制", icon: Copy },
  { id: "extend", label: "扩展", icon: Expand },
  { id: "external", label: "仅第二屏幕", icon: MonitorUp },
];

interface ProjectionPanelProps {
  status: ProjectionStatus | null;
  config: AppConfig | null;
  pending: "toggle" | ProjectionMode | null;
  recording: boolean;
  onToggle: () => void;
  onSwitchTo: (mode: ProjectionMode) => void;
  onToggleRecording: () => void;
  onResetLearning: () => void;
}

export function ProjectionPanel({
  status,
  config,
  pending,
  recording,
  onToggle,
  onSwitchTo,
  onToggleRecording,
  onResetLearning,
}: ProjectionPanelProps) {
  const [confirmReset, setConfirmReset] = useState(false);

  const detected = status?.detected ?? "unknown";
  const known = detected === "internal" || detected === "external";

  const handleReset = () => {
    if (!confirmReset) {
      setConfirmReset(true);
      window.setTimeout(() => setConfirmReset(false), 3000);
      return;
    }
    setConfirmReset(false);
    onResetLearning();
  };

  return (
    <div className="flex flex-col gap-5">
      <header>
        <h2 className="text-lg font-semibold tracking-tight">投影模式</h2>
        <p className="mt-0.5 text-xs text-muted">
          在「仅电脑屏幕」与「仅第二屏幕」之间智能取反（分辨率签名 + 状态记忆）
        </p>
      </header>

      <section>
        <div className="mb-1.5 text-xs text-muted">当前模式</div>
        <div className="flex items-center gap-2.5">
          <span
            className={cn("size-2 rounded-full", known ? "bg-ok" : "bg-warn")}
          />
          <span className="text-lg font-semibold tracking-tight">
            {DETECTED_LABEL[detected] ?? "未知"}
          </span>
        </div>
        <div className="mt-1.5 text-xs leading-5 text-muted">
          {status === null
            ? "正在读取…"
            : status.currentSignature === ""
              ? "未获取到当前分辨率"
              : `当前输出分辨率 ${status.currentSignature}`}
        </div>
      </section>

      <section className="flex flex-col gap-3">
        <button
          type="button"
          onClick={onToggle}
          disabled={pending !== null}
          className="flex h-11 items-center justify-center gap-2 rounded-md bg-accent text-[13px] font-medium text-white transition-colors hover:bg-accent-hover disabled:cursor-not-allowed disabled:opacity-60 focus-visible:outline-2 focus-visible:outline-accent focus-visible:outline-offset-2"
        >
          {pending === "toggle" ? (
            <Loader2 size={16} className="animate-spin" />
          ) : (
            <ArrowLeftRight size={16} />
          )}
          {pending === "toggle" ? "正在切换…" : "一键切换"}
        </button>
        <div className="grid grid-cols-2 gap-2.5">
          {MODES.map(({ id, label, icon: Icon }) => (
            <button
              key={id}
              type="button"
              onClick={() => onSwitchTo(id)}
              disabled={pending !== null}
              title={label}
              className="flex h-[62px] flex-col items-center justify-center gap-1.5 rounded-md border border-border bg-panel transition-colors hover:border-accent/50 hover:text-text disabled:cursor-not-allowed disabled:opacity-60 focus-visible:outline-2 focus-visible:outline-accent"
            >
              {pending === id ? (
                <Loader2 size={18} className="animate-spin text-muted" />
              ) : (
                <Icon size={18} className="text-muted" />
              )}
              <span className="text-xs">{label}</span>
            </button>
          ))}
        </div>
      </section>

      <section className="border-t border-border pt-4">
        <ShortcutRow
          label="切换快捷键"
          value={config?.shortcut}
          recording={recording}
          onToggleRecording={onToggleRecording}
        />
      </section>

      <section className="mt-auto border-t border-border pt-4">
        <div className="mb-2 text-xs text-muted">已学习的屏幕签名</div>
        <div className="flex items-center justify-between gap-3">
          <div className="text-xs leading-5 text-muted">
            <div>
              内置屏：
              <span className="text-text">{status?.state.sigInt || "未学习"}</span>
            </div>
            <div>
              外接屏：
              <span className="text-text">{status?.state.sigExt || "未学习"}</span>
            </div>
          </div>
          <button
            type="button"
            onClick={handleReset}
            className={cn(
              "flex h-7 shrink-0 items-center gap-1.5 rounded-md border px-2.5 text-xs transition-colors focus-visible:outline-2 focus-visible:outline-accent",
              confirmReset
                ? "border-danger text-danger"
                : "border-border text-muted hover:text-text",
            )}
          >
            <RefreshCw size={13} />
            {confirmReset ? "确认重置？" : "重置学习"}
          </button>
        </div>
        <p className="mt-2.5 text-[11px] leading-4 text-muted">
          换显示器或修改分辨率后会自动重新学习
        </p>
      </section>
    </div>
  );
}
