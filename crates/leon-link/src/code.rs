//! The short one-time pairing code.
//!
//! A code is ten symbols of an unambiguous alphabet, shown as `ABCD-EFG-HJK`:
//!
//! * the first four are the **room**: not secret, they only tell the relay
//!   which waiting host to connect a newcomer to;
//! * the last six are the **secret**: 30 bits that never leave the two
//!   computers and feed SPAKE2. A password-authenticated key exchange lets
//!   such a short secret be safe: an attacker, the relay included, gets one
//!   guess per attempt and learns nothing offline, and the host burns the code
//!   after a handful of failures (see [`crate::pairing::PairingOffer`]).

use std::fmt;

use leon_wire::id::{symbol_value, ALPHABET};
use rand_core::{OsRng, RngCore};
use zeroize::Zeroize;

/// Symbols in the room part.
pub const ROOM_LEN: usize = leon_wire::relay::ROOM_LEN;
/// Symbols in the secret part.
pub const SECRET_LEN: usize = 6;
/// Symbols in a whole code.
pub const CODE_LEN: usize = ROOM_LEN + SECRET_LEN;

/// What typing a code looks like so far; for live validation in a form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodeShape {
    /// Nothing typed.
    Empty,
    /// A valid beginning: `have` of `need` symbols.
    Incomplete {
        /// Symbols so far.
        have: usize,
        /// Symbols a code has.
        need: usize,
    },
    /// A well-formed code.
    Complete,
    /// A character that is not in the alphabet (`0`, `1`, `I` and `O` are not
    /// used, so they are never mistaken for each other).
    BadCharacter(char),
    /// More symbols than a code has.
    TooLong,
}

/// Cleans typed input: drops separators and spaces, upper-cases.
fn normalise(input: &str) -> Vec<char> {
    input
        .chars()
        .filter(|c| !c.is_whitespace() && !matches!(c, '-' | '_' | '.'))
        .map(|c| c.to_ascii_uppercase())
        .collect()
}

/// Classifies what has been typed.
pub fn shape(input: &str) -> CodeShape {
    let symbols = normalise(input);
    if symbols.is_empty() {
        return CodeShape::Empty;
    }
    if let Some(bad) = symbols.iter().find(|c| symbol_value(**c).is_none()) {
        return CodeShape::BadCharacter(*bad);
    }
    match symbols.len() {
        CODE_LEN => CodeShape::Complete,
        n if n > CODE_LEN => CodeShape::TooLong,
        have => CodeShape::Incomplete {
            have,
            need: CODE_LEN,
        },
    }
}

/// A pairing code.
#[derive(Clone, PartialEq, Eq)]
pub struct PairingCode {
    symbols: [u8; CODE_LEN],
}

impl PairingCode {
    /// A fresh random code.
    pub fn generate() -> Self {
        let mut random = [0u8; CODE_LEN];
        OsRng.fill_bytes(&mut random);
        let mut symbols = [0u8; CODE_LEN];
        for (symbol, byte) in symbols.iter_mut().zip(random) {
            // 32 symbols and a byte's low five bits: uniform.
            *symbol = ALPHABET[usize::from(byte & 31)];
        }
        Self { symbols }
    }

    /// Reads a typed code; `None` unless [`shape`] says it is complete.
    pub fn parse(input: &str) -> Option<Self> {
        if shape(input) != CodeShape::Complete {
            return None;
        }
        let mut symbols = [0u8; CODE_LEN];
        for (slot, ch) in symbols.iter_mut().zip(normalise(input)) {
            *slot = ch as u8;
        }
        Some(Self { symbols })
    }

    /// The public room.
    pub fn room(&self) -> String {
        String::from_utf8_lossy(&self.symbols[..ROOM_LEN]).into_owned()
    }

    /// The secret, as the SPAKE2 password.
    pub(crate) fn password(&self) -> &[u8] {
        &self.symbols[ROOM_LEN..]
    }

    /// The code as shown: `ABCD-EFG-HJK`.
    pub fn display(&self) -> String {
        let text = String::from_utf8_lossy(&self.symbols);
        format!("{}-{}-{}", &text[0..4], &text[4..7], &text[7..10])
    }
}

/// The scheme and path of a pairing link.
const LINK: &str = "leon://pair";

/// What was pasted or scanned to pair: a code, and the relay it is for when
/// it came as a link.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairingText {
    /// The code.
    pub code: PairingCode,
    /// The relay the showing computer listens on; `None` for a bare code.
    pub relay: Option<String>,
}

impl PairingCode {
    /// The code as a link, for a QR and for copying:
    /// `leon://pair?code=ABCD-EFG-HJK&relay=wss%3A%2F%2Frelay.example`.
    ///
    /// It carries the secret part, exactly as the code on the screen does, so
    /// it is shown and copied only where the code is. It is a custom scheme on
    /// purpose: opening it never reaches a web server.
    pub fn to_link(&self, relay: &str) -> String {
        format!("{LINK}?code={}&relay={}", self.display(), encode(relay))
    }
}

/// Reads what a person pasted or a camera scanned: a pairing link, or a bare
/// code with whatever spaces and case it came with.
pub fn parse_text(input: &str) -> Option<PairingText> {
    let text = input.trim();
    let Some(query) = text
        .get(..LINK.len())
        .filter(|head| head.eq_ignore_ascii_case(LINK))
        .map(|_| text[LINK.len()..].trim_start_matches('/'))
    else {
        return PairingCode::parse(text).map(|code| PairingText { code, relay: None });
    };
    let query = query.strip_prefix('?')?;
    let mut code = None;
    let mut relay = None;
    for pair in query.split('&') {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        match key {
            "code" => code = PairingCode::parse(&decode(value)?),
            "relay" => relay = Some(decode(value)?).filter(|url| !url.trim().is_empty()),
            _ => {}
        }
    }
    Some(PairingText { code: code?, relay })
}

/// Percent-encodes everything but the unreserved characters.
fn encode(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            out.push(char::from(byte));
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

/// Undoes [`encode`]; `None` for a broken escape or bytes that are not text.
fn decode(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        match bytes[at] {
            b'%' => {
                let hex = text.get(at + 1..at + 3)?;
                out.push(u8::from_str_radix(hex, 16).ok()?);
                at += 3;
            }
            b'+' => {
                out.push(b' ');
                at += 1;
            }
            byte => {
                out.push(byte);
                at += 1;
            }
        }
    }
    String::from_utf8(out).ok()
}

impl Drop for PairingCode {
    fn drop(&mut self) {
        self.symbols.zeroize();
    }
}

impl fmt::Debug for PairingCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PairingCode(room {}, secret hidden)", self.room())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_generated_code_has_ten_symbols_of_the_alphabet_and_parses_back() {
        for _ in 0..50 {
            let code = PairingCode::generate();
            let shown = code.display();
            assert_eq!(shown.len(), 12, "{shown}");
            assert_eq!(PairingCode::parse(&shown), Some(code.clone()));
            assert_eq!(code.room().len(), ROOM_LEN);
            assert_eq!(code.password().len(), SECRET_LEN);
        }
    }

    #[test]
    fn typing_is_forgiving_about_case_spaces_and_dashes() {
        let a = PairingCode::parse("abcd efg-hjk").unwrap();
        let b = PairingCode::parse("ABCDEFGHJK").unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn the_shape_follows_the_typing() {
        assert_eq!(shape(""), CodeShape::Empty);
        assert_eq!(shape("  - "), CodeShape::Empty);
        assert_eq!(shape("AB2"), CodeShape::Incomplete { have: 3, need: 10 });
        assert_eq!(shape("ABCD-EFG-HJK"), CodeShape::Complete);
        assert_eq!(shape("ABCDEFGHJKM"), CodeShape::TooLong);
        assert_eq!(shape("AB0D"), CodeShape::BadCharacter('0'));
        assert_eq!(shape("ABOD"), CodeShape::BadCharacter('O'));
    }

    #[test]
    fn an_incomplete_or_wrong_code_does_not_parse() {
        assert!(PairingCode::parse("ABCD").is_none());
        assert!(PairingCode::parse("ABCD-EFG-HJ1").is_none());
    }

    #[test]
    fn debug_hides_the_secret_part() {
        let code = PairingCode::parse("ABCD-EFG-HJK").unwrap();
        let text = format!("{code:?}");
        assert!(text.contains("ABCD"));
        assert!(!text.contains("EFG"));
    }

    #[test]
    fn codes_are_not_repeated_in_a_sample() {
        let mut seen = std::collections::HashSet::new();
        for _ in 0..200 {
            assert!(seen.insert(PairingCode::generate().display()));
        }
    }
    #[test]
    fn a_link_carries_the_code_and_the_relay_and_reads_back() {
        let code = PairingCode::parse("ABCD-EFG-HJK").unwrap();
        let link = code.to_link("ws://192.168.1.20:8787/a b");
        assert_eq!(
            link,
            "leon://pair?code=ABCD-EFG-HJK&relay=ws%3A%2F%2F192.168.1.20%3A8787%2Fa%20b"
        );
        let read = parse_text(&format!("  {link}\n")).unwrap();
        assert_eq!(read.code, code);
        assert_eq!(read.relay.as_deref(), Some("ws://192.168.1.20:8787/a b"));
    }

    #[test]
    fn pasted_text_is_a_link_or_a_bare_code() {
        let bare = parse_text(" abcd efg hjk ").unwrap();
        assert_eq!(bare.code.display(), "ABCD-EFG-HJK");
        assert_eq!(bare.relay, None);
        let no_relay = parse_text("LEON://pair/?code=ABCDEFGHJK").unwrap();
        assert_eq!(no_relay.code, bare.code);
        assert_eq!(no_relay.relay, None);
        assert!(parse_text("leon://pair?relay=wss%3A%2F%2Fx").is_none());
        assert!(parse_text("leon://pair?code=ABCD").is_none());
        assert!(parse_text("leon://pair?code=ABCD-EFG-HJK&relay=%ZZ").is_none());
        assert!(parse_text("https://example.com/?code=ABCD-EFG-HJK").is_none());
        assert!(parse_text("").is_none());
    }
}
