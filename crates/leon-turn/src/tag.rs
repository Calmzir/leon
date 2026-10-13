//! Tags: the names a message to an agent is known by once it was delivered.
//!
//! A tag made from an id the agent chose must survive whatever carries it (a
//! pipe, a terminal line) and be read back exactly, so an id that is not
//! plain letters, digits, `-` and `_` is written as hexadecimal.

/// A tag made of `kind` and an id of the agent's, reversible whatever the id
/// holds: an id that is not plain is written in hexadecimal after `x`.
pub fn id_tag(kind: char, id: &str) -> String {
    let plain = !id.is_empty()
        && !id.starts_with('x')
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'));
    if plain {
        return format!("{kind}.{id}");
    }
    let hex: String = id.bytes().map(|byte| format!("{byte:02x}")).collect();
    format!("{kind}.x{hex}")
}

/// The id inside a tag written by [`id_tag`] for `kind`.
pub fn tag_id(kind: char, tag: &str) -> Option<String> {
    let rest = tag.strip_prefix(kind)?.strip_prefix('.')?;
    let Some(hex) = rest.strip_prefix('x') else {
        return Some(rest.to_owned());
    };
    if hex.len() % 2 != 0 {
        return None;
    }
    let bytes: Option<Vec<u8>> = (0..hex.len() / 2)
        .map(|at| u8::from_str_radix(hex.get(at * 2..at * 2 + 2)?, 16).ok())
        .collect();
    String::from_utf8(bytes?).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tag_is_read_back_as_the_id_it_was_made_from() {
        assert_eq!(id_tag('a', "0f3c-11"), "a.0f3c-11");
        assert_eq!(tag_id('a', "a.0f3c-11").as_deref(), Some("0f3c-11"));
        for id in ["", "x1", "has space", "né", "7", "a.b"] {
            let tag = id_tag('q', id);
            assert!(
                tag.chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.')),
                "{tag:?}"
            );
            assert_eq!(tag_id('q', &tag).as_deref(), Some(id), "{id:?}");
            assert_eq!(tag_id('a', &tag), None);
        }
    }
}
