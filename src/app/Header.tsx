import { ArrowLeft, Settings, UsersRound, X } from "lucide-react";
import type { Ref } from "react";

import { IconButton } from "../components/IconButton";

type HeaderProps = {
  view: "list" | "settings";
  onBack: () => void;
  onSettings: () => void;
  onClose: () => void;
  ref?: Ref<HTMLElement>;
};

export function Header({ view, onBack, onSettings, onClose, ref }: HeaderProps) {
  return (
    <header ref={ref} className="flex h-[52px] shrink-0 items-center gap-2 pr-2.5 pl-3">
      {view === "settings" ? (
        <IconButton label="Back to accounts" onClick={onBack}>
          <ArrowLeft aria-hidden="true" size={16} />
        </IconButton>
      ) : (
        <span aria-hidden="true" className="grid size-8 place-items-center text-accent">
          <UsersRound size={18} />
        </span>
      )}
      <h1 id="flyout-title" className="flex-1 truncate text-[14.5px] font-semibold">
        {view === "settings" ? "Settings" : "Switch account"}
      </h1>
      {view === "list" && (
        <IconButton label="Settings" onClick={onSettings}>
          <Settings aria-hidden="true" size={16} />
        </IconButton>
      )}
      <IconButton label="Close" onClick={onClose}>
        <X aria-hidden="true" size={16} />
      </IconButton>
    </header>
  );
}
