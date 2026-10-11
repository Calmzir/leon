// What to tell the person when the library fails: a headline and what to do
// next, in the desktop application's words (crates/app/src/pair.rs).
import { LinkError, type ConnState, type LinkErrorCode } from './types';

export type Explanation = { headline: string; advice: string };

const TABLE: Record<LinkErrorCode, (detail: string, relayUrl: string) => Explanation> = {
  Storage: (detail) => ({
    headline: 'This device could not read or save its key.',
    advice: `Free some storage and try again. ${detail}`.trim(),
  }),
  BadCode: () => ({
    headline: 'That is not a code.',
    advice: 'A code is ten letters and digits, shown as ABCD-EFG-HJK. It never contains 0, 1, I or O.',
  }),
  BadHost: () => ({
    headline: 'The saved details of that computer are damaged.',
    advice: 'Remove it from the list and pair with it again.',
  }),
  RelayUnreachable: (detail, relayUrl) => ({
    headline: `Cannot reach the relay at ${relayUrl}${detail ? ` (${detail})` : ''}.`,
    advice:
      'Check that this device is online. The relay service is operated by Zavu and may not be live yet; if you run your own relay, check its address.',
  }),
  RelayRefused: (detail, relayUrl) => ({
    headline: `The relay at ${relayUrl} refused the connection${detail ? `: ${detail}` : '.'}`,
    advice: 'Try again in a moment.',
  }),
  CodeNotFound: () => ({
    headline: 'No computer is showing that code.',
    advice:
      'Check the code for typing mistakes. A code lasts ten minutes and works once: on the other computer choose New code, and make sure Share this machine is on there.',
  }),
  CodeExpired: () => ({
    headline: 'That code has expired.',
    advice: 'Codes last ten minutes. On the other computer choose New code.',
  }),
  CodeBurned: () => ({
    headline: 'That code was burned by too many wrong attempts.',
    advice: 'On the other computer choose New code and type it carefully.',
  }),
  CodeUsed: () => ({
    headline: 'That code was already used.',
    advice: 'A code works once. On the other computer choose New code.',
  }),
  WrongCode: () => ({
    headline: 'The other computer did not accept the code.',
    advice:
      'It is wrong, expired or already used. Check it against the screen of the other computer; it allows five wrong attempts.',
  }),
  Denied: () => ({
    headline: 'Someone on the other computer declined.',
    advice: 'Ask them to allow the connection when it is offered there, then try again.',
  }),
  Timeout: () => ({
    headline: 'The other computer stopped answering.',
    advice: 'It may have gone to sleep or lost its connection. Try again.',
  }),
  Offline: (detail) => ({
    headline: 'The other computer is not reachable right now.',
    advice: detail || 'Check that it is on, awake and sharing, then try again.',
  }),
  Disconnected: () => ({
    headline: 'The connection dropped before the answer arrived.',
    advice: 'It reconnects by itself. Try again in a moment.',
  }),
  Closed: () => ({
    headline: 'The connection to that computer is closed.',
    advice: 'Open the computer from the list again.',
  }),
  Host: (detail) => ({
    headline: 'The other computer could not do that.',
    advice: detail || 'Try again.',
  }),
  UnknownAgent: () => ({
    headline: 'This app cannot drive that agent yet.',
    advice: 'Start a chat with Claude Code, Codex or opencode.',
  }),
  TurnNotFound: () => ({
    headline: 'That answer is no longer on the other computer.',
    advice: 'It keeps a finished answer for half an hour. Ask again.',
  }),
  UnknownQuestion: () => ({
    headline: 'The agent no longer waits for that answer.',
    advice: 'It was answered already, or the agent went on without it.',
  }),
  BadAnswer: () => ({
    headline: 'That answer does not fit what was asked.',
    advice: 'Answer it again from the question itself.',
  }),
  NotSupported: () => ({
    headline: 'This agent cannot be told that while it works.',
    advice: 'Wait for its answer, or stop it.',
  }),
  TurnEnded: () => ({
    headline: 'The agent has already finished.',
    advice: 'Send it as a new message.',
  }),
  Protocol: () => ({
    headline: 'The two devices could not agree on a secure channel.',
    advice: 'Make sure both run a recent Leon, then try again.',
  }),
  Internal: (detail) => ({
    headline: 'Something went wrong in the app.',
    advice: detail || 'Try again. If it keeps happening, report it.',
  }),
};

/** Explains any thrown value; `relayUrl` names the relay in the messages about it. */
export function explain(error: unknown, relayUrl = 'the relay'): Explanation {
  if (error instanceof LinkError) {
    return TABLE[error.code](error.detail ?? '', relayUrl);
  }
  return TABLE.Internal(error instanceof Error ? error.message : String(error), relayUrl);
}

/** One line for the connection state; nothing when online. */
export function describeConnection(state: ConnState | undefined): string | null {
  if (!state || state.kind === 'connecting') return 'Connecting…';
  if (state.kind === 'online') return null;
  if (state.kind === 'closed') return 'Not connected.';
  switch (state.reason) {
    case 'relay-unreachable':
      return 'Cannot reach the relay. Check that this device is online; it keeps trying.';
    case 'host-offline':
      return 'The other computer is off, asleep or not sharing. It keeps trying.';
    case 'revoked':
      return 'The other computer no longer accepts this device. Pair with it again.';
    case 'version-mismatch':
      return 'The other computer runs a Leon this app cannot talk to. Update both.';
    case 'other':
      return state.message ? `Not connected: ${state.message}. It keeps trying.` : 'Not connected. It keeps trying.';
  }
}

/** A word for the connection, for lists. */
export function connectionWord(state: ConnState | undefined): string {
  if (!state || state.kind === 'connecting') return 'Connecting';
  if (state.kind === 'online') return 'Online';
  if (state.kind === 'closed') return 'Closed';
  if (state.reason === 'revoked') return 'Revoked';
  if (state.reason === 'version-mismatch') return 'Update needed';
  return 'Offline';
}
