// A computer that is not there: what the app shows before anything is paired,
// so that it can be tried (and reviewed) without a Leon to connect to. Its
// projects, sessions and Den are made up, and its agent answers every prompt
// with the same short, scripted turn. Nothing leaves the phone.
import {
  LinkError,
  type AgentOptions,
  type ConnState,
  type DenCub,
  type HistorySession,
  type HostLink,
  type PairedHost,
  type SharedState,
  type TranscriptEntry,
  type TurnEvent,
  type TurnHandle,
} from './types';

export const DEMO_HOST_ID = 'demo';

export const DEMO_HOST: PairedHost = {
  hostId: DEMO_HOST_ID,
  hostKeyHex: '',
  hostName: 'Demo computer',
  relayUrl: 'made up, on this phone',
};

export function isDemo(hostId: string): boolean {
  return hostId === DEMO_HOST_ID;
}

const HOME = '/home/demo';
const SHOP = `${HOME}/code/shop`;
const API = `${HOME}/code/api`;
const MIN = 60_000;

function session(
  agent: string,
  externalId: string,
  cwd: string,
  title: string,
  ago: number,
  messages: number,
): HistorySession {
  const updatedMs = Date.now() - ago * MIN;
  return {
    agent,
    externalId,
    cwd,
    title,
    startedMs: updatedMs - 40 * MIN,
    updatedMs,
    messages,
    canResume: true,
  };
}

function sessions(): HistorySession[] {
  return [
    session('claude', 'demo-checkout', SHOP, 'Fix the rounding of totals at checkout', 2, 14),
    session('codex', 'demo-rate-limit', API, 'Add rate limiting to the public endpoints', 6, 22),
    session('opencode', 'demo-readme', SHOP, 'Update the README with the new setup steps', 35, 8),
    session('claude', 'demo-flaky', API, 'Find why the orders test is flaky', 180, 31),
    session('claude', 'demo-dark', SHOP, 'Dark mode for the product page', 1500, 19),
  ];
}

const TRANSCRIPTS: Record<string, [string, string][]> = {
  'demo-checkout': [
    ['user', 'The totals at checkout are off by a cent sometimes. Can you find out why?'],
    [
      'assistant',
      'The cart adds prices as floating-point numbers, so 0.1 + 0.2 becomes 0.30000000000000004 before rounding. I changed `cartTotal` to add whole cents and only format at the end, and added a test with the three prices that failed.',
    ],
  ],
  'demo-rate-limit': [
    ['user', 'Add rate limiting to the public endpoints, 60 requests a minute per key.'],
    [
      'assistant',
      'Done: a token bucket per API key in `middleware/limit.ts`, applied to every route under `/v1/public`. Requests over the limit get a 429 with a `Retry-After` header.',
    ],
  ],
};

function den(): DenCub[] {
  return [
    {
      id: 'demo-1',
      name: 'Rounding',
      agent: 'claude',
      state: 'needs-permission',
      detail: 'npm test -- cart',
      level: 12,
      mystery: false,
      externalId: 'demo-checkout',
      cwd: SHOP,
    },
    {
      id: 'demo-2',
      name: 'Limits',
      agent: 'codex',
      state: 'editing',
      detail: 'src/middleware/limit.ts',
      level: 20,
      mystery: false,
      externalId: 'demo-rate-limit',
      cwd: API,
    },
    {
      id: 'demo-3',
      name: 'Scout',
      agent: 'claude',
      state: 'searching',
      detail: 'cartTotal',
      level: 3,
      parent: 'demo-1',
      mystery: false,
      cwd: SHOP,
    },
    {
      id: 'demo-4',
      name: 'Docs',
      agent: 'opencode',
      state: 'idle',
      level: 5,
      mystery: false,
      externalId: 'demo-readme',
      cwd: SHOP,
    },
  ];
}

function options(agent: string): AgentOptions {
  return {
    modes: [
      { id: 'default', label: 'Ask before acting', dangerous: false },
      { id: 'acceptEdits', label: 'Accept edits', dangerous: false },
    ],
    defaultMode: 'default',
    models: [
      {
        id: agent === 'codex' ? 'gpt-5' : 'default',
        label: agent === 'codex' ? 'GPT-5' : 'Default',
        efforts: ['low', 'medium', 'high'],
      },
    ],
    questions: true,
    followUps: false,
    liveChanges: false,
    assumed: false,
  };
}

const wait = (ms: number) => new Promise((done) => setTimeout(done, ms));

const REPLY =
  'This is the demo computer, so nothing ran for real. On your own computer the agent would now work in the project folder, and you would follow its steps here, answer what it asks, and get its answer on this phone.';

/** The scripted turn: it reads, asks to run a command, then answers. */
function runTurn(prompt: string, onEvent: (event: TurnEvent) => void): TurnHandle {
  let over = false;
  let answered: ((allowed: boolean) => void) | null = null;
  const say = (event: TurnEvent) => {
    if (!over) onEvent(event);
  };
  const end = (event: TurnEvent) => {
    say(event);
    say({ type: 'exited', code: 0 });
    over = true;
  };

  void (async () => {
    say({ type: 'started', sessionId: `demo-${Date.now().toString(36)}`, model: 'Demo' });
    await wait(600);
    say({ type: 'tool-use', id: 't1', name: 'Read', summary: 'README.md', detail: '' });
    await wait(700);
    say({ type: 'tool-result', id: 't1', ok: true, summary: '42 lines', detail: '' });
    await wait(400);
    say({
      type: 'question',
      id: 'q1',
      ask: {
        kind: 'permission',
        tool: 'Bash',
        summary: 'npm test',
        detail: 'npm test',
        reason: 'To check that everything still passes.',
        choices: [
          { id: 'allow', effect: 'allow-once', label: 'Allow', takesReason: false },
          { id: 'deny', effect: 'deny', label: 'Deny', takesReason: true },
        ],
      },
    });
    const allowed = await new Promise<boolean>((done) => {
      answered = done;
    });
    say({ type: 'question-resolved', id: 'q1' });
    if (allowed) {
      say({ type: 'tool-use', id: 't2', name: 'Bash', summary: 'npm test', detail: 'npm test' });
      await wait(1200);
      say({ type: 'tool-result', id: 't2', ok: true, summary: '18 passed', detail: '' });
    }
    await wait(300);
    const text = `You asked: “${prompt.trim().slice(0, 120)}”.\n\n${REPLY}`;
    for (const word of text.split(/(?<= )/)) {
      if (over) return;
      say({ type: 'text-delta', text: word });
      await wait(25);
    }
    say({ type: 'text', text });
    end({
      type: 'finished',
      ok: true,
      durationMs: 4_000,
      deniedTools: allowed ? [] : ['Bash'],
      interrupted: false,
    });
  })();

  return {
    pty: `demo-${Date.now()}`,
    cancel() {
      end({ type: 'finished', ok: false, deniedTools: [], interrupted: true });
    },
    answer(_questionId, reply) {
      const allowed = reply.kind === 'permission' && reply.choiceId === 'allow';
      answered?.(allowed);
      answered = null;
    },
    say() {
      throw new LinkError('NotSupported');
    },
    interrupt() {
      throw new LinkError('NotSupported');
    },
    setMode() {
      throw new LinkError('NotSupported');
    },
    setModel() {
      throw new LinkError('NotSupported');
    },
    unsent: () => 0,
  };
}

/** The connection to the demo computer: online a moment after it is opened. */
export function createDemoLink(onState: (state: ConnState) => void): HostLink {
  let state: ConnState = { kind: 'connecting' };
  const timer = setTimeout(() => {
    state = { kind: 'online' };
    onState(state);
  }, 300);
  queueMicrotask(() => onState(state));

  return {
    state: () => state,
    hostName: () => DEMO_HOST.hostName,
    reconnectNow() {},
    close() {
      clearTimeout(timer);
      state = { kind: 'closed' };
    },
    async shareState(): Promise<SharedState> {
      const all = sessions();
      return {
        projects: [
          { name: 'shop', root: SHOP },
          { name: 'api', root: API },
        ],
        sessions: all,
        truncated: false,
        pinned: all.filter((s) => s.externalId === 'demo-rate-limit' || s.externalId === 'demo-dark'),
        den: true,
      };
    },
    den: async () => den(),
    async transcript(_agent, externalId): Promise<TranscriptEntry[] | undefined> {
      const said = TRANSCRIPTS[externalId];
      const start = Date.now() - 30 * MIN;
      return (said ?? [['user', 'A session of the demo computer.']]).map(([role, text], at) => ({
        role,
        text,
        atMs: start + at * MIN,
      }));
    },
    homeDir: async () => HOME,
    async subdirs(path) {
      if (path === HOME) return ['code'];
      if (path === `${HOME}/code`) return ['api', 'shop'];
      return [];
    },
    worktrees: async (roots) => roots.map((root) => [{ path: root, branch: 'main', main: true }]),
    projectLogo: async () => ({ kind: 'none' }),
    installedAgents: async () => ['claude', 'codex', 'opencode'],
    agentOptions: async (agent) => options(agent),
    startTurn: async (request, onEvent) => runTurn(request.prompt, onEvent),
    runningTurns: async () => [],
    async attachTurn() {
      throw new LinkError('TurnNotFound');
    },
  };
}
