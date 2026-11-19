import { MousePointer2 } from "lucide-react";
import { useEffect, useRef, useState, useSyncExternalStore, type CSSProperties } from "react";

import iconUrl from "../../assets/app-icon.svg";
import { App } from "../app/App";
import { heightStore } from "./heightStore";
import {
  CAPTIONS,
  CLICKS,
  END_CARD,
  FLYOUT_WINDOWS,
  KEY_STAGGER_MS,
  KEY_VISIBLE_MS,
  KEYSTROKES,
  SIGN_IN,
  easeInOutCubic,
  easeOutCubic,
  envelope,
  mix,
  progress,
} from "./timeline";
import { Taskbar } from "./Taskbar";

declare global {
  interface Window {
    __setTime?: (ms: number) => Promise<void>;
  }
}

type Point = { x: number; y: number };
type Frame = { t: number; cursor: Point | null };

const CURSOR_REST: Point = { x: 1180, y: 690 };

function center(stage: HTMLElement, element: Element | null | undefined): Point | null {
  if (!element) return null;
  const s = stage.getBoundingClientRect();
  const r = element.getBoundingClientRect();
  return { x: r.left - s.left + r.width / 2, y: r.top - s.top + r.height / 2 };
}

function rows() {
  return document.querySelectorAll<HTMLButtonElement>("[data-row]");
}

type Action = { at: number; run: () => void };

function createActions(): Action[] {
  return [
    { at: 0, run: () => rows()[0]?.focus() },
    { at: 2650, run: () => rows()[1]?.focus() },
    { at: FLYOUT_WINDOWS[1]?.open ?? 7750, run: () => rows()[0]?.focus() },
    {
      at: CLICKS.settings,
      run: () => {
        document.querySelector<HTMLButtonElement>('button[aria-label="Settings"]')?.click();
      },
    },
    {
      at: CLICKS.light,
      run: () => {
        document.querySelector<HTMLInputElement>('input[name="theme"][value="light"]')?.click();
      },
    },
    {
      at: 11050,
      run: () => {
        window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" }));
      },
    },
  ].sort((a, b) => a.at - b.at);
}

let gearPosition: Point | null = null;

function cursorAt(t: number, stage: HTMLElement): Point | null {
  if (t < CLICKS.settings - 700 || t > CLICKS.light + 900) return null;
  gearPosition =
    center(stage, document.querySelector('button[aria-label="Settings"]')) ?? gearPosition;
  const gear = gearPosition;
  const light = center(
    stage,
    document.querySelector('input[name="theme"][value="light"]')?.closest("label"),
  );
  if (t <= CLICKS.settings) {
    const k = easeInOutCubic(progress(t, CLICKS.settings - 700, CLICKS.settings - 80));
    const to = gear ?? CURSOR_REST;
    return { x: mix(CURSOR_REST.x, to.x, k), y: mix(CURSOR_REST.y, to.y, k) };
  }
  const from = gear ?? CURSOR_REST;
  const to = light ?? from;
  const k = easeInOutCubic(progress(t, CLICKS.settings + 340, CLICKS.light - 80));
  return { x: mix(from.x, to.x, k), y: mix(from.y, to.y, k) };
}

function flyoutStyle(t: number): CSSProperties {
  for (const window of FLYOUT_WINDOWS) {
    if (t < window.open || t > window.close + 220) continue;
    if (t < window.open + 260) {
      const k = easeOutCubic(progress(t, window.open, window.open + 260));
      return {
        opacity: k,
        transform: `translateY(${String(mix(14, 0, k))}px) scale(${String(mix(0.975, 1, k))})`,
      };
    }
    if (t > window.close) {
      const k = easeInOutCubic(progress(t, window.close, window.close + 220));
      return { opacity: 1 - k, transform: `translateY(${String(mix(0, 8, k))}px)` };
    }
    return { opacity: 1 };
  }
  return { opacity: 0, visibility: "hidden" };
}

function Keycaps({ t }: { t: number }) {
  const active = KEYSTROKES.find((k) => t >= k.at - 200 && t <= k.at + KEY_VISIBLE_MS);
  if (!active) return null;
  const opacity = envelope(
    t,
    active.at - 200,
    active.at,
    active.at + KEY_VISIBLE_MS - 250,
    active.at + KEY_VISIBLE_MS,
  );
  const pressAt = active.at + (active.keys.length - 1) * KEY_STAGGER_MS + 120;
  const pressed = t >= pressAt && t <= pressAt + 160;
  return (
    <div className="video-keys" style={{ opacity }}>
      {active.keys.map((key, index) => {
        const appear = easeOutCubic(
          progress(t, active.at + index * KEY_STAGGER_MS, active.at + index * KEY_STAGGER_MS + 180),
        );
        return (
          <span key={key} className="video-key-group" style={{ opacity: appear }}>
            {index > 0 && <span className="video-plus">+</span>}
            <kbd
              className={pressed ? "video-key video-key-pressed" : "video-key"}
              style={{
                transform: `translateY(${String(mix(10, 0, appear))}px) scale(${String(mix(0.85, 1, appear))})`,
              }}
            >
              {key}
            </kbd>
          </span>
        );
      })}
    </div>
  );
}

function Captions({ t }: { t: number }) {
  return (
    <>
      {CAPTIONS.map((caption) => {
        const k = envelope(t, caption.in, caption.in + 600, caption.out - 400, caption.out);
        if (k === 0) return null;
        return (
          <div
            key={caption.title}
            className="video-caption"
            style={{
              opacity: k,
              transform: `translateY(calc(-50% + ${String(mix(18, 0, k))}px))`,
            }}
          >
            <p className="video-caption-title">{caption.title}</p>
            <p className="video-caption-body">{caption.body}</p>
          </div>
        );
      })}
    </>
  );
}

function SignInScene({ t }: { t: number }) {
  const k = envelope(t, SIGN_IN.in, SIGN_IN.in + 450, SIGN_IN.out - 450, SIGN_IN.out);
  if (k === 0) return null;
  const card = easeOutCubic(progress(t, SIGN_IN.in + 150, SIGN_IN.in + 650));
  const note = envelope(t, SIGN_IN.in + 900, SIGN_IN.in + 1400, SIGN_IN.out - 450, SIGN_IN.out);
  return (
    <div className="desktop-screen video-signin" style={{ opacity: k }}>
      <div
        className="video-signin-card"
        style={{
          opacity: card,
          transform: `translateY(${String(mix(24, 0, card))}px)`,
        }}
      >
        <span className="desktop-avatar">SR</span>
        <p className="video-signin-name">Sam Rivera</p>
        <p className="video-signin-text">Windows asks Sam to sign in here.</p>
        <p className="video-signin-note" style={{ opacity: note }}>
          Userhop never sees this screen. Your apps keep running.
        </p>
      </div>
    </div>
  );
}

function EndCard({ t }: { t: number }) {
  const k = easeInOutCubic(progress(t, END_CARD.in, END_CARD.in + 800));
  if (k === 0) return null;
  const radius = `${String(mix(0, 150, k))}%`;
  const content = easeOutCubic(progress(t, END_CARD.in + 300, END_CARD.in + 1100));
  const details = easeOutCubic(progress(t, END_CARD.in + 700, END_CARD.in + 1400));
  return (
    <div className="video-end" style={{ clipPath: `circle(${radius} at 97% 97%)` }}>
      <div
        style={{ opacity: content, transform: `translateY(${String(mix(20, 0, content))}px)` }}
        className="video-end-content"
      >
        <img src={iconUrl} alt="" className="video-end-icon" />
        <p className="video-end-title">Userhop</p>
        <p className="video-end-body">Switch users in one keystroke.</p>
        <p className="video-end-details" style={{ opacity: details }}>
          Windows · macOS · Linux · free and open source
        </p>
      </div>
    </div>
  );
}

export function VideoStage() {
  const stageRef = useRef<HTMLDivElement>(null);
  const [frame, setFrame] = useState<Frame>({ t: 0, cursor: null });
  const height = useSyncExternalStore(heightStore.subscribe, heightStore.get);

  useEffect(() => {
    const actions = createActions();
    let next = 0;
    window.__setTime = (ms: number) =>
      new Promise((resolve) => {
        while (next < actions.length && (actions[next]?.at ?? Infinity) <= ms) {
          actions[next]?.run();
          next += 1;
        }
        const stage = stageRef.current;
        setFrame({ t: ms, cursor: stage ? cursorAt(ms, stage) : null });
        requestAnimationFrame(() => {
          requestAnimationFrame(() => {
            resolve();
          });
        });
      });
    return () => {
      delete window.__setTime;
    };
  }, []);

  const { t, cursor } = frame;
  const flyoutOpen = FLYOUT_WINDOWS.some((w) => t >= w.open && t <= w.close);
  const clickRipple = [CLICKS.settings, CLICKS.light]
    .map((at) => progress(t, at, at + 320))
    .find((p) => p > 0 && p < 1);

  return (
    <div ref={stageRef} className="desktop preview-stage video-stage" data-theme="dark">
      <Captions t={t} />
      <div
        className="desktop-flyout preview-window video-window"
        style={{ height, ...flyoutStyle(t) }}
      >
        <App />
      </div>
      <Taskbar active={flyoutOpen} />
      <Keycaps t={t} />
      <SignInScene t={t} />
      {cursor && (
        <div className="video-cursor" style={{ left: cursor.x, top: cursor.y }}>
          {clickRipple !== undefined && (
            <span
              className="video-ripple"
              style={{
                opacity: 1 - clickRipple,
                transform: `translate(-50%, -50%) scale(${String(mix(0.4, 1.6, clickRipple))})`,
              }}
            />
          )}
          <MousePointer2 size={28} aria-hidden="true" className="video-cursor-icon" />
        </div>
      )}
      <EndCard t={t} />
    </div>
  );
}
