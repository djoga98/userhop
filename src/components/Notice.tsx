import { AlertCircle } from "lucide-react";
import type { ReactNode } from "react";

type NoticeProps = {
  message: string;
  tone?: "error" | "info";
  children?: ReactNode;
};

export function Notice({ message, tone = "error", children }: NoticeProps) {
  return (
    <div
      role={tone === "error" ? "alert" : "status"}
      className="mx-2 mb-2 flex flex-col gap-2 rounded-row border border-border bg-raised px-3 py-2.5"
    >
      <p className="flex items-start gap-2 text-[12.5px]">
        <AlertCircle
          aria-hidden="true"
          size={16}
          className={`mt-px shrink-0 ${tone === "error" ? "text-danger" : "text-secondary"}`}
        />
        <span>{message}</span>
      </p>
      {children && <div className="flex flex-wrap gap-2 pl-6">{children}</div>}
    </div>
  );
}

type NoticeActionProps = { onClick: () => void; children: ReactNode; primary?: boolean };

export function NoticeAction({ onClick, children, primary = false }: NoticeActionProps) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={`rounded-md px-2.5 py-1 text-[12.5px] font-medium transition-[color,background-color,border-color] duration-100 ${
        primary
          ? "bg-accent text-on-accent hover:opacity-90"
          : "bg-control text-text hover:bg-hover"
      }`}
    >
      {children}
    </button>
  );
}
