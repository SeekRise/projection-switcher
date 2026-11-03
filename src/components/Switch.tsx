import { cn } from "../lib/utils";

interface SwitchProps {
  checked: boolean;
  disabled?: boolean;
  onChange: (next: boolean) => void;
  label?: string;
}

/** 轻量开关（未引入 Radix，视觉遵循 shadcn Switch 规范） */
export function Switch({ checked, disabled, onChange, label }: SwitchProps) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={label}
      disabled={disabled}
      onClick={() => onChange(!checked)}
      className={cn(
        "h-5 w-9 shrink-0 rounded-full p-0.5 transition-colors focus-visible:outline-2 focus-visible:outline-accent",
        checked ? "bg-accent" : "bg-border",
        disabled && "cursor-not-allowed opacity-60",
      )}
    >
      <span
        className={cn(
          "block size-4 rounded-full bg-white transition-transform duration-150",
          checked && "translate-x-4",
        )}
      />
    </button>
  );
}
