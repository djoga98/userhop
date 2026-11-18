import { describe, expect, it } from "vitest";

import { AVATAR_TONES, avatarTone, initials } from "./avatar";

describe("initials", () => {
  it("uses first and last word, falling back to the user name", () => {
    expect(initials("Ana Marija Kovač", "ana")).toBe("AK");
    expect(initials("", "zoran.p")).toBe("ZP");
    expect(initials("  ", "")).toBe("?");
    expect(initials("élodie", "e")).toBe("É");
  });

  it("assigns a stable tone per account id", () => {
    expect(avatarTone("t:1")).toBe(avatarTone("t:1"));
    expect(avatarTone("win:S-1-5-21-1")).toBeLessThan(AVATAR_TONES);
  });
});
