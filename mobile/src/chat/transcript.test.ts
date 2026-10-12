import { describe, expect, test } from 'bun:test';

import { fromCwd, type Message } from './model';
import {
  activityWord,
  classifyUserText,
  combineRows,
  mergeTranscript,
  nextActivity,
  refreshRows,
  sizeLabel,
  transcriptRows,
  withoutNotes,
} from './transcript';

const user = (id: string, text: string, at: number): Message => ({ id, kind: 'user', text, at });
const reply = (id: string, text: string, at: number): Message => ({
  id,
  kind: 'assistant',
  text,
  at,
  streaming: false,
});

describe('transcriptRows', () => {
  test('maps roles, splits a tool entry, keeps system text as a note and drops empty entries', () => {
    const rows = transcriptRows('c', [
      { role: 'user', text: ' hi ', atMs: 1 },
      { role: 'system', text: 'init', atMs: 2 },
      { role: 'assistant', text: 'hello', atMs: 3 },
      { role: 'tool', text: 'Bash: ls\nfile.txt', atMs: 4 },
      { role: 'assistant', text: '   ', atMs: 5 },
    ]);
    expect(rows.map((r) => [r.id, r.kind])).toEqual([
      ['c:t0', 'user'],
      ['c:t1', 'note'],
      ['c:t2', 'assistant'],
      ['c:t3', 'tool'],
    ]);
    expect(rows[3]).toMatchObject({ name: 'Bash', summary: 'ls', result: 'file.txt', output: 'file.txt', status: 'ok' });
    expect(withoutNotes(rows).map((r) => r.kind)).toEqual(['user', 'assistant', 'tool']);
  });
});

describe('mergeTranscript', () => {
  const transcript = [user('t0', 'first', 100), reply('t1', 'one', 110)];

  test('without a transcript, what this device holds is shown', () => {
    const local = [user('l0', 'q', 5)];
    expect(mergeTranscript([], local)).toBe(local);
  });

  test('a turn the transcript does not have yet stays after it', () => {
    const local = [user('l0', 'second', 200), reply('l1', 'two', 210)];
    expect(mergeTranscript(transcript, local).map((m) => m.id)).toEqual(['t0', 't1', 'l0', 'l1']);
  });

  test('a turn the transcript caught up with is dropped, by time', () => {
    const local = [user('l0', 'first', 90), reply('l1', 'one', 95), user('l2', 'second', 200)];
    expect(mergeTranscript(transcript, local).map((m) => m.id)).toEqual(['t0', 't1', 'l2']);
  });

  test('a turn the transcript has is dropped even when this clock runs ahead', () => {
    const local = [user('l0', 'first', 500), reply('l1', 'one', 510)];
    expect(mergeTranscript(transcript, local).map((m) => m.id)).toEqual(['t0', 't1']);
  });

  test('everything before a known turn goes with it', () => {
    const local = [
      user('l0', 'zero', 400),
      reply('l1', 'z', 410),
      user('l2', 'first', 420),
      user('l3', 'new', 430),
    ];
    expect(mergeTranscript(transcript, local).map((m) => m.id)).toEqual(['t0', 't1', 'l3']);
  });
});

// Shapes taken from real histories of Claude Code, Codex and opencode.
const TASK = `<task-notification>
<task-id>bu57dcfo8</task-id>
<status>completed</status>
<summary>Agent "Build" finished</summary>
<result>All done.</result>
<usage><subagent_tokens>555652</subagent_tokens></usage>
</task-notification>`;

describe('classifyUserText', () => {
  test('what a person typed is all theirs', () => {
    expect(classifyUserText('  fix the login flow  ')).toEqual({ said: 'fix the login flow' });
    // Their own paste and their pictures are part of what they said.
    const pasted = '<pasted_content id="a1">\nstack trace\n</pasted_content> why?';
    expect(classifyUserText(pasted)).toEqual({ said: pasted });
    expect(classifyUserText('[Image #3] esto está mal')).toEqual({ said: '[Image #3] esto está mal' });
    expect(classifyUserText('use a <div> and 1 < 2')).toEqual({ said: 'use a <div> and 1 < 2' });
  });

  test('an entry that is only harness text has nothing said', () => {
    expect(classifyUserText(TASK)).toEqual({
      said: '',
      note: { label: 'task notification', text: TASK },
    });
    expect(classifyUserText('[Request interrupted by user for tool use]')).toEqual({
      said: '',
      note: { label: 'interruption', text: '[Request interrupted by user for tool use]' },
    });
    const ran =
      '<command-name>/model</command-name>\n<command-message>model</command-message>\n<command-args></command-args>';
    expect(classifyUserText(ran)).toMatchObject({ said: '', note: { label: 'command' } });
    expect(
      classifyUserText('<local-command-stdout>Set model to Opus</local-command-stdout>'),
    ).toMatchObject({ said: '', note: { label: 'command output' } });
    expect(
      classifyUserText('<bash-stdout></bash-stdout><bash-stderr>aws: [ERROR]</bash-stderr>'),
    ).toMatchObject({ said: '', note: { label: 'shell output' } });
    expect(
      classifyUserText('<environment_context>\n  <cwd>/srv</cwd>\n</environment_context>'),
    ).toMatchObject({ said: '', note: { label: 'environment' } });
    expect(
      classifyUserText(
        '<external_codex_apps_writing_block_edits_part_2_of_4>x</external_codex_apps_writing_block_edits_part_2_of_4>',
      ),
    ).toMatchObject({ said: '', note: { label: 'editor changes' } });
  });

  test('a mix keeps the words and sets the rest aside, each kind named once', () => {
    const mixed = `please continue\n<system-reminder>\nToday is Tuesday.\n</system-reminder>\n\nand run the tests\n<system-reminder>second</system-reminder>${TASK}`;
    const { said, note } = classifyUserText(mixed);
    expect(said).toBe('please continue\n\nand run the tests');
    expect(note?.label).toBe('system reminder + task notification');
    expect(note?.text).toContain('Today is Tuesday.');
    expect(note?.text).toContain('<task-id>bu57dcfo8</task-id>');
  });

  test('a block that never closes is left where it is, unless the entry begins with it', () => {
    const open = 'look: <system-reminder> was printed';
    expect(classifyUserText(open)).toEqual({ said: open });
    // A long notification clipped by the history: all of it is the harness's.
    const clipped = '<task-notification>\n<task-id>x</task-id>\n<result>a very long rep';
    expect(classifyUserText(clipped)).toEqual({
      said: '',
      note: { label: 'task notification', text: clipped },
    });
  });
});

describe('notes among the rows', () => {
  test('a harness entry is a note; a mixed one is a note and then the words', () => {
    const rows = transcriptRows('c', [
      { role: 'user', text: TASK, atMs: 1 },
      { role: 'user', text: `go on\n<system-reminder>x</system-reminder>`, atMs: 2 },
      { role: 'system', text: 'This session is being continued from a previous conversation.', atMs: 3 },
    ]);
    expect(rows.map((r) => [r.id, r.kind])).toEqual([
      ['c:t0n', 'note'],
      ['c:t1n', 'note'],
      ['c:t1', 'user'],
      ['c:t2', 'note'],
    ]);
    expect(rows[2]).toMatchObject({ text: 'go on' });
    expect(rows[3]).toMatchObject({ label: 'summary of the earlier conversation' });
    expect(withoutNotes(rows).map((r) => r.id)).toEqual(['c:t1']);
  });

  test('a list without notes is returned as it is', () => {
    const rows = [user('a', 'hi', 1)];
    expect(withoutNotes(rows)).toBe(rows);
  });

  test('sizes read short', () => {
    expect(sizeLabel('x'.repeat(840))).toBe('840 chars');
    expect(sizeLabel('x'.repeat(1234))).toBe('1.2k chars');
    expect(sizeLabel('x'.repeat(25400))).toBe('25k chars');
  });
});

describe('refreshRows', () => {
  const first = [user('c:t0', 'hi', 1), reply('c:t1', 'hel', 2)];

  test('a reading that changed nothing is the list that was shown', () => {
    const again = [user('c:t0', 'hi', 1), reply('c:t1', 'hel', 2)];
    expect(refreshRows(first, again)).toBe(first);
    expect(refreshRows(undefined, again)).toBe(again);
  });

  test('new entries are appended and the old rows stay the same objects', () => {
    const more = [user('c:t0', 'hi', 1), reply('c:t1', 'hel', 2), user('c:t2', 'more', 3)];
    const rows = refreshRows(first, more);
    expect(rows).toHaveLength(3);
    expect(rows[0]).toBe(first[0]);
    expect(rows[1]).toBe(first[1]);
    expect(rows[2]).toBe(more[2]);
  });

  test('a tail still being written is replaced, the rest is kept', () => {
    const grown = [user('c:t0', 'hi', 1), reply('c:t1', 'hello there', 2)];
    const rows = refreshRows(first, grown);
    expect(rows[0]).toBe(first[0]);
    expect(rows[1]).toBe(grown[1]);
  });

  test('a transcript imported afresh, shorter or different, is taken as read', () => {
    const shorter = [user('c:t0', 'hi', 1)];
    expect(refreshRows(first, shorter)).toEqual(shorter);
    expect(refreshRows(first, shorter)[0]).toBe(first[0]);
    const other = [user('c:t0', 'something else', 9)];
    expect(refreshRows(first, other)).toEqual(other);
    expect(refreshRows(first, [])).toEqual([]);
  });
});

describe('combineRows', () => {
  const turn = {
    turnId: 't',
    items: [reply('t:1', 'working on it', 50)],
    draftId: 't:1',
    made: 1,
  } as unknown as Parameters<typeof combineRows>[2];

  test('the turn in flight stays last whatever the transcript says meanwhile', () => {
    const local = [user('t:u', 'do it', 40)];
    // The other computer has not imported the turn yet.
    const before = combineRows([user('c:t0', 'old', 1)], local, turn);
    expect(before.map((r) => r.id)).toEqual(['c:t0', 't:u', 't:1']);
    // Now it has the prompt: this device's copy of it goes, the live answer stays.
    const after = combineRows([user('c:t0', 'old', 1), user('c:t1', 'do it', 41)], local, turn);
    expect(after.map((r) => r.id)).toEqual(['c:t0', 'c:t1', 't:1']);
  });

  test('without a transcript the device shows what it holds', () => {
    const local = [user('a', 'hi', 1)];
    expect(combineRows(undefined, local, undefined)).toBe(local);
    expect(combineRows([], local, undefined)).toBe(local);
  });
});

describe('activity', () => {
  test('the first reading asks for the transcript and claims no change', () => {
    const { activity, changed } = nextActivity(undefined, { updatedMs: 100, messages: 3 }, 1000);
    expect(changed).toBe(true);
    expect(activity.changedAt).toBeUndefined();
  });

  test('a change is remembered with this device clock; no change keeps the last one', () => {
    const one = nextActivity(undefined, { updatedMs: 100, messages: 3 }, 1000).activity;
    const same = nextActivity(one, { updatedMs: 100, messages: 3 }, 2000);
    expect(same.changed).toBe(false);
    const moved = nextActivity(same.activity, { updatedMs: 100, messages: 4 }, 3000);
    expect(moved).toEqual({ activity: { updatedMs: 100, messages: 4, changedAt: 3000 }, changed: true });
    const still = nextActivity(moved.activity, { updatedMs: 100, messages: 4 }, 4000);
    expect(still.activity.changedAt).toBe(3000);
  });

  test('it reads live for a while after a change, then as a time ago', () => {
    const now = 10_000_000;
    expect(activityWord({ updatedMs: 1, messages: 1, changedAt: now - 5_000 }, now)).toEqual({
      live: true,
      word: 'changing there',
    });
    expect(activityWord({ updatedMs: now - 42_000, messages: 1, changedAt: now - 200_000 }, now)).toEqual({
      live: false,
      word: 'updated 42 s ago',
    });
    expect(activityWord({ updatedMs: now - 3 * 3600_000, messages: 1 }, now).word).toBe('updated 3 h ago');
    expect(activityWord(undefined, now)).toEqual({ live: false, word: '' });
  });
});

describe('fromCwd', () => {
  const cwd = '/home/v/code/leon';
  test('says the paths inside the folder from it', () => {
    expect(fromCwd('Write: /home/v/code/leon/src/a.ts', cwd)).toBe('Write: src/a.ts');
    expect(fromCwd('ls /home/v/code/leon', cwd)).toBe('ls .');
    expect(fromCwd('cd "/home/v/code/leon" && ls', `${cwd}/`)).toBe('cd "." && ls');
  });
  test('leaves a folder that only begins like it whole', () => {
    const sibling = 'cat /home/v/code/leon-worktrees/x/a.ts /home/v/code/leon-web/b.ts';
    expect(fromCwd(sibling, cwd)).toBe(sibling);
    expect(fromCwd('/mnt/home/v/code/leon/a', cwd)).toBe('/mnt/home/v/code/leon/a');
    expect(fromCwd('/home/v/code/leon.bak/a', cwd)).toBe('/home/v/code/leon.bak/a');
  });
  test('does nothing without a folder', () => {
    expect(fromCwd('a /b', undefined)).toBe('a /b');
    expect(fromCwd('a /b', '/')).toBe('a /b');
  });
});
