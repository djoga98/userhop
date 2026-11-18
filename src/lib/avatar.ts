export function initials(displayName: string, userName: string): string {
  const source = displayName.trim() || userName.trim();
  const words = source.split(/[\s._-]+/u).filter(Boolean);
  const letters =
    words.length >= 2
      ? [words[0], words[words.length - 1]].map((w) => Array.from(w ?? "")[0] ?? "")
      : Array.from(words[0] ?? "?").slice(0, 1);
  return letters.join("").toLocaleUpperCase() || "?";
}

export const AVATAR_TONES = 8;

export function avatarTone(id: string): number {
  let hash = 0;
  for (const char of id) hash = (hash * 31 + (char.codePointAt(0) ?? 0)) >>> 0;
  return hash % AVATAR_TONES;
}
