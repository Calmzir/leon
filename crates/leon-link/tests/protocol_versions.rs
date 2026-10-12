//! A client reaches a host of the previous wire version and of this one.
//!
//! The hosts here are small stand-ins that do what `leon-host` does at the
//! start of a session: accept the handshake for a known device, read the
//! greeting in their one version and hang up without a word when they cannot
//! (the 0.5.0 release does exactly that with a frame of version 2).

use std::sync::atomic::{AtomicU16, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use leon_link::client::{
    Client, ClientConfig, ClientError, ConnState, DialFuture, Dialer, Failure,
};
use leon_link::{accept, Identity, Pipe};
use leon_wire::{ErrorCode, ExecOutput, ExecSpec, Grid, Message, PROTOCOL_VERSION};

/// What the stand-in host does with a connection.
#[derive(Clone, Copy, PartialEq)]
enum Speaks {
    /// Serves sessions in its version (kept in an atomic so a test can
    /// "update" the host between connections).
    Version,
    /// Accepts the handshake, then hangs up whatever arrives.
    Nothing,
}

struct FakeHost {
    identity: Identity,
    device: [u8; 32],
    speaks: Speaks,
    version: AtomicU16,
    dials: AtomicUsize,
    sessions: AtomicUsize,
}

impl FakeHost {
    fn new(device: &Identity, speaks: Speaks, version: u16) -> Arc<Self> {
        Arc::new(Self {
            identity: Identity::generate(),
            device: device.static_public(),
            speaks,
            version: AtomicU16::new(version),
            dials: AtomicUsize::new(0),
            sessions: AtomicUsize::new(0),
        })
    }

    async fn serve(self: Arc<Self>, pipe: Pipe) {
        let device = self.device;
        let Ok(mut channel) = accept(pipe, &self.identity, |key| *key == device).await else {
            return;
        };
        if self.speaks == Speaks::Nothing {
            let _ = channel.recv_bytes().await;
            return;
        }
        let version = self.version.load(Ordering::Relaxed);
        channel.set_protocol(version);
        // As `leon-host` does: anything but a greeting it can read ends it.
        let Ok(Some(Message::Hello { protocol, .. })) = channel.recv().await else {
            return;
        };
        if protocol != version {
            return;
        }
        let hello = Message::Hello {
            protocol: version,
            app_version: "test".into(),
            device_name: format!("host-v{version}"),
            token: None,
        };
        if channel.send(&hello).await.is_err() {
            return;
        }
        self.sessions.fetch_add(1, Ordering::Relaxed);
        while let Ok(Some(message)) = channel.recv().await {
            let reply = match message {
                Message::Ping { nonce } => Message::Pong { nonce },
                Message::Exec { id, spec, .. } => Message::ExecOutput {
                    id,
                    output: ExecOutput {
                        status: Some(0),
                        stdout: format!("{} {}", spec.program, spec.args.join(" ")).into_bytes(),
                        stderr: Vec::new(),
                        truncated: false,
                    },
                },
                Message::PtyOpen { id, .. } => Message::PtyOpened { id, pty: 41 },
                _ => continue,
            };
            if channel.send(&reply).await.is_err() {
                return;
            }
        }
    }
}

struct FakeDialer(Arc<FakeHost>);

impl Dialer for FakeDialer {
    fn dial(&self) -> DialFuture {
        let host = self.0.clone();
        Box::pin(async move {
            host.dials.fetch_add(1, Ordering::Relaxed);
            let (ours, theirs) = Pipe::pair();
            tokio::spawn(host.serve(theirs));
            Ok(ours)
        })
    }
}

fn client_for(device: Identity, host: &Arc<FakeHost>) -> Client {
    let mut config = ClientConfig::new(
        Arc::new(device),
        host.identity.static_public(),
        Arc::new(FakeDialer(host.clone())),
        "phone",
    );
    config.backoff = (Duration::from_millis(10), Duration::from_millis(40));
    Client::start(config)
}

fn echo(text: &str) -> ExecSpec {
    ExecSpec {
        program: "echo".into(),
        args: vec![text.into()],
        ..Default::default()
    }
}

const LIMIT: Duration = Duration::from_secs(5);

/// Waits for the client to say it is offline, and why.
async fn offline(client: &Client) -> (Failure, String) {
    let mut state = client.watch();
    let wait = async {
        loop {
            if let ConnState::Offline {
                failure, reason, ..
            } = &*state.borrow_and_update()
            {
                return (*failure, reason.clone());
            }
            state.changed().await.unwrap();
        }
    };
    tokio::time::timeout(LIMIT, wait).await.expect("offline")
}

#[tokio::test]
async fn a_host_of_the_previous_version_is_reached_and_serves_commands_and_terminals() {
    let device = Identity::generate();
    let host = FakeHost::new(&device, Speaks::Version, 1);
    let client = client_for(device, &host);
    assert_eq!(client.protocol(), None);
    client.wait_online(LIMIT).await.unwrap();

    assert_eq!(client.protocol(), Some(1));
    assert_eq!(client.host_name().as_deref(), Some("host-v1"));
    // One connection found out that it does not speak ours, the next worked.
    assert_eq!(host.dials.load(Ordering::Relaxed), 2);
    assert_eq!(host.sessions.load(Ordering::Relaxed), 1);

    let out = client.exec(echo("hola"), None).await.unwrap();
    assert_eq!(out.stdout, b"echo hola");
    let grid = Grid {
        cols: 80,
        rows: 24,
        cell_width: 8,
        cell_height: 16,
    };
    let stream = client.pty_open(echo("x"), grid, "sh").await.unwrap();
    assert_eq!(stream.pty, 41);
}

#[tokio::test]
async fn a_request_the_older_host_cannot_take_fails_alone_and_the_connection_stays() {
    let device = Identity::generate();
    let host = FakeHost::new(&device, Speaks::Version, 1);
    let client = client_for(device, &host);
    client.wait_online(LIMIT).await.unwrap();

    let mut with_input = echo("needs input");
    with_input.stdin = Some(b"y\n".to_vec());
    match client.exec(with_input, None).await {
        Err(ClientError::Remote(error)) => {
            assert_eq!(error.code, ErrorCode::Unsupported);
            assert!(error.message.contains("older"), "{}", error.message);
        }
        other => panic!("expected a refusal, got {other:?}"),
    }
    // Nothing was sent and nothing dropped: the same session serves on.
    assert_eq!(client.state(), ConnState::Online);
    assert_eq!(
        client.exec(echo("after"), None).await.unwrap().stdout,
        b"echo after"
    );
    assert_eq!(host.sessions.load(Ordering::Relaxed), 1);
}

#[tokio::test]
async fn a_host_of_this_version_is_reached_on_the_first_try() {
    let device = Identity::generate();
    let host = FakeHost::new(&device, Speaks::Version, PROTOCOL_VERSION);
    let client = client_for(device, &host);
    client.wait_online(LIMIT).await.unwrap();

    assert_eq!(client.protocol(), Some(PROTOCOL_VERSION));
    assert_eq!(host.dials.load(Ordering::Relaxed), 1);
    let mut with_input = echo("fine here");
    with_input.stdin = Some(b"y\n".to_vec());
    assert!(client.exec(with_input, None).await.is_ok());
}

#[tokio::test]
async fn the_version_that_worked_is_tried_first_when_reconnecting() {
    let device = Identity::generate();
    let host = FakeHost::new(&device, Speaks::Version, 1);
    let client = client_for(device, &host);
    client.wait_online(LIMIT).await.unwrap();
    assert_eq!(host.dials.load(Ordering::Relaxed), 2);

    client.reconnect_now();
    let again = async {
        while host.sessions.load(Ordering::Relaxed) < 2 {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    };
    tokio::time::timeout(LIMIT, again).await.expect("a session");
    client.wait_online(LIMIT).await.unwrap();
    // One more connection, not two: no probing with the newer version.
    assert_eq!(host.dials.load(Ordering::Relaxed), 3);
    assert_eq!(client.protocol(), Some(1));
}

#[tokio::test]
async fn a_host_that_was_updated_is_found_again_in_its_new_version() {
    let device = Identity::generate();
    let host = FakeHost::new(&device, Speaks::Version, 1);
    let client = client_for(device, &host);
    client.wait_online(LIMIT).await.unwrap();
    assert_eq!(client.protocol(), Some(1));

    host.version.store(PROTOCOL_VERSION, Ordering::Relaxed);
    client.reconnect_now();
    let updated = async {
        while client.protocol() != Some(PROTOCOL_VERSION) {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    };
    tokio::time::timeout(LIMIT, updated)
        .await
        .expect("the new version");
    client.wait_online(LIMIT).await.unwrap();
    assert_eq!(client.host_name().as_deref(), Some("host-v2"));
}

#[tokio::test]
async fn a_device_the_host_does_not_accept_is_revoked_and_no_other_version_is_tried() {
    let device = Identity::generate();
    let stranger = Identity::generate();
    // The host knows another device, not this one.
    let host = FakeHost::new(&stranger, Speaks::Version, 1);
    let client = client_for(device, &host);

    let (failure, _) = offline(&client).await;
    assert_eq!(failure, Failure::Revoked);
    // It halts there: one connection, and none after it.
    tokio::time::sleep(Duration::from_millis(150)).await;
    assert_eq!(host.dials.load(Ordering::Relaxed), 1);
    assert_eq!(client.protocol(), None);
}

#[tokio::test]
async fn a_host_that_speaks_no_version_of_ours_is_a_version_mismatch() {
    let device = Identity::generate();
    let host = FakeHost::new(&device, Speaks::Nothing, 0);
    let client = client_for(device, &host);

    let (failure, reason) = offline(&client).await;
    assert_eq!(failure, Failure::VersionMismatch);
    assert!(reason.contains("update Leon"), "{reason}");
    // Every version was tried once, and it halts.
    tokio::time::sleep(Duration::from_millis(150)).await;
    assert_eq!(host.dials.load(Ordering::Relaxed), 2);
}
