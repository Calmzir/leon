// Going on hearing the agents behind another application (Android): while a
// turn of this phone is running, a foreground service keeps the process, and
// with it the link to the computer. It has to be started while the
// application is in front, which is when a turn is: the person just sent it.
import { AppState } from 'react-native';
import * as Background from 'leon-background';

import { isWaiting } from '@/chat/reduce';

import { useApp } from './app';
import { link, useRuntime } from './runtime';

/** The address a notification opens. */
export function chatUrl(chatId: string): string {
  return `leon://chat/${encodeURIComponent(chatId)}`;
}

let said = '';
let asked = false;

/** What the quiet notification says of the turns still running. */
function summary(): { title: string; text: string; url?: string } | null {
  const { turns } = useRuntime.getState();
  const chats = useApp.getState().chats;
  const live = Object.entries(turns).filter(([id, turn]) => turn.status === 'running' && chats[id]);
  if (live.length === 0) return null;
  const waiting = live.filter(([, turn]) => isWaiting(turn));
  const shown = waiting.length > 0 ? waiting : live;
  const names = shown.map(([id]) => chats[id]?.title).filter(Boolean);
  const text = names.slice(0, 3).join(' · ');
  if (shown.length > 1) {
    const title = waiting.length > 0 ? `${shown.length} agents are waiting for you` : `${shown.length} agents working`;
    return { title, text };
  }
  const [chatId] = shown[0]!;
  const agent = chats[chatId]?.agent;
  const name = link().agents().find((a) => a.id === agent)?.name ?? agent ?? 'Agent';
  const title = waiting.length > 0 ? `${name} is waiting for you` : `${name} is working`;
  return { title, text, url: chatUrl(chatId) };
}

function sync(): void {
  const next = summary();
  const key = next ? `${next.title}\n${next.text}\n${next.url ?? ''}` : '';
  if (key === said) return;
  if (!next) {
    said = key;
    Background.release();
    return;
  }
  // Refused when asked from behind another application: tried again at the next change.
  if (Background.hold(next.title, next.text, next.url)) said = key;
  // Asked once, at the first turn, when the person can see why.
  if (!asked && AppState.currentState === 'active' && !Background.canNotify()) {
    asked = true;
    void Background.askToNotify();
  }
}

/** Follows the turns from now on. Returns what stops it. */
export function keepHearing(): () => void {
  if (!Background.supported) return () => undefined;
  sync();
  const turns = useRuntime.subscribe((state, before) => {
    if (state.turns !== before.turns) sync();
  });
  // A service refused from behind is started on the way back.
  const front = AppState.addEventListener('change', (state) => {
    if (state === 'active') sync();
  });
  return () => {
    turns();
    front.remove();
    said = '';
    Background.release();
  };
}
