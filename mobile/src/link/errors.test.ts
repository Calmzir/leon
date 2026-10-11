import { describe, expect, test } from 'bun:test';

import { connectionWord, describeConnection, explain } from './errors';
import { LinkError } from './types';

describe('explain', () => {
  test('names the relay it could not reach and why', () => {
    const { headline } = explain(new LinkError('RelayUnreachable', 'timed out'), 'ws://10.0.0.2:8787');
    expect(headline).toBe('Cannot reach the relay at ws://10.0.0.2:8787 (timed out).');
  });

  test('every code has a headline and advice', () => {
    for (const code of ['BadCode', 'CodeExpired', 'WrongCode', 'Denied', 'Protocol'] as const) {
      const said = explain(new LinkError(code));
      expect(said.headline.length).toBeGreaterThan(5);
      expect(said.advice.length).toBeGreaterThan(5);
    }
  });

  test('anything else is an internal error with its message', () => {
    expect(explain(new Error('boom')).advice).toBe('boom');
  });
});

describe('connection wording', () => {
  test('online says nothing; the rest say what to do', () => {
    expect(describeConnection({ kind: 'online' })).toBeNull();
    expect(describeConnection(undefined)).toBe('Connecting…');
    expect(
      describeConnection({ kind: 'offline', reason: 'revoked', message: '', attempt: 1 }),
    ).toContain('Pair with it again');
    expect(connectionWord({ kind: 'offline', reason: 'host-offline', message: '', attempt: 3 })).toBe('Offline');
  });
});
