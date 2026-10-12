import { describe, expect, test } from 'bun:test';

import type { HistorySession } from '@/link/types';

import {
  allRows,
  buildChatTree,
  pinnedRows,
  projectRows,
  searchWords,
  sessionChatId,
  type ChatRecord,
  type Worktrees,
} from './list';

function session(over: Partial<HistorySession>): HistorySession {
  return {
    agent: 'claude',
    externalId: 'e1',
    cwd: '/home/me/code/leon',
    title: 'Fix the build',
    startedMs: 1,
    updatedMs: 10,
    messages: 4,
    canResume: true,
    ...over,
  };
}

function chat(over: Partial<ChatRecord>): ChatRecord {
  return {
    id: 'c1',
    hostId: 'H',
    agent: 'claude',
    cwd: '/home/me/code/leon',
    title: 'From the phone',
    sessionIds: [],
    createdAt: 1,
    updatedAt: 20,
    ...over,
  };
}

const LEON = '/home/me/code/leon';
const PROJECTS = [
  { name: 'Leon', root: `${LEON}/` },
  { name: 'api', root: '/srv/api' },
];
const TREES: Worktrees = {
  [LEON]: [
    { path: LEON, branch: 'main', main: true },
    { path: '/home/me/trees/fix-build', branch: 'feat/fix-build', main: false },
    { path: '/home/me/trees/review', main: false },
    { path: '/home/me/trees/idle', branch: 'idle', main: false },
  ],
  '/srv/api': [{ path: '/srv/api', branch: 'trunk', main: true }],
};

describe('buildChatTree', () => {
  test('only shared projects are projects, in the order they are shared', () => {
    const tree = buildChatTree(
      'H',
      [],
      [
        session({ externalId: 'a', updatedMs: 5, cwd: '/srv/api' }),
        session({ externalId: 'b', updatedMs: 90, cwd: '/home/me/Downloads' }),
        session({ externalId: 'c', updatedMs: 50 }),
      ],
      PROJECTS,
      TREES,
    );
    // Not by recency: the computer's own order.
    expect(tree.projects.map((p) => [p.title, p.path, p.count])).toEqual([
      ['Leon', LEON, 1],
      ['api', '/srv/api', 1],
    ]);
    expect(tree.unsorted.map((f) => f.title)).toEqual(['Downloads']);
    expect(tree.unsortedCount).toBe(1);
  });

  test('a session belongs to the project whose root or worktree holds its folder', () => {
    const tree = buildChatTree(
      'H',
      [chat({ cwd: '/home/me/trees/fix-build/docs', updatedAt: 50 })],
      [
        session({ externalId: 'a', updatedMs: 30, cwd: `${LEON}/crates/app` }),
        session({ externalId: 'b', updatedMs: 40, cwd: '/home/me/trees/fix-build' }),
        session({ externalId: 'c', updatedMs: 20, cwd: '/home/me/trees/review' }),
      ],
      PROJECTS,
      TREES,
    );
    const leon = tree.projects[0]!;
    expect(leon.count).toBe(4);
    expect(leon.trees).toBe(4);
    expect(leon.latest).toBe(50);
    expect(tree.unsortedCount).toBe(0);
    expect(leon.places.map((p) => [p.label, p.main, p.rows.map((r) => r.updatedAt)])).toEqual([
      ['main', true, [30]],
      ['feat/fix-build', false, [50, 40]],
      // A detached head is called by its folder.
      ['review', false, [20]],
      ['idle', false, []],
    ]);
  });

  test('the main worktree is first, then worktrees by their newest session, then the empty ones', () => {
    const tree = buildChatTree(
      'H',
      [],
      [
        session({ externalId: 'a', updatedMs: 10, cwd: '/home/me/trees/fix-build' }),
        session({ externalId: 'b', updatedMs: 70, cwd: '/home/me/trees/review' }),
      ],
      PROJECTS,
      TREES,
    );
    expect(tree.projects[0]!.places.map((p) => p.label)).toEqual(['main', 'review', 'feat/fix-build', 'idle']);
  });

  test('an unrelated repository and a look-alike folder are unsorted, by folder, newest folder first', () => {
    const tree = buildChatTree(
      'H',
      [],
      [
        session({ externalId: 'a', updatedMs: 10, cwd: '/home/me/code/leon-site' }),
        session({ externalId: 'b', updatedMs: 30, cwd: '/home/me/other-repo/src' }),
        session({ externalId: 'c', updatedMs: 20, cwd: '/home/me/code/leon-site/' }),
        session({ externalId: 'd', updatedMs: 5, cwd: '/home/me/other-repo' }),
      ],
      PROJECTS,
      TREES,
    );
    expect(tree.projects.every((p) => p.count === 0)).toBe(true);
    expect(tree.unsorted.map((f) => [f.title, f.path, f.rows.map((r) => r.updatedAt)])).toEqual([
      ['src', '/home/me/other-repo/src', [30]],
      ['leon-site', '/home/me/code/leon-site', [20, 10]],
      ['other-repo', '/home/me/other-repo', [5]],
    ]);
    expect(tree.unsortedCount).toBe(4);
  });

  test('a computer that shares no projects has everything unsorted', () => {
    const tree = buildChatTree(
      'H',
      [chat({ cwd: undefined, updatedAt: 99 })],
      [session({ externalId: 'a' }), session({ externalId: 'b', cwd: '/tmp' })],
      [],
    );
    expect(tree.projects).toEqual([]);
    expect(tree.unsorted.map((f) => f.title)).toEqual(['Default folder', 'leon', 'tmp']);
    expect(tree.unsortedCount).toBe(3);
  });

  test('before its worktrees are known, a project holds its own folder and the rest is unsorted', () => {
    const tree = buildChatTree(
      'H',
      [],
      [
        session({ externalId: 'a', updatedMs: 30, cwd: `${LEON}/crates/app` }),
        session({ externalId: 'b', updatedMs: 40, cwd: '/home/me/trees/fix-build' }),
      ],
      PROJECTS,
    );
    const leon = tree.projects[0]!;
    expect(leon.trees).toBe(0);
    expect(leon.places.map((p) => [p.label, p.git, p.path, p.rows.length])).toEqual([[undefined, false, LEON, 1]]);
    expect(tree.unsorted.map((f) => f.title)).toEqual(['fix-build']);
  });

  test('the deepest owner wins: a worktree inside the root, and a project inside another', () => {
    const tree = buildChatTree(
      'H',
      [],
      [
        session({ externalId: 'a', cwd: `${LEON}/.trees/exp/src` }),
        session({ externalId: 'b', cwd: `${LEON}/web/app` }),
      ],
      [...PROJECTS, { name: 'web', root: `${LEON}/web` }],
      { ...TREES, [LEON]: [...TREES[LEON]!, { path: `${LEON}/.trees/exp`, branch: 'exp', main: false }] },
    );
    expect(tree.projects[0]!.places.find((p) => p.label === 'exp')!.rows).toHaveLength(1);
    expect(tree.projects[0]!.count).toBe(1);
    expect(tree.projects[2]!.count).toBe(1);
  });

  test('a shared session this device already continues appears once', () => {
    const tree = buildChatTree(
      'H',
      [chat({ id: sessionChatId('H', 'claude', 'e1'), externalId: 'e1', sessionIds: ['e2'] })],
      [session({ externalId: 'e1' }), session({ externalId: 'e2' }), session({ externalId: 'e3' })],
      PROJECTS,
      TREES,
    );
    expect(tree.projects[0]!.places[0]!.rows.map((r) => r.key)).toEqual(['H/claude/e1', 'H/claude/e3']);
  });

  test('chats of other computers are left out, and running ones mark their project', () => {
    const tree = buildChatTree(
      'H',
      [chat({ id: 'x', hostId: 'OTHER' }), chat({ activeTurn: { turnId: 't' } })],
      [session({ canResume: false })],
      PROJECTS,
      TREES,
    );
    const leon = tree.projects[0]!;
    expect(leon.running).toBe(true);
    expect(leon.places[0]!.rows.map((r) => [r.key, r.running, r.canContinue])).toEqual([
      ['c1', true, true],
      ['H/claude/e1', false, false],
    ]);
    expect(tree.projects[1]!.running).toBe(false);
  });
});

describe('the newest first, and search', () => {
  const tree = buildChatTree(
    'H',
    [chat({ cwd: '/home/me/trees/fix-build', title: 'From the phone', updatedAt: 70 })],
    [
      session({ externalId: 'a', updatedMs: 5, cwd: '/srv/api', title: 'Rate limits' }),
      session({ externalId: 'b', updatedMs: 90, cwd: '/home/me/Downloads', title: 'Sort the invoices' }),
      session({ externalId: 'c', updatedMs: 50, title: 'Fix the build' }),
    ],
    PROJECTS,
    TREES,
  );

  test('every session of the computer, newest first, with where it lives', () => {
    expect(allRows(tree).map(({ row, where }) => [row.title, where])).toEqual([
      ['Sort the invoices', 'Downloads'],
      ['From the phone', 'Leon / feat/fix-build'],
      ['Fix the build', 'Leon'],
      ['Rate limits', 'api'],
    ]);
  });

  test('a search keeps the sessions every word is found in: title, agent or place', () => {
    expect(allRows(tree, searchWords('  FIX  ')).map(({ row }) => row.title)).toEqual([
      'From the phone',
      'Fix the build',
    ]);
    expect(allRows(tree, searchWords('claude api')).map(({ row }) => row.title)).toEqual(['Rate limits']);
    expect(allRows(tree, searchWords('nothing'))).toEqual([]);
  });

  test('the pinned sessions keep the order of the computer, and say what this device knows of them', () => {
    const chats = [
      chat({
        id: 'mine',
        sessionIds: ['c'],
        title: 'Continued here',
        updatedAt: 60,
        activeTurn: { turnId: 't', waiting: true },
      }),
    ];
    const sessions = [
      session({ externalId: 'a', updatedMs: 5, cwd: '/srv/api', title: 'Rate limits' }),
      session({ externalId: 'c', updatedMs: 50, title: 'Fix the build' }),
    ];
    const old = session({ externalId: 'z', updatedMs: 1, cwd: '/home/me/notes/', title: 'An old one' });
    const withMine = buildChatTree('H', chats, sessions, PROJECTS, TREES);
    const rows = pinnedRows('H', chats, [old, sessions[1]!, sessions[0]!], withMine);
    expect(rows.map(({ row, where }) => [row.title, where, row.waiting])).toEqual([
      ['An old one', 'notes', false],
      ['Continued here', 'Leon', true],
      ['Rate limits', 'api', false],
    ]);
    expect(rows[0]!.row.key).toBe(sessionChatId('H', 'claude', 'z'));
    expect(pinnedRows('H', chats, [], withMine)).toEqual([]);
  });

  test('the sessions of a project across its worktrees, newest first', () => {
    const leon = tree.projects[0]!;
    expect(projectRows(leon).map(({ row, where }) => [row.title, where])).toEqual([
      ['From the phone', 'feat/fix-build'],
      ['Fix the build', 'main'],
    ]);
    expect(projectRows(leon, searchWords('main')).map(({ row }) => row.title)).toEqual(['Fix the build']);
  });
});
