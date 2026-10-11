// The Android half of telling the person while the application is behind
// another: a foreground service that keeps the process (and so the link)
// alive, and the system notifications. Absent on iOS, where every call is a
// no-op: there the relay has to push.
import { requireOptionalNativeModule } from 'expo';

type Native = {
  hold(title: string, text: string, url: string | null): boolean;
  release(): void;
  alert(tag: string, title: string, text: string, url: string | null): void;
  dismiss(tag: string): void;
  canNotify(): boolean;
  askToNotify(): Promise<{ granted?: boolean }>;
};

const native = requireOptionalNativeModule<Native>('LeonBackground');

/** Whether this build can go on hearing agents behind another application. */
export const supported = native !== null;

/** Keeps the link alive, with a quiet notification saying so. Whether it is held. */
export function hold(title: string, text: string, url?: string): boolean {
  try {
    return native?.hold(title, text, url ?? null) ?? false;
  } catch {
    return false;
  }
}

export function release(): void {
  try {
    native?.release();
  } catch {
    // Nothing was held.
  }
}

/** A notification that calls the person; `tag` replaces the one before it. */
export function alert(tag: string, title: string, text: string, url?: string): void {
  try {
    native?.alert(tag, title, text, url ?? null);
  } catch {
    // Said again in the application when it comes back.
  }
}

export function dismiss(tag: string): void {
  try {
    native?.dismiss(tag);
  } catch {
    // It was not there.
  }
}

export function canNotify(): boolean {
  try {
    return native?.canNotify() ?? false;
  } catch {
    return false;
  }
}

export async function askToNotify(): Promise<boolean> {
  if (!native) return false;
  try {
    return (await native.askToNotify()).granted === true;
  } catch {
    return false;
  }
}
