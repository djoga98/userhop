let height = 300;
const listeners = new Set<() => void>();

export const heightStore = {
  set: (next: number) => {
    height = next;
    for (const listener of listeners) listener();
  },
  get: () => height,
  subscribe: (listener: () => void) => {
    listeners.add(listener);
    return () => {
      listeners.delete(listener);
    };
  },
};
