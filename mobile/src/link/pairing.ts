// What a pairing code arrives as when it is not typed: the link "Share this
// machine" shows as a QR and copies (`leon://pair?code=…&relay=…`), or the
// bare code. The same reading as `leon_link::parse_text`.

export type PairingText = { code: string; relay?: string };

const LINK = /^leon:\/\/pair\/?\?(.*)$/i;
const SYMBOLS = /^[A-HJ-NP-Z2-9]{10}$/;

/** The code as shown, `ABCD-EFG-HJK`; `null` unless it is a whole code. */
function shown(text: string): string | null {
  const symbols = text.replace(/[\s._-]/g, '').toUpperCase();
  if (!SYMBOLS.test(symbols)) return null;
  return `${symbols.slice(0, 4)}-${symbols.slice(4, 7)}-${symbols.slice(7)}`;
}

function decoded(value: string): string | null {
  try {
    return decodeURIComponent(value.replace(/\+/g, ' '));
  } catch {
    return null;
  }
}

/** Reads what was pasted or scanned; `null` when it is neither a code nor a link. */
export function parsePairing(input: string): PairingText | null {
  const text = input.trim();
  const link = LINK.exec(text);
  if (!link) {
    const code = shown(text);
    return code ? { code } : null;
  }
  let code: string | null = null;
  let relay: string | undefined;
  for (const pair of (link[1] ?? '').split('&')) {
    const at = pair.indexOf('=');
    const key = at < 0 ? pair : pair.slice(0, at);
    const value = decoded(at < 0 ? '' : pair.slice(at + 1));
    if (value === null) return null;
    if (key === 'code') code = shown(value);
    if (key === 'relay' && value.trim()) relay = value.trim();
  }
  return code ? { code, relay } : null;
}

/** Whether a relay address only means something on the computer that shows it. */
export function isLoopback(relayUrl: string): boolean {
  return /^wss?:\/\/(localhost|127\.\d+\.\d+\.\d+|\[::1\])([:/]|$)/i.test(relayUrl.trim());
}
