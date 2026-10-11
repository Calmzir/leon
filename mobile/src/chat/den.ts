// The Den on a phone: the lions of every paired computer in one list, the
// ones that wait for the person first. Pure: what the computers said goes
// in, what the screen shows comes out.
import type { DenCub } from '@/link/types';

import { sessionChatId } from './list';

/** How a lion is doing, as far as the person has to care. */
export type Mood = 'waiting' | 'working' | 'quiet' | 'down';

export type Lion = {
  key: string;
  hostId: string;
  hostName: string;
  cub: DenCub;
  mood: Mood;
  /** The state in words: `Editing`, `Needs permission`. */
  label: string;
  /** The sub-agents it sent out that are still in the den. */
  kids: DenCub[];
  /** The chat that opens it, when its session is known. */
  chatKey?: string;
};

const LABELS: Record<string, string> = {
  editing: 'Editing',
  reading: 'Reading',
  searching: 'Searching',
  running: 'Running',
  web: 'On the web',
  delegating: 'Delegating',
  planning: 'Planning',
  'using-tool': 'Using a tool',
  thinking: 'Thinking',
  mystery: 'Working',
  'waiting-for-user': 'Waiting for you',
  'needs-permission': 'Needs permission',
  idle: 'Idle',
  asleep: 'Asleep',
  fainted: 'Stopped with an error',
  gone: 'Gone',
};

const QUIET = new Set(['idle', 'asleep', 'gone']);
const WAITING = new Set(['waiting-for-user', 'needs-permission']);

/** What a state means for the person. A state not known here is some kind of working. */
export function moodOf(state: string): Mood {
  if (WAITING.has(state)) return 'waiting';
  if (state === 'fainted') return 'down';
  return QUIET.has(state) ? 'quiet' : 'working';
}

export function labelOf(state: string): string {
  return LABELS[state] ?? 'Working';
}

const ORDER: Record<Mood, number> = { waiting: 0, down: 1, working: 2, quiet: 3 };

/**
 * The lions of the computers, sub-agents under the lion that sent them: who
 * waits first, then who stopped with an error, who works, who is quiet;
 * computers and lions otherwise in the order they came. A lion that left
 * (`gone`) is not listed.
 */
export function lionsOf(
  hosts: { hostId: string; hostName: string }[],
  dens: Record<string, DenCub[] | undefined>,
): Lion[] {
  const lions: Lion[] = [];
  for (const { hostId, hostName } of hosts) {
    const cubs = (dens[hostId] ?? []).filter((cub) => cub.state !== 'gone');
    for (const cub of cubs) {
      if (cub.parent !== undefined && cubs.some((other) => other.id === cub.parent)) continue;
      lions.push({
        key: `${hostId}/${cub.id}`,
        hostId,
        hostName,
        cub,
        mood: moodOf(cub.state),
        label: labelOf(cub.state),
        kids: cubs.filter((other) => other.parent === cub.id),
        chatKey:
          cub.agent && cub.externalId ? sessionChatId(hostId, cub.agent, cub.externalId) : undefined,
      });
    }
  }
  return lions
    .map((lion, at) => ({ lion, at }))
    .sort((a, b) => ORDER[a.lion.mood] - ORDER[b.lion.mood] || a.at - b.at)
    .map(({ lion }) => lion);
}

/** The count line of the den: how many are in it and how many of them work or wait. */
export function tally(lions: Lion[]): { live: number; working: number; waiting: number } {
  return {
    live: lions.length,
    working: lions.filter((lion) => lion.mood === 'working').length,
    waiting: lions.filter((lion) => lion.mood === 'waiting').length,
  };
}

/** Which frames of a lion's strip show a state, in order; one frame stands still. */
export function framesOf(state: string): number[] {
  switch (state) {
    case 'reading':
    case 'searching':
      return [5, 6];
    case 'thinking':
    case 'mystery':
    case 'planning':
      return [0, 0, 0, 7];
    case 'waiting-for-user':
    case 'needs-permission':
    case 'idle':
      return [0];
    case 'asleep':
      return [7];
    case 'fainted':
      return [9];
    default:
      return QUIET.has(state) ? [0] : [3, 4];
  }
}

/** A number from a lion's name and id that stays the same: which body and style it wears. */
export function seedOf(cub: DenCub): number {
  let hash = 2166136261;
  for (const ch of `${cub.name}/${cub.id}`) {
    hash ^= ch.codePointAt(0) ?? 0;
    hash = Math.imul(hash, 16777619);
  }
  return hash >>> 0;
}
