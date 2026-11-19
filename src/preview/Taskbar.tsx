import iconUrl from "../../assets/app-icon.svg";

/** Static taskbar for README media; mirrors the one on the landing page. */
export function Taskbar({ active }: { active: boolean }) {
  return (
    <div className="desktop-taskbar" aria-hidden="true">
      <div className="desktop-apps">
        <span className="desktop-app">
          <svg viewBox="0 0 24 24">
            <path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />
          </svg>
        </span>
        <span className="desktop-app">
          <svg viewBox="0 0 24 24">
            <circle cx="12" cy="12" r="8.5" />
            <path d="M3.5 12h17M12 3.5c2.4 2.4 3.5 5.3 3.5 8.5s-1.1 6.1-3.5 8.5c-2.4-2.4-3.5-5.3-3.5-8.5s1.1-6.1 3.5-8.5z" />
          </svg>
        </span>
        <span className="desktop-app">
          <svg viewBox="0 0 24 24">
            <rect x="3.5" y="4.5" width="17" height="15" rx="2" />
            <path d="M7 9.5l3 2.5-3 2.5M12.5 15h4" />
          </svg>
        </span>
        <span className="desktop-app">
          <svg viewBox="0 0 24 24">
            <path d="M5 4.5h10l4 4v11H5z" />
            <path d="M8.5 12h7M8.5 15.5h7" />
          </svg>
        </span>
      </div>
      <div className="desktop-tray">
        <span className="desktop-tray-button" data-active={active ? "" : undefined}>
          <img src={iconUrl} alt="" width={18} height={18} />
        </span>
        <span className="desktop-status">
          <svg viewBox="0 0 24 24">
            <path d="M4 9.5a12 12 0 0 1 16 0M7 12.8a7.5 7.5 0 0 1 10 0M10 16a3 3 0 0 1 4 0" />
            <circle cx="12" cy="18.6" r="0.9" />
          </svg>
          <svg viewBox="0 0 24 24">
            <path d="M5 9.5h3l4-3.5v12l-4-3.5H5z" />
            <path d="M15.5 9a4.5 4.5 0 0 1 0 6M18 7a7.5 7.5 0 0 1 0 10" />
          </svg>
        </span>
        <span className="desktop-clock">
          <span>10:24 AM</span>
          <span>11/19/2026</span>
        </span>
      </div>
    </div>
  );
}
