// Where the application keeps what must survive a restart: expo-sqlite's
// key-value store. It is synchronous, so the saved state is there before the
// first screen draws, and it is Expo's own module (one native dependency
// fewer than MMKV, which needs Nitro Modules).
import Storage from 'expo-sqlite/kv-store';
import { createJSONStorage } from 'zustand/middleware';

export const persistStorage = createJSONStorage(() => ({
  getItem: (key: string) => Storage.getItemSync(key),
  setItem: (key: string, value: string) => Storage.setItemSync(key, value),
  removeItem: (key: string) => {
    Storage.removeItemSync(key);
  },
}));

export function readJson<T>(key: string): T | undefined {
  try {
    const raw = Storage.getItemSync(key);
    return raw === null ? undefined : (JSON.parse(raw) as T);
  } catch {
    return undefined;
  }
}

export function writeJson(key: string, value: unknown): void {
  Storage.setItemSync(key, JSON.stringify(value));
}

export function remove(key: string): void {
  Storage.removeItemSync(key);
}
