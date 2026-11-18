#!/usr/bin/env node
// Renders the README video frame by frame through the Chrome DevTools protocol.
// Usage: node scripts/record-readme-video.mjs --browser <chrome|edge> --url <preview url with ?video=1> --out <dir>
import { spawn } from "node:child_process";
import { mkdir, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { setTimeout as sleep } from "node:timers/promises";

const options = { fps: "30", duration: "15000", scale: "1.5", width: "1280", height: "720" };
for (let i = 2; i < process.argv.length; i += 2)
  options[process.argv[i].replace(/^--/, "")] = process.argv[i + 1];
for (const required of ["browser", "url", "out"]) {
  if (!options[required]) throw new Error(`missing --${required}`);
}
const fps = Number(options.fps);
const duration = Number(options.duration);
const scale = Number(options.scale);
const width = Number(options.width);
const height = Number(options.height);

await mkdir(join(options.out, "frames"), { recursive: true });
const browser = spawn(
  options.browser,
  [
    "--headless=new",
    "--disable-gpu",
    "--hide-scrollbars",
    "--no-first-run",
    "--allow-file-access-from-files",
    `--user-data-dir=${join(options.out, "profile")}`,
    "--remote-debugging-pipe",
    `--window-size=${width},${height}`,
    "about:blank",
  ],
  { stdio: ["ignore", "ignore", "ignore", "pipe", "pipe"] },
);
const [, , , toBrowser, fromBrowser] = browser.stdio;

let nextId = 0;
let buffered = "";
const pending = new Map();
fromBrowser.on("data", (chunk) => {
  buffered += chunk.toString("utf8");
  let end;
  while ((end = buffered.indexOf("\0")) >= 0) {
    const message = JSON.parse(buffered.slice(0, end));
    buffered = buffered.slice(end + 1);
    const waiter = pending.get(message.id);
    if (!waiter) continue;
    pending.delete(message.id);
    if (message.error) waiter.reject(new Error(message.error.message));
    else waiter.resolve(message.result);
  }
});
let sessionId;
const send = (method, params = {}) =>
  new Promise((resolve, reject) => {
    nextId += 1;
    pending.set(nextId, { resolve, reject });
    const message = { id: nextId, method, params, ...(sessionId ? { sessionId } : {}) };
    toBrowser.write(`${JSON.stringify(message)}\0`);
  });

const { targetInfos } = await send("Target.getTargets");
const page = targetInfos.find((t) => t.type === "page");
if (!page) throw new Error("browser has no page target");
({ sessionId } = await send("Target.attachToTarget", { targetId: page.targetId, flatten: true }));

const evaluate = async (expression) => {
  const { result, exceptionDetails } = await send("Runtime.evaluate", {
    expression,
    awaitPromise: true,
    returnByValue: true,
  });
  if (exceptionDetails) throw new Error(exceptionDetails.text);
  return result.value;
};

await send("Emulation.setFocusEmulationEnabled", { enabled: true });
await send("Emulation.setDeviceMetricsOverride", {
  width,
  height,
  deviceScaleFactor: scale,
  mobile: false,
});
await send("Page.navigate", { url: options.url });
for (let attempt = 0; !(await evaluate("typeof window.__setTime === 'function'")); attempt += 1) {
  if (attempt > 100) throw new Error("preview page did not expose __setTime");
  await sleep(100);
}
await sleep(1500);

const frames = Math.round((duration / 1000) * fps);
for (let index = 0; index < frames; index += 1) {
  await evaluate(`window.__setTime(${(index * 1000) / fps})`);
  const { data } = await send("Page.captureScreenshot", { format: "png" });
  await writeFile(
    join(options.out, "frames", `frame-${String(index).padStart(5, "0")}.png`),
    Buffer.from(data, "base64"),
  );
  if (index % 60 === 0) console.log(`frame ${index}/${frames}`);
}
console.log(`done: ${frames} frames`);
sessionId = undefined;
await send("Browser.close").catch(() => undefined);
browser.kill();
