import "@testing-library/jest-dom/vitest";
import { clearMocks } from "@tauri-apps/api/mocks";
import { cleanup } from "@testing-library/react";
import { afterEach, vi } from "vitest";

const mediaState = new Map<string, boolean>();

export function setMedia(query: string, matches: boolean) {
  mediaState.set(query, matches);
}

Object.defineProperty(window, "matchMedia", {
  configurable: true,
  value: (query: string) => ({
    matches: mediaState.get(query) ?? false,
    media: query,
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
  }),
});

afterEach(async () => {
  cleanup();
  await new Promise((resolve) => setTimeout(resolve, 0));
  clearMocks();
  mediaState.clear();
});
