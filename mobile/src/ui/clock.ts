// One coarse clock for everything that says how long ago: a single timer,
// whatever the number of rows reading it, stopped when nothing does.
import { useSyncExternalStore } from 'react';

const EVERY_MS = 10_000;

let now = Date.now();
let timer: ReturnType<typeof setInterval> | undefined;
const listeners = new Set<() => void>();

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  if (!timer) {
    now = Date.now();
    timer = setInterval(() => {
      now = Date.now();
      for (const each of listeners) each();
    }, EVERY_MS);
  }
  return () => {
    listeners.delete(listener);
    if (listeners.size === 0 && timer) {
      clearInterval(timer);
      timer = undefined;
    }
  };
}

/** The time, to within ten seconds; a component that reads it is drawn again as it passes. */
export function useClock(): number {
  return useSyncExternalStore(subscribe, () => now, () => now);
}
