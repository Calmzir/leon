import { describe, expect, test } from 'bun:test';

import type { DenCub } from '@/link/types';

import { framesOf, labelOf, lionsOf, moodOf, seedOf, tally } from './den';
import { sessionChatId } from './list';

function cub(over: Partial<DenCub>): DenCub {
  return { id: '1', name: 'fix the build', agent: 'claude', state: 'editing', level: 0, mystery: false, ...over };
}

const HOSTS = [
  { hostId: 'A', hostName: 'home' },
  { hostId: 'B', hostName: 'laptop' },
];

describe('the lions of the den', () => {
  test('who waits comes first, then who failed, who works, who is quiet', () => {
    const lions = lionsOf(HOSTS, {
      A: [
        cub({ id: '1', state: 'editing' }),
        cub({ id: '2', state: 'asleep' }),
        cub({ id: '3', state: 'needs-permission', externalId: 'x' }),
      ],
      B: [cub({ id: '1', state: 'fainted' }), cub({ id: '2', state: 'gone' })],
    });
    expect(lions.map((lion) => [lion.key, lion.mood, lion.label])).toEqual([
      ['A/3', 'waiting', 'Needs permission'],
      ['B/1', 'down', 'Stopped with an error'],
      ['A/1', 'working', 'Editing'],
      ['A/2', 'quiet', 'Asleep'],
    ]);
    expect(lions[0]!.chatKey).toBe(sessionChatId('A', 'claude', 'x'));
    expect(lions[2]!.chatKey).toBeUndefined();
    expect(tally(lions)).toEqual({ live: 4, working: 1, waiting: 1 });
  });

  test('a sub-agent is under the lion that sent it, and alone when that one is not there', () => {
    const lions = lionsOf(HOSTS, {
      A: [
        cub({ id: '1', state: 'delegating' }),
        cub({ id: '2', parent: '1', name: 'explore', state: 'reading' }),
        cub({ id: '9', parent: '7', name: 'orphan', state: 'running' }),
      ],
    });
    expect(lions.map((lion) => [lion.cub.id, lion.kids.map((kid) => kid.name)])).toEqual([
      ['1', ['explore']],
      ['9', []],
    ]);
  });

  test('a computer that said nothing has no lions', () => {
    expect(lionsOf(HOSTS, {})).toEqual([]);
  });
});

test('a state this app does not know is some kind of working', () => {
  expect(moodOf('juggling')).toBe('working');
  expect(labelOf('juggling')).toBe('Working');
  expect(framesOf('juggling')).toEqual([3, 4]);
  expect(framesOf('fainted')).toEqual([9]);
  expect(framesOf('needs-permission')).toEqual([0]);
});

test('a lion wears the same body every time', () => {
  expect(seedOf(cub({}))).toBe(seedOf(cub({})));
  expect(seedOf(cub({ id: '2' }))).not.toBe(seedOf(cub({})));
});
