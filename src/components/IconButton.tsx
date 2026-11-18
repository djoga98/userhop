import type { ReactNode } from "react";

type IconButtonProps = {
  label: string;
  onClick: () => void;
  children: ReactNode;
  disabled?: boolean;
};

export function IconButton({ label, onClick, children, disabled = false }: IconButtonProps) {
  return (
    <button
      type="button"
      aria-label={label}
      title={label}
      disabled={disabled}
      onClick={onClick}
      className="grid size-8 shrink-0 place-items-center rounded-md text-secondary transition-colors duration-100 hover:bg-hover hover:text-text active:bg-pressed disabled:opacity-40"
    >
      {children}
    </button>
  );
}
