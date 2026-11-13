use std::fs;
use std::io::Read as _;
use std::os::fd::AsFd as _;
use std::os::unix::fs::{PermissionsExt as _, symlink};
use std::os::unix::net::UnixListener;
use std::process::Command;
use std::sync::mpsc;

use nix::poll::{PollFd, PollFlags, poll};
use serde_json::{Value, json};
use tempfile::TempDir;

use super::*;

const TEST_SESSION_ID: &str = "01234567-89ab-cdef-0123-456789abcdef";

/// An isolated registry and executable whose sole listed process is this test.
struct Fixture {
    directory: TempDir,
    inventory: SessionInventory,
    listener: UnixListener,
}

impl Fixture {
    /// Keeps every path short enough for macOS's Unix socket path limit.
    fn new() -> Self {
        let directory = tempfile::tempdir_in("/tmp").unwrap();
        let configuration_directory = directory.path().to_path_buf();
        let executable = directory.path().join("claude");
        fs::write(
            &executable,
            "#!/bin/sh\nexec /bin/cat \"$CLAUDE_CONFIG_DIR/list.json\"\n",
        )
        .unwrap();
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
        fs::create_dir(directory.path().join("sessions")).unwrap();
        let socket_path = directory.path().join("inbox.sock");
        let listener = UnixListener::bind(&socket_path).unwrap();
        let process_id = std::process::id();
        let output = crate::command_output::command_output(
            Command::new("/bin/ps")
                .args(["-p", &process_id.to_string(), "-o", "lstart="])
                .env("LC_ALL", "C")
                .env("TZ", "UTC"),
            Duration::from_secs(2),
        )
        .unwrap();
        let process_start = String::from_utf8(output).unwrap().trim().to_owned();
        fs::write(
            directory.path().join(format!("sessions/{process_id}.json")),
            json!({
                "pid":process_id,"sessionId":TEST_SESSION_ID,"procStart":process_start,
                "messagingSocketPath":socket_path,
            })
            .to_string(),
        )
        .unwrap();
        let fixture = Self {
            directory,
            inventory: SessionInventory {
                executable,
                configuration_directory,
            },
            listener,
        };
        fixture.set_list(&json!([{"pid":process_id,"sessionId":TEST_SESSION_ID,"name":"hello","cwd":"/test/project","status":"idle"}]));
        fixture
    }

    fn set_list(&self, value: &Value) {
        fs::write(self.directory.path().join("list.json"), value.to_string()).unwrap();
    }

    fn session(&self) -> ClaudeSession {
        self.inventory.refresh().unwrap().remove(0)
    }

    /// Receives one bounded message, failing even when no client connects.
    fn receive(&self) -> mpsc::Receiver<Vec<u8>> {
        let listener = self.listener.try_clone().unwrap();
        let (sender, receiver) = mpsc::sync_channel(1);
        std::thread::spawn(move || {
            let mut descriptors = [PollFd::new(listener.as_fd(), PollFlags::POLLIN)];
            assert_eq!(poll(&mut descriptors, 2000_u16).unwrap(), 1);
            let (mut stream, _) = listener.accept().unwrap();
            stream.set_nonblocking(true).unwrap();
            let mut bytes = Vec::new();
            let deadline = Instant::now() + Duration::from_secs(2);
            loop {
                let remaining = deadline.saturating_duration_since(Instant::now());
                assert!(!remaining.is_zero());
                let mut descriptors = [PollFd::new(stream.as_fd(), PollFlags::POLLIN)];
                assert_eq!(
                    poll(
                        &mut descriptors,
                        nix::poll::PollTimeout::try_from(remaining).unwrap()
                    )
                    .unwrap(),
                    1
                );
                let mut buffer = [0; 4096];
                match stream.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(count) => bytes.extend_from_slice(&buffer[..count]),
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
                    Err(error) => panic!("{error}"),
                }
            }
            sender.send(bytes).unwrap();
        });
        receiver
    }
}

/// Sends the exported text unchanged, followed by attribution, in one frame without reply or permission claims.
#[test]
fn submission_preserves_content_and_explains_its_source() {
    let fixture = Fixture::new();
    let selected = fixture.session();
    let receiver = fixture.receive();
    SessionSender::new(fixture.inventory.clone())
        .send(&selected, "Review\nquotes: \"x\" — café\n")
        .unwrap();
    let bytes = receiver.recv_timeout(Duration::from_secs(3)).unwrap();
    assert_eq!(bytes.last(), Some(&b'\n'));
    assert!(!bytes[..bytes.len() - 1].contains(&b'\n'));
    let frame: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(frame["session_id"], TEST_SESSION_ID);
    assert_eq!(frame["type"], "user");
    assert_eq!(frame["message"]["role"], "user");
    assert_eq!(
        frame["message"]["content"],
        format!("Review\nquotes: \"x\" — café\n\n\n---\n\n{ATTRIBUTION}")
    );
    assert_eq!(frame.as_object().unwrap().len(), 3);
}

/// Claude's UTC registry timestamp matches a live process even when the app inherits local time.
#[test]
fn inventory_accepts_utc_registry_in_non_utc_timezone() {
    const CHILD_VARIABLE: &str = "QD_CLAUDE_TIMEZONE_TEST_CHILD";

    // Isolate the timezone in a subprocess; changing this process's environment
    // would race the other tests and is unsafe in a multithreaded Rust program.
    if std::env::var_os(CHILD_VARIABLE).is_none() {
        crate::command_output::command_output(
            Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "session_sender::tests::inventory_accepts_utc_registry_in_non_utc_timezone",
                    "--nocapture",
                ])
                .env(CHILD_VARIABLE, "1")
                .env("TZ", "Europe/Berlin"),
            Duration::from_secs(5),
        )
        .expect("Inventory must find a UTC-recorded session from a non-UTC app process");
        return;
    }

    // The fixture uses Claude's UTC representation, independently of the
    // inventory's OS process query. No messages are sent in this test.
    let fixture = Fixture::new();
    assert_eq!(fixture.inventory.refresh().unwrap().len(), 1);
}

/// Every send rereads inventory, so a session removed since selection is never contacted.
#[test]
fn ended_selection_is_rejected_before_connecting() {
    let fixture = Fixture::new();
    let selected = fixture.session();
    fixture.set_list(&json!([]));
    let error = SessionSender::new(fixture.inventory.clone())
        .send(&selected, "notes")
        .unwrap_err();
    assert!(error.to_string().contains("ended or changed"));
    fixture.listener.set_nonblocking(true).unwrap();
    assert_eq!(
        fixture.listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
}

/// A reused PID or a different UUID cannot inherit an old selection.
#[test]
fn replaced_identity_is_not_a_destination() {
    let fixture = Fixture::new();
    let mut selected = fixture.session();
    selected.process_start = "previous process".into();
    assert!(
        SessionSender::new(fixture.inventory.clone())
            .send(&selected, "notes")
            .is_err()
    );
    selected = fixture.session();
    selected.id = crate::SessionId::parse("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee".into()).unwrap();
    assert!(
        SessionSender::new(fixture.inventory.clone())
            .send(&selected, "notes")
            .is_err()
    );
}

/// Exited background records are not offered alongside a live session.
#[test]
fn inventory_skips_unsendable_entries() {
    let fixture = Fixture::new();
    fixture.set_list(&json!([
        {"pid":std::process::id(),"sessionId":TEST_SESSION_ID,"cwd":"/test","status":"busy"},
        {"cwd":"/test","state":"working"},
        {"pid":0,"sessionId":"bad","cwd":"/test"}
    ]));
    let sessions = fixture.inventory.refresh().unwrap();
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].status, "busy");
    assert_eq!(sessions[0].name, "Unnamed session");
}

/// Corrupt records, reused PIDs and symlinked endpoints cannot become destinations.
#[test]
fn changing_registry_and_symlink_are_skipped() {
    let fixture = Fixture::new();
    let registry = fixture
        .directory
        .path()
        .join(format!("sessions/{}.json", std::process::id()));
    let original = fs::read_to_string(&registry).unwrap();
    fs::write(&registry, "{").unwrap();
    assert!(fixture.inventory.refresh().unwrap().is_empty());
    let mut value: Value = serde_json::from_str(&original).unwrap();
    value["procStart"] = json!("old process");
    fs::write(&registry, value.to_string()).unwrap();
    assert!(fixture.inventory.refresh().unwrap().is_empty());
    fs::write(&registry, original).unwrap();
    let socket = fixture.directory.path().join("inbox.sock");
    let moved = fixture.directory.path().join("moved.sock");
    fs::rename(&socket, &moved).unwrap();
    symlink(moved, socket).unwrap();
    assert!(fixture.inventory.refresh().unwrap().is_empty());
}

/// Malformed command output is a visible discovery error rather than an empty result.
#[test]
fn malformed_inventory_reports_an_error() {
    let fixture = Fixture::new();
    fixture.set_list(&json!({"unexpected":"version"}));
    assert!(
        fixture
            .inventory
            .refresh()
            .unwrap_err()
            .to_string()
            .contains("unsupported")
    );
}

/// The serialized cap includes escaping and the attribution, before any connection.
#[test]
fn empty_and_oversized_messages_are_rejected() {
    let fixture = Fixture::new();
    let session = fixture.session();
    assert!(message_frame(&session, " \n").is_err());
    assert!(message_frame(&session, &"\"".repeat(MAX_FRAME_BYTES / 2)).is_err());
}
