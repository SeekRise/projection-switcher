import { ArrowLeftRight, Monitor, MonitorUp, Settings2, Sun, Moon } from "lucide-react";
import { cn } from "../lib/utils";

export type ModuleId = "projection" | "input" | "settings";

const NAV_ITEMS: Array<{
  id: ModuleId;
  label: string;
  icon: typeof Monitor;
}> = [
  { id: "projection", label: "投影模式", icon: Monitor },
  { id: "input", label: "输入源", icon: ArrowLeftRight },
  { id: "settings", label: "设置", icon: Settings2 },
];

interface SidebarProps {
  active: ModuleId;
  onSelect: (id: ModuleId) => void;
  dark: boolean;
  onToggleTheme: () => void;
}

export function Sidebar({ active, onSelect, dark, onToggleTheme }: SidebarProps) {
  return (
    <aside className="flex w-52 shrink-0 flex-col border-r border-border">
      <div className="flex h-12 shrink-0 items-center gap-2 px-4">
        <MonitorUp size={16} className="text-accent" />
        <span className="text-[13px] font-semibold tracking-tight">投影切换</span>
      </div>
      <nav className="flex-1 space-y-0.5 px-2 py-2">
        {NAV_ITEMS.map(({ id, label, icon: Icon }) => (
          <button
            key={id}
            type="button"
            onClick={() => onSelect(id)}
            className={cn(
              "flex h-8 w-full items-center gap-2 rounded-md px-2.5 text-[13px] transition-colors focus-visible:outline-2 focus-visible:outline-accent",
              active === id
                ? "bg-panel text-text"
                : "text-muted hover:text-text",
            )}
          >
            <Icon size={15} className="shrink-0" />
            {label}
          </button>
        ))}
      </nav>
      <div className="border-t border-border p-2">
        <button
          type="button"
          onClick={onToggleTheme}
          title={dark ? "切换到浅色主题" : "切换到深色主题"}
          className="flex h-8 w-full items-center gap-2 rounded-md px-2.5 text-[13px] text-muted transition-colors hover:text-text focus-visible:outline-2 focus-visible:outline-accent"
        >
          {dark ? <Sun size={15} /> : <Moon size={15} />}
          {dark ? "浅色主题" : "深色主题"}
        </button>
      </div>
    </aside>
  );
}
