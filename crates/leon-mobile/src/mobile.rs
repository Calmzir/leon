//! The entry point: this device's identity, pairing, and connecting.
//!
//! The application gives a folder of its own (private to it on both phone
//! systems); the device's keys live there as `identity.key`, as on a
//! desktop. Pairing returns a [`PairedHost`], which the application stores
//! (it is the other computer's public identity, nothing secret) and hands
//! back to [`LeonMobile::connect`] from then on.

use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;

use leon_link::client::{Client, ClientConfig, RelayDialer};
use leon_link::code::{shape, CodeShape};
use leon_link::relay_client::{dial, Target};
use leon_link::{pair_as_client, Identity, PairingCode};
use leon_wire::HostId;

use crate::error::MobileError;
use crate::runtime::{on_runtime, runtime};
use crate::session::{ConnectionListener, HostSession};

/// A computer this device has paired with. Store it; it holds no secret.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct PairedHost {
    /// The computer's id on the relay.
    pub host_id: String,
    /// Its pinned public key, in hexadecimal. Sessions accept this key only.
    pub host_key_hex: String,
    /// The name it gave itself.
    pub host_name: String,
    /// The relay both sides meet at.
    pub relay_url: String,
}

/// What has been typed of a pairing code so far.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum CodeCheck {
    /// Nothing yet.
    Empty,
    /// A good beginning: `have` of `need` symbols.
    Incomplete {
        /// Symbols typed.
        have: u32,
        /// Symbols a code has.
        need: u32,
    },
    /// A whole code: `display` is how to show it (`ABCD-EFG-HJK`).
    Complete {
        /// The code, grouped.
        display: String,
    },
    /// A character codes never contain (`0`, `1`, `I` and `O` are not used).
    BadCharacter {
        /// The character.
        character: String,
    },
    /// More symbols than a code has.
    TooLong,
}

/// Classifies a pairing code while it is being typed. Spaces, dashes and
/// case do not matter.
#[uniffi::export]
pub fn check_code(input: String) -> CodeCheck {
    match shape(&input) {
        CodeShape::Empty => CodeCheck::Empty,
        CodeShape::Incomplete { have, need } => CodeCheck::Incomplete {
            have: have as u32,
            need: need as u32,
        },
        CodeShape::Complete => match PairingCode::parse(&input) {
            Some(code) => CodeCheck::Complete {
                display: code.display(),
            },
            None => CodeCheck::TooLong,
        },
        CodeShape::BadCharacter(character) => CodeCheck::BadCharacter {
            character: character.to_string(),
        },
        CodeShape::TooLong => CodeCheck::TooLong,
    }
}

/// This device, as the computers it pairs with know it.
#[derive(uniffi::Object)]
pub struct LeonMobile {
    identity: Arc<Identity>,
    device_name: String,
}

#[uniffi::export]
impl LeonMobile {
    /// Loads this device's keys from `data_dir`, creating them the first
    /// time. `device_name` is what the other computer shows when it asks its
    /// owner to approve the pairing, and in its list of devices.
    #[uniffi::constructor]
    pub fn new(data_dir: String, device_name: String) -> Result<Arc<Self>, MobileError> {
        let identity = Identity::load_or_create(Path::new(&data_dir))?;
        Ok(Arc::new(Self {
            identity: Arc::new(identity),
            device_name,
        }))
    }

    /// This device's fingerprint (`ABC-DEF-GHJ-KLM`): what the other
    /// computer shows next to its name, to compare before approving.
    pub fn device_id(&self) -> String {
        self.identity.device_id().short()
    }

    /// The name this device gives itself.
    pub fn device_name(&self) -> String {
        self.device_name.clone()
    }

    /// Pairs with the computer showing `code`, through the relay at
    /// `relay_url`. The other computer may ask its owner first, which can
    /// take up to two minutes.
    pub async fn pair(&self, relay_url: String, code: String) -> Result<PairedHost, MobileError> {
        let code = PairingCode::parse(&code).ok_or(MobileError::BadCode)?;
        let identity = self.identity.clone();
        let device_name = self.device_name.clone();
        on_runtime(async move {
            let pipe = dial(&relay_url, &Target::Room(code.room()), None).await?;
            let paired = pair_as_client(pipe, &identity, &code, &device_name).await?;
            Ok(PairedHost {
                host_id: paired.host_id.to_string(),
                host_key_hex: hex(&paired.host_key),
                host_name: paired.host_name,
                relay_url,
            })
        })
        .await
    }

    /// Opens the durable session to a paired computer. It connects in the
    /// background and keeps reconnecting; `listener` is told every change,
    /// starting with the current state.
    pub fn connect(
        &self,
        host: PairedHost,
        listener: Box<dyn ConnectionListener>,
    ) -> Result<Arc<HostSession>, MobileError> {
        let host_id = HostId::from_str(&host.host_id).map_err(|_| MobileError::BadHost)?;
        let host_key = unhex(&host.host_key_hex).ok_or(MobileError::BadHost)?;
        let dialer = RelayDialer {
            url: host.relay_url,
            host_id,
            token: None,
        };
        let config = ClientConfig::new(
            self.identity.clone(),
            host_key,
            Arc::new(dialer),
            self.device_name.clone(),
        );
        // The client spawns its task on the runtime that is current.
        let _context = runtime().enter();
        Ok(HostSession::new(Client::start(config), listener))
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn unhex(text: &str) -> Option<[u8; 32]> {
    let text = text.trim().as_bytes();
    if text.len() != 64 {
        return None;
    }
    let mut key = [0u8; 32];
    for (byte, pair) in key.iter_mut().zip(text.chunks(2)) {
        let pair = std::str::from_utf8(pair).ok()?;
        *byte = u8::from_str_radix(pair, 16).ok()?;
    }
    Some(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_code_is_classified_while_it_is_typed() {
        assert_eq!(check_code("".into()), CodeCheck::Empty);
        assert_eq!(
            check_code("abc".into()),
            CodeCheck::Incomplete { have: 3, need: 10 }
        );
        assert_eq!(
            check_code("ab0".into()),
            CodeCheck::BadCharacter {
                character: "0".into()
            }
        );
        assert_eq!(
            check_code("abcd efg-hjk".into()),
            CodeCheck::Complete {
                display: "ABCD-EFG-HJK".into()
            }
        );
        assert_eq!(check_code("ABCDEFGHJKL".into()), CodeCheck::TooLong);
    }

    #[test]
    fn a_key_survives_being_written_as_text() {
        let key: [u8; 32] = std::array::from_fn(|at| (at as u8).wrapping_mul(7).wrapping_add(250));
        assert_eq!(unhex(&hex(&key)), Some(key));
        assert_eq!(unhex("abc"), None);
        assert_eq!(unhex(&"zz".repeat(32)), None);
        assert_eq!(unhex(&"é".repeat(32)), None);
    }

    #[test]
    fn the_same_folder_gives_the_same_device() {
        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path().to_string_lossy().into_owned();
        let first = LeonMobile::new(folder.clone(), "Ana's phone".into()).unwrap();
        let again = LeonMobile::new(folder, "Ana's phone".into()).unwrap();
        assert_eq!(first.device_id(), again.device_id());
        assert_eq!(first.device_id().len(), 15);
    }

    #[test]
    fn a_damaged_saved_computer_is_refused_before_connecting() {
        struct Quiet;
        impl ConnectionListener for Quiet {
            fn on_state(&self, _state: crate::session::ConnectionState) {}
        }
        let dir = tempfile::tempdir().unwrap();
        let mobile =
            LeonMobile::new(dir.path().to_string_lossy().into_owned(), "phone".into()).unwrap();
        let host = PairedHost {
            host_id: "not an id".into(),
            host_key_hex: "00".repeat(32),
            host_name: "box".into(),
            relay_url: "ws://127.0.0.1:1".into(),
        };
        let refused = mobile.connect(host, Box::new(Quiet));
        assert!(matches!(refused, Err(MobileError::BadHost)));
    }

    #[test]
    fn an_incomplete_code_never_reaches_the_relay() {
        let dir = tempfile::tempdir().unwrap();
        let mobile =
            LeonMobile::new(dir.path().to_string_lossy().into_owned(), "phone".into()).unwrap();
        let outcome = runtime().block_on(mobile.pair("ws://127.0.0.1:1".into(), "ABC".into()));
        assert_eq!(outcome, Err(MobileError::BadCode));
    }
}
