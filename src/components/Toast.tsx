import { CheckCircle2, AlertCircle } from "lucide-react";
import { cn } from "../lib/utils";

export interface ToastState {
  id: number;
  kind: "success" | "error";
  message: string;
}

export function Toast({ toast }: { toast: ToastState | null }) {
  if (!toast) return null;
  return (
    <div
      key={toast.id}
      role="status"
      className="toast-enter fixed bottom-4 left-1/2 flex max-w-[calc(100%-32px)] items-center gap-2 rounded-md border border-border bg-panel px-3 py-2 text-xs shadow-lg"
    >
      {toast.kind === "success" ? (
        <CheckCircle2 size={14} className="shrink-0 text-ok" />
      ) : (
        <AlertCircle size={14} className="shrink-0 text-danger" />
      )}
      <span className={cn(toast.kind === "error" && "text-danger")}>
        {toast.message}
      </span>
    </div>
  );
}
