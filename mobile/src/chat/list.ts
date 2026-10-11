// The chats of one computer as the tree its Leon shows in the sidebar: the
// projects it shares, each with its git worktrees and the sessions that ran
// in them, and one "unsorted" bucket of folders for everything else. What
// the computer shares is merged with the chats started on this device.
//
// The placement rule is the desktop's (`crates/app/src/ui/tree.rs`): walking
// up from a session's folder, the first project root or worktree met owns it,
// so the deepest one wins; a session no project contains is unsorted, under
// its folder.
import type { HistorySession, Project, TurnOptions, Worktree } from '@/link/types';

/** A chat this device knows: started here, or a shared session opened here. */
export type ChatRecord = {
  id: string;
  hostId: string;
  agent: string;
  cwd?: string;
  title: string;
  /** The agent's conversation to continue; set by the first turn's answer. */
  resumeId?: string;
  /** Every conversation id this chat has had: what hides its twin in the shared history. */
  sessionIds: string[];
  /** The shared session this chat continues, when it began on the other computer. */
  externalId?: string;
  /** How its turns are run: the agent's mode, model and effort. */
  options?: TurnOptions;
  createdAt: number;
  updatedAt: number;
  /**
   * The turn in flight: its terminal over there and the id its rows derive
   * from. `waiting`: the agent asked something and nobody answered yet.
   */
  activeTurn?: { pty?: string; turnId: string; waiting?: boolean };
};

export type ChatRow = {
  key: string;
  /** Set when this device has a record of the chat. */
  chatId?: string;
  /** Set when the row comes from the shared history. */
  session?: HistorySession;
  agent: string;
  cwd: string;
  title: string;
  updatedAt: number;
  canContinue: boolean;
  running: boolean;
  /** Its agent asked something and waits for the answer. */
  waiting: boolean;
};

/** The git worktrees of each shared project, by the project's root as shared. */
export type Worktrees = Record<string, Worktree[]>;

/** One place of a project where sessions work: a git worktree, or its root. */
export type Place = {
  key: string;
  path: string;
  /**
   * What it is called: its branch, or its folder when the head is detached.
   * Absent for the project's own folder when git listed no worktree there.
   */
  label?: string;
  /** Whether it is the repository's main worktree. */
  main: boolean;
  /** Whether git listed it. */
  git: boolean;
  /** Newest first. */
  rows: ChatRow[];
};

/** A project the computer shares. */
export type ProjectNode = {
  key: string;
  title: string;
  path: string;
  /**
   * The main worktree, then the places with sessions by their newest one,
   * then the worktrees nothing ran in, as git lists them.
   */
  places: Place[];
  /** How many sessions it holds. */
  count: number;
  /** How many worktrees git lists for it. */
  trees: number;
  /** When its newest session was last touched; 0 without any. */
  latest: number;
  running: boolean;
  /** One of its sessions asked something and waits for the answer. */
  waiting: boolean;
};

/** A folder of sessions no project contains. */
export type FolderNode = { key: string; title: string; path: string; rows: ChatRow[] };

export type ChatTree = {
  /** In the order the computer shares them, which is its sidebar's. */
  projects: ProjectNode[];
  /** The folder of the newest session first. */
  unsorted: FolderNode[];
  unsortedCount: number;
};

/** The id of the record that continues a shared session. */
export function sessionChatId(hostId: string, agent: string, externalId: string): string {
  return `${hostId}/${agent}/${externalId}`;
}

function trimmed(path: string): string {
  return path.length > 1 ? path.replace(/[\\/]+$/, '') : path;
}

function baseName(path: string): string {
  const parts = path.split(/[\\/]/).filter(Boolean);
  return parts[parts.length - 1] ?? path;
}

/** A folder and the folders above it, nearest first. */
function ancestors(path: string): string[] {
  const out: string[] = [];
  let at = trimmed(path);
  while (at) {
    out.push(at);
    const cut = Math.max(at.lastIndexOf('/'), at.lastIndexOf('\\'));
    if (cut < 0) break;
    const up = cut === 0 ? at.slice(0, 1) : at.slice(0, cut);
    if (up === at) break;
    at = up;
  }
  return out;
}

function rowsOf(hostId: string, chats: ChatRecord[], sessions: HistorySession[]): ChatRow[] {
  const rows: ChatRow[] = [];
  const taken = new Set<string>();
  for (const chat of chats) {
    if (chat.hostId !== hostId) continue;
    for (const id of chat.sessionIds) taken.add(`${chat.agent}/${id}`);
    if (chat.externalId) taken.add(`${chat.agent}/${chat.externalId}`);
    rows.push({
      key: chat.id,
      chatId: chat.id,
      agent: chat.agent,
      cwd: chat.cwd ?? '',
      title: chat.title,
      updatedAt: chat.updatedAt,
      canContinue: true,
      running: chat.activeTurn !== undefined,
      waiting: chat.activeTurn?.waiting === true,
    });
  }
  for (const session of sessions) {
    if (taken.has(`${session.agent}/${session.externalId}`)) continue;
    rows.push({
      key: sessionChatId(hostId, session.agent, session.externalId),
      session,
      agent: session.agent,
      cwd: session.cwd,
      title: session.title || 'Untitled',
      updatedAt: session.updatedMs,
      canContinue: session.canResume,
      running: false,
      waiting: false,
    });
  }
  return rows.sort((a, b) => b.updatedAt - a.updatedAt);
}

/**
 * The chats of a computer as its sidebar's tree. Only shared projects are
 * projects: a session belongs to one when its folder is inside the project's
 * root or inside a worktree git lists for the project; everything else is
 * unsorted. Until `worktrees` is known for a project, the sessions of its
 * worktrees outside its root are unsorted too.
 */
export function buildChatTree(
  hostId: string,
  chats: ChatRecord[],
  sessions: HistorySession[],
  projects: Project[],
  worktrees: Worktrees = {},
): ChatTree {
  type Owner = { project: ProjectNode; place: Place };
  const owners = new Map<string, Owner>();
  const nodes: ProjectNode[] = [];
  const seen = new Set<string>();

  // Roots first, so that a worktree at the very same path takes over.
  for (const project of projects) {
    const root = trimmed(project.root);
    if (!root || seen.has(root)) continue;
    seen.add(root);
    const node: ProjectNode = {
      key: root,
      title: project.name || baseName(root),
      path: root,
      places: [],
      count: 0,
      trees: 0,
      latest: 0,
      running: false,
      waiting: false,
    };
    nodes.push(node);
    owners.set(root, {
      project: node,
      place: { key: root, path: root, main: false, git: false, rows: [] },
    });
  }
  for (const node of nodes) {
    const listed = worktrees[node.path] ?? worktrees[`${node.path}/`] ?? [];
    for (const tree of listed) {
      const path = trimmed(tree.path);
      if (!path || node.places.some((place) => place.key === path)) continue;
      const place: Place = {
        key: path,
        path,
        label: tree.branch || baseName(path),
        main: tree.main,
        git: true,
        rows: [],
      };
      node.places.push(place);
      owners.set(path, { project: node, place });
    }
    node.trees = node.places.length;
  }

  const folders = new Map<string, FolderNode>();
  for (const row of rowsOf(hostId, chats, sessions)) {
    const cwd = trimmed(row.cwd);
    let owner: Owner | undefined;
    for (const folder of ancestors(cwd)) {
      owner = owners.get(folder);
      if (owner) break;
    }
    if (owner) {
      const { project, place } = owner;
      // The project's own folder, when no worktree is listed there.
      if (!place.git && !project.places.includes(place)) project.places.push(place);
      place.rows.push(row);
      project.count += 1;
      project.latest = Math.max(project.latest, row.updatedAt);
      project.running ||= row.running;
      project.waiting ||= row.waiting;
      continue;
    }
    const key = cwd || '~';
    let folder = folders.get(key);
    if (!folder) {
      folder = { key, title: cwd ? baseName(cwd) : 'Default folder', path: cwd, rows: [] };
      folders.set(key, folder);
    }
    folder.rows.push(row);
  }

  for (const node of nodes) {
    const listed = new Map(node.places.map((place, at) => [place, at]));
    const rank = (place: Place) => (place.main ? 0 : !place.git ? 1 : place.rows.length > 0 ? 2 : 3);
    node.places.sort((a, b) => {
      if (rank(a) !== rank(b)) return rank(a) - rank(b);
      if (rank(a) === 2) return (b.rows[0]?.updatedAt ?? 0) - (a.rows[0]?.updatedAt ?? 0);
      return (listed.get(a) ?? 0) - (listed.get(b) ?? 0);
    });
  }

  const unsorted = [...folders.values()];
  return {
    projects: nodes,
    unsorted,
    unsortedCount: unsorted.reduce((sum, folder) => sum + folder.rows.length, 0),
  };
}

/** A session with where it lives, for a list that is not grouped by place. */
export type FoundRow = { row: ChatRow; where: string };

/** The words of a search, lower-cased; empty when nothing is searched. */
export function searchWords(query: string): string[] {
  return query.toLowerCase().split(/\s+/).filter(Boolean);
}

function found(rows: FoundRow[], words: string[]): FoundRow[] {
  const kept =
    words.length === 0
      ? rows
      : rows.filter(({ row, where }) => {
          const text = `${row.title} ${row.agent} ${where}`.toLowerCase();
          return words.every((word) => text.includes(word));
        });
  return kept.sort((a, b) => b.row.updatedAt - a.row.updatedAt);
}

/** The sessions of a project, newest first whatever their worktree; only those every word is found in. */
export function projectRows(project: ProjectNode, words: string[] = []): FoundRow[] {
  return found(
    project.places.flatMap((place) => place.rows.map((row) => ({ row, where: place.label ?? '' }))),
    words,
  );
}

/** Every session of a computer, newest first, said with its project (and worktree) or its folder. */
export function allRows(tree: ChatTree, words: string[] = []): FoundRow[] {
  const inProjects = tree.projects.flatMap((project) =>
    project.places.flatMap((place) =>
      place.rows.map((row) => ({
        row,
        where: place.label && !place.main ? `${project.title} / ${place.label}` : project.title,
      })),
    ),
  );
  const outside = tree.unsorted.flatMap((folder) => folder.rows.map((row) => ({ row, where: folder.title })));
  return found([...inProjects, ...outside], words);
}
