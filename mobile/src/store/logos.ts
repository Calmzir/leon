// The logos of the projects of each computer: asked for when a project comes
// on screen, kept on this device so they are there at once the next time and
// when the computer cannot be reached.
//
// One record per project in the key-value store, apart from the saved state
// (an image is up to 256 KiB and would be rewritten with it on every change).
// "Nothing found" is kept too, so a project without a logo is not scanned on
// every visit; a pull to refresh asks again for what is on screen.
import Storage from 'expo-sqlite/kv-store';
import { create } from 'zustand';

import type { ProjectLogo } from '@/link/types';

import { readJson, remove, writeJson } from './storage';

type Saved = { logo: ProjectLogo; at: number };

type LogoState = {
  /** By `key(hostId, root)`: what is known, cached or just found. */
  logos: Record<string, ProjectLogo>;
  /** Bumped when everything on screen should be asked for again. */
  epoch: number;
};

export const useLogos = create<LogoState>(() => ({ logos: {}, epoch: 0 }));

/** How many scans run at once: each is a command on the other computer. */
const AT_ONCE = 2;
/** How long "nothing found" and an avatar stand before the project is scanned again. */
const RECHECK_MS = 24 * 60 * 60 * 1000;
const PREFIX = 'leon.logo.v1.';

export const logoKey = (hostId: string, root: string) => `${hostId}\n${root}`;
const savedKey = (key: string) => PREFIX + key;

type Fetch = (hostId: string, root: string) => Promise<ProjectLogo> | undefined;

const loaded = new Set<string>();
const stale = new Set<string>();
const waiting: { hostId: string; root: string }[] = [];
const asked = new Set<string>();
let running = 0;

function pump(fetch: Fetch): void {
  while (running < AT_ONCE) {
    const next = waiting.shift();
    if (!next) return;
    const key = logoKey(next.hostId, next.root);
    const answer = fetch(next.hostId, next.root);
    if (!answer) {
      // Not reachable now: asked again when the row is next drawn online.
      asked.delete(key);
      continue;
    }
    running += 1;
    answer
      .then((logo) => {
        stale.delete(key);
        writeJson(savedKey(key), { logo, at: Date.now() } satisfies Saved);
        useLogos.setState((s) => ({ logos: { ...s.logos, [key]: logo } }));
      })
      // A failed scan leaves what was known; the next visit tries again.
      .catch(() => undefined)
      .finally(() => {
        running -= 1;
        asked.delete(key);
        pump(fetch);
      });
  }
}

/**
 * Says a project's logo is on screen: reads it from this device the first
 * time, and asks the other computer when nothing is known or what is known
 * is due for another look. Never waits; the answer arrives in the store.
 */
export function wantLogo(hostId: string, root: string, fetch: Fetch): void {
  if (!root) return;
  const key = logoKey(hostId, root);
  if (!loaded.has(key)) {
    loaded.add(key);
    const saved = readJson<Saved>(savedKey(key));
    if (saved) {
      useLogos.setState((s) => ({ logos: { ...s.logos, [key]: saved.logo } }));
      const settled = saved.logo.kind === 'image' || saved.logo.kind === 'svg';
      if (!settled && Date.now() - saved.at > RECHECK_MS) stale.add(key);
    } else {
      stale.add(key);
    }
  }
  if (!stale.has(key) || asked.has(key)) return;
  asked.add(key);
  waiting.push({ hostId, root });
  pump(fetch);
}

/** Asks again for every logo of a computer that is drawn from now on. */
export function recheckLogos(hostId: string): void {
  for (const key of loaded) if (key.startsWith(`${hostId}\n`)) stale.add(key);
  useLogos.setState((s) => ({ epoch: s.epoch + 1 }));
}

/** Forgets the logos of a computer, here and in the store. */
export function forgetLogos(hostId: string): void {
  const mine = (key: string) => key.startsWith(`${hostId}\n`);
  for (const key of [...loaded]) if (mine(key)) loaded.delete(key);
  for (const key of [...stale]) if (mine(key)) stale.delete(key);
  try {
    for (const key of Storage.getAllKeysSync()) {
      if (key.startsWith(`${PREFIX}${hostId}\n`)) remove(key);
    }
  } catch {
    // Nothing to forget, or the store cannot list: the records are harmless.
  }
  useLogos.setState((s) => ({
    logos: Object.fromEntries(Object.entries(s.logos).filter(([key]) => !mine(key))),
  }));
}
