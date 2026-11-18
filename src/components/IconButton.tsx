import type { ReactNode } from "react";

type IconButtonProps = {
  label: string;
  onClick: () => void;
  children: ReactNode;
  disabled?: boolean;
  autoFocus?: boolean;
};

export function IconButton({
  label,
  onClick,
  children,
  disabled = false,
  autoFocus = false,
}: IconButtonProps) {
  return (
    <button
      type="button"
      autoFocus={autoFocus}
      aria-label={label}
      title={label}
      disabled={disabled}
      onClick={onClick}
      className="grid size-8 shrink-0 place-items-center rounded-md text-secondary transition-[color,background-color,border-color] duration-100 hover:bg-hover hover:text-text active:bg-pressed disabled:opacity-40"
    >
      {children}
    </button>
  );
}
