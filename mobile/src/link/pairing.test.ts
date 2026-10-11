import { describe, expect, test } from 'bun:test';

import { isLoopback, parsePairing } from './pairing';

describe('parsePairing', () => {
  test('reads the link a computer shows, with its relay', () => {
    expect(parsePairing(' leon://pair?code=ABCD-EFG-HJK&relay=ws%3A%2F%2F192.168.1.20%3A8787\n')).toEqual({
      code: 'ABCD-EFG-HJK',
      relay: 'ws://192.168.1.20:8787',
    });
    expect(parsePairing('LEON://pair/?code=abcdefghjk')).toEqual({ code: 'ABCD-EFG-HJK', relay: undefined });
  });

  test('reads a bare code however it was written', () => {
    expect(parsePairing('abcd efg hjk')).toEqual({ code: 'ABCD-EFG-HJK' });
  });

  test('refuses what is not a code', () => {
    expect(parsePairing('')).toBeNull();
    expect(parsePairing('ABCD-EFG-HJ1')).toBeNull();
    expect(parsePairing('leon://pair?relay=wss%3A%2F%2Fx')).toBeNull();
    expect(parsePairing('leon://pair?code=ABCD-EFG-HJK&relay=%ZZ')).toBeNull();
    expect(parsePairing('https://example.com/?code=ABCD-EFG-HJK')).toBeNull();
  });
});

test('a relay on the computer itself is not one a phone can reach', () => {
  expect(isLoopback('ws://127.0.0.1:8787')).toBe(true);
  expect(isLoopback('ws://localhost:8787')).toBe(true);
  expect(isLoopback('wss://relay.getleon.dev')).toBe(false);
  expect(isLoopback('ws://10.0.2.2:8787')).toBe(false);
});
