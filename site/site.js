const desk = document.getElementById("desk");
const flyout = document.getElementById("flyout");
const frame = document.getElementById("demo-frame");
const tray = document.getElementById("tray-button");
const tip = document.getElementById("desk-tip");
const signin = document.getElementById("signin-screen");
const lock = document.getElementById("lock-screen");

const OPEN_TIP = "Press U or click the Userhop icon in the taskbar.";
let deskInView = false;

function post(message) {
  frame.contentWindow?.postMessage(message, window.location.origin);
}

function screenShown() {
  return !signin.hidden || !lock.hidden;
}

function setFlyout(visible) {
  flyout.dataset.state = visible ? "visible" : "hidden";
  tray.setAttribute("aria-expanded", String(visible));
  tray.setAttribute("aria-label", visible ? "Close Userhop" : "Open Userhop");
}

function openFlyout() {
  if (screenShown()) return;
  setFlyout(true);
  tip.textContent = "";
  frame.focus({ preventScroll: true });
  post({ type: "open" });
}

function closeFlyout() {
  const hadFocus = document.activeElement === frame;
  setFlyout(false);
  post({ type: "close" });
  tip.textContent = OPEN_TIP;
  if (hadFocus) tray.focus();
}

function initials(name) {
  return name
    .split(/\s+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((part) => part[0]?.toUpperCase() ?? "")
    .join("");
}

function showSignIn(name) {
  const known = name !== "Other user";
  document.getElementById("signin-avatar").textContent = known ? initials(name) : "";
  document.getElementById("signin-name").textContent = known ? name : "Sign in";
  document.getElementById("signin-text").textContent = known
    ? `Windows would now ask ${name.split(" ")[0]} to sign in.`
    : "Windows would now show its sign-in screen.";
  signin.dataset.account = known ? name : "";
  signin.hidden = false;
  document.getElementById("signin-back").focus();
}

function showLock() {
  updateClock();
  lock.hidden = false;
  document.getElementById("lock-back").focus();
}

function leaveScreens() {
  const account = signin.dataset.account;
  const fromSignIn = !signin.hidden;
  signin.hidden = true;
  lock.hidden = true;
  tip.textContent =
    fromSignIn && account
      ? `Back on Alex's desktop. Press U again: ${account} is now first in the list, marked Last used.`
      : OPEN_TIP;
  tray.focus();
}

window.addEventListener("message", (event) => {
  if (event.origin !== window.location.origin || event.source !== frame.contentWindow) return;
  const message = event.data;
  switch (message?.type) {
    case "height":
      frame.style.height = `${Math.ceil(message.value)}px`;
      break;
    case "hidden":
      if (flyout.dataset.state !== "hidden") {
        const hadFocus = document.activeElement === frame;
        setFlyout(false);
        tip.textContent = OPEN_TIP;
        if (hadFocus) tray.focus();
      }
      break;
    case "handoff":
      setFlyout(false);
      showSignIn(String(message.name));
      break;
    case "lock":
      setFlyout(false);
      showLock();
      break;
  }
});

tray.addEventListener("click", () => {
  if (flyout.dataset.state === "hidden") openFlyout();
  else closeFlyout();
});

document.getElementById("signin-back").addEventListener("click", leaveScreens);
document.getElementById("lock-back").addEventListener("click", leaveScreens);
lock.addEventListener("click", (event) => {
  if (event.target === lock) leaveScreens();
});

function typing(target) {
  return (
    target instanceof HTMLElement &&
    (target.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName))
  );
}

document.addEventListener("keydown", (event) => {
  if (event.defaultPrevented || typing(event.target)) return;
  if (screenShown()) {
    if (event.key === "Escape") leaveScreens();
    return;
  }
  const plainU =
    event.key.toLowerCase() === "u" && !event.ctrlKey && !event.metaKey && !event.altKey;
  const macShortcut = event.code === "KeyU" && event.ctrlKey && event.altKey;
  if ((plainU || macShortcut) && deskInView) {
    event.preventDefault();
    openFlyout();
  }
});

new IntersectionObserver(
  ([entry]) => {
    deskInView = entry.isIntersecting;
  },
  { threshold: 0.35 },
).observe(desk);

function updateClock() {
  const now = new Date();
  const time = new Intl.DateTimeFormat(undefined, { hour: "numeric", minute: "2-digit" }).format(
    now,
  );
  const shortDate = new Intl.DateTimeFormat(undefined, {
    day: "numeric",
    month: "numeric",
    year: "numeric",
  }).format(now);
  const longDate = new Intl.DateTimeFormat(undefined, {
    weekday: "long",
    month: "long",
    day: "numeric",
  }).format(now);
  document.getElementById("clock-time").textContent = time;
  document.getElementById("clock-date").textContent = shortDate;
  document.getElementById("lock-time").textContent = time;
  document.getElementById("lock-date").textContent = longDate;
}

updateClock();
setInterval(updateClock, 30_000);
setFlyout(true);

function detectOs() {
  const platform = (navigator.userAgentData?.platform ?? navigator.platform ?? "").toLowerCase();
  const agent = navigator.userAgent.toLowerCase();
  if (/android|iphone|ipad/.test(agent)) return null;
  if (platform.includes("win")) return "windows";
  if (platform.includes("mac")) return "mac";
  if (platform.includes("linux")) return "linux";
  return null;
}

const os = detectOs();
if (os) {
  const labels = {
    windows: "Download for Windows",
    mac: "Download for macOS",
    linux: "Download for Linux",
  };
  const primary = document.getElementById("primary-download");
  primary.textContent = labels[os];
  for (const row of document.querySelectorAll(`.files tr[data-os="${os}"]`)) {
    row.dataset.current = "";
  }
}
