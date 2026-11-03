import { Keyboard } from "lucide-react";
import { cn } from "../lib/utils";

interface ShortcutRowProps {
  label: string;
  /** 当前快捷键值（加载中为 undefined） */
  value: string | undefined;
  /** 本行是否处于录入状态 */
  recording: boolean;
  onToggleRecording: () => void;
}

/** 快捷键设置行：标签 + 键帽展示 + 录入按钮（Esc 取消）。 */
export function ShortcutRow({
  label,
  value,
  recording,
  onToggleRecording,
}: ShortcutRowProps) {
  return (
    <div className="flex items-center justify-between gap-3">
      <div className="flex min-w-0 items-center gap-2 text-muted">
        <Keyboard size={14} className="shrink-0" />
        <span className="text-xs">{label}</span>
      </div>
      <div className="flex min-w-0 items-center gap-2">
        <kbd
          className={cn(
            "truncate rounded border border-border px-2 py-1 font-mono text-xs text-text",
            recording && "border-accent text-accent",
          )}
        >
          {recording ? "按下新组合…" : (value ?? "…")}
        </kbd>
        <button
          type="button"
          onClick={onToggleRecording}
          className={cn(
            "h-7 shrink-0 rounded-md border px-2.5 text-xs transition-colors focus-visible:outline-2 focus-visible:outline-accent",
            recording
              ? "border-accent text-accent"
              : "border-border text-muted hover:text-text",
          )}
        >
          {recording ? "取消" : "修改"}
        </button>
      </div>
    </div>
  );
}
