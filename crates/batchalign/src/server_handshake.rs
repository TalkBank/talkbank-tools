//! What a running server has published about itself.
//!
//! # Why this module exists
//!
//! "Which port is the server listening on?" had three answers in the state
//! directory and none of them was an observation:
//!
//! - `server.yaml`'s `port` is a REQUEST. It says what the operator asked for,
//!   not what the server got, and for an ephemeral request it cannot say.
//! - `daemon.json`'s `port` is written by the PARENT process before the child
//!   has bound anything, so it is a prediction.
//! - `server.pid` held a bare PID, so every reader re-derived the port from
//!   `server.yaml`, which is the request again.
//!
//! Passing a port NUMBER between processes is racy exactly because the number
//! is a prediction: the reserve-then-bind window belongs to whoever asks the
//! kernel next. The cure is for the process that binds to report what it got.
//! This module owns that report.
//!
//! # One owner
//!
//! Exactly one process writes: the server, after its bind succeeds, because it
//! is the only one that knows the answer. The parse was previously implemented
//! three times, each with its own idea of what a corrupt file means. Both are
//! this module's job now.
//!
//! # Compatibility
//!
//! A server from an older build wrote a bare PID. That is a legal, meaningful
//! state, the PID is known and the port is not, rather than a corrupt file, so
//! it reads back as [`ServerHandshake::Starting`] instead of an error. Nothing
//! current produces that state.

use std::num::NonZeroU16;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Which server a handshake belongs to.
///
/// The CLI can run two independent servers at once, and they must not overwrite
/// each other's record. Every other per-server artifact was already namespaced
/// this way (`daemon.json` / `sidecar-daemon.json`, the lock, the log); the
/// handshake was not, which was survivable only while nothing read it for a
/// port. Once discovery started returning the PUBLISHED port, a shared slot
/// meant a sidecar could hand out its port as the main server's.
///
/// There is deliberately no `Default`, so a Rust caller cannot omit the fact.
/// The one place a value is assumed is `--handshake-slot`, which defaults to
/// `main` because a human running `serve start` by hand is the main server and
/// demanding the flag would be hostile. That default is written as
/// `HandshakeSlot::Main.as_arg()`, so it is a reference to this type rather
/// than a silent choice made elsewhere. A SPAWNED child is always told
/// explicitly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum HandshakeSlot {
    /// The primary server: `serve start`, and the main auto-daemon.
    // Pinned rather than left to clap's kebab-case derivation, so the name the
    // parser accepts and the name [`as_arg`](Self::as_arg) writes onto a
    // child's argv are visibly one decision. They were two, agreeing only by
    // convention: renaming a variant would have changed the parser and not the
    // writer, and the parent would spawn a child its own CLI rejects.
    #[value(name = "main")]
    Main,
    /// The secondary transcribe daemon, which runs its own Python.
    #[value(name = "sidecar")]
    Sidecar,
}

impl HandshakeSlot {
    /// Filename under the state directory.
    ///
    /// `Main` keeps `server.pid`, the name it has always had, so a NEW CLI can
    /// still find and stop a server started by an older build (its bare PID
    /// parses as [`ServerHandshake::Starting`]).
    ///
    /// The reverse direction is NOT covered, and the scope is worth stating
    /// rather than implying: an OLD CLI reading this file finds JSON where it
    /// expects an integer, calls it a corrupt PID file and deletes it, which
    /// removes a live server's record. That is only reachable by running a
    /// stale binary against a current server.
    fn filename(self) -> &'static str {
        match self {
            Self::Main => "server.pid",
            Self::Sidecar => "sidecar-server.pid",
        }
    }

    /// The value to pass on a child's command line.
    ///
    /// `const` so it can be the clap `default_value` too, making that a third
    /// reference to this table rather than a third copy of the string.
    pub const fn as_arg(self) -> &'static str {
        match self {
            Self::Main => "main",
            Self::Sidecar => "sidecar",
        }
    }
}

/// A port a server ACTUALLY bound.
///
/// The constructor is private to this module and takes the address of a
/// listener that has already bound, so a `BoundPort` cannot be manufactured
/// from a config value. That is the whole point of the type: a `u16` from
/// `server.yaml` and a `u16` from `local_addr()` mean different things, and
/// only the second one is a fact about a running server.
///
/// Non-zero by construction: a bound listener always has a real port, so the
/// "port 0" that means "choose for me" on the way in cannot come back out.
///
/// Deliberately NOT `Serialize`/`Deserialize`. A derived `Deserialize` would
/// have let `serde_json::from_str::<BoundPort>("8000")` succeed from anywhere,
/// which makes the sentence above false: the number would carry the type
/// without ever having come from a bind. The record on disk holds a plain
/// `NonZeroU16` and this module turns it into a `BoundPort` through
/// [`reported`](Self::reported), so both ways in are named and both are here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoundPort(NonZeroU16);

impl BoundPort {
    /// The port a listener reports for itself.
    ///
    /// Returns `None` only if the OS reported port 0 for a bound socket, which
    /// would mean the bind did not actually happen; callers treat that as a
    /// startup failure rather than publishing a handshake nobody can use.
    pub(crate) fn from_listener_addr(addr: std::net::SocketAddr) -> Option<Self> {
        NonZeroU16::new(addr.port()).map(Self)
    }

    /// The port a binder REPORTED in its own handshake record.
    ///
    /// Second-hand but not a guess: the only writer is a server that had
    /// already bound. Private to this module so the record is the only route
    /// in from disk.
    fn reported(port: NonZeroU16) -> Self {
        Self(port)
    }

    /// The port number, for building an address.
    pub fn get(self) -> u16 {
        self.0.get()
    }
}

impl std::fmt::Display for BoundPort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// The on-disk form. Private: callers go through [`ServerHandshake`], so the
/// difference between the JSON record and a legacy bare PID stays here.
#[derive(Serialize, Deserialize)]
struct HandshakeRecord {
    /// OS process ID of the server.
    pid: u32,
    /// The port the server bound.
    ///
    /// Not optional: only a server that has BOUND writes this record, so a
    /// record without a port is not a state that can occur. The port-less
    /// state still exists, but it is reached by reading a legacy bare-PID
    /// file, and it is [`ServerHandshake::Starting`].
    ///
    /// A plain `NonZeroU16` rather than a [`BoundPort`], so the observation
    /// type has no public deserializer. `parse` lifts it via
    /// [`BoundPort::reported`].
    port: NonZeroU16,
}

/// What the state directory says about the server process.
///
/// The two variants are different operator situations, not one situation with
/// a missing field: [`Starting`](Self::Starting) means a process exists but
/// has published no address yet, so a caller must wait or look elsewhere;
/// [`Listening`](Self::Listening) means the address is known and usable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerHandshake {
    /// A PID is known but no port is. Reached only by reading a bare-PID file
    /// from an older build: no current writer can produce one, which is why
    /// [`HandshakeRecord`]'s port is not optional.
    Starting {
        /// OS process ID of the spawned server.
        pid: u32,
    },
    /// The server bound and published the port it actually got.
    Listening {
        /// OS process ID of the running server.
        pid: u32,
        /// The port it is listening on.
        port: BoundPort,
    },
}

/// Why a server never published a port.
///
/// The error half of [`ServerHandshake::await_published`]. It is a named enum
/// rather than a bare `Option<BoundPort>` because the wait loop knows exactly
/// why it stopped, and collapsing "the process died" into "no port" threw that
/// away; the caller then re-probed the process to reconstruct it, which races,
/// so a server that timed out and then exited was reported as having exited
/// immediately.
///
/// It is the ERROR half specifically, rather than a third variant on a
/// success-or-failure enum, because both callers had already handled the
/// success case by the time they needed this and could only close the
/// impossible arm with `unreachable!("handled above")`: a panic, in a daemon
/// startup path, standing in for a fact the type can carry. Each also kept its
/// own copy of the message below, so the two could drift.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublishFailure {
    /// The process exited before publishing.
    Exited,
    /// The deadline passed while the process was still alive.
    TimedOut,
}

impl PublishFailure {
    /// Operator-facing description, phrased to follow a process identifier.
    ///
    /// THE owner of this wording.
    pub const fn reason(self) -> &'static str {
        match self {
            Self::Exited => "exited before reporting a listening port",
            Self::TimedOut => "did not report a listening port in time",
        }
    }
}

/// Failures reading or writing the handshake.
#[derive(Debug, thiserror::Error)]
pub enum HandshakeError {
    /// The file could not be read or written.
    #[error("server handshake I/O at {path}: {source}")]
    Io {
        /// The file involved.
        path: PathBuf,
        /// The underlying failure.
        #[source]
        source: std::io::Error,
    },
    /// The file exists but holds neither a bare PID nor a handshake record.
    ///
    /// Reported rather than swallowed: the previous readers deleted a file
    /// they could not parse and returned "no server", which silently discards
    /// the one clue about a server that may well still be running.
    #[error("server handshake at {path} is neither a PID nor a handshake record")]
    Unreadable {
        /// The file involved.
        path: PathBuf,
    },
}

impl ServerHandshake {
    /// Path of the handshake file for a state directory and slot.
    pub fn path_in(state_dir: &Path, slot: HandshakeSlot) -> PathBuf {
        state_dir.join(slot.filename())
    }

    /// The recorded process ID, whichever state the server is in.
    pub fn pid(self) -> u32 {
        match self {
            Self::Starting { pid } | Self::Listening { pid, .. } => pid,
        }
    }

    /// The published port, when the server has reported one.
    ///
    /// `None` is [`Starting`](Self::Starting), which is a real state and not a
    /// missing value: the caller should wait, or fall back to the configured
    /// port for an older server, and either way should say which it did.
    pub fn bound_port(self) -> Option<BoundPort> {
        match self {
            Self::Starting { .. } => None,
            Self::Listening { port, .. } => Some(port),
        }
    }

    /// The port a server has published for this slot, if any.
    ///
    /// The primitive six call sites were spelling out as
    /// `read(..)` -> `Ok(Some(h))` -> `h.bound_port()`. An unreadable or
    /// absent record and a record without a port all mean the same thing to
    /// every one of them: no port to use.
    pub fn published_port(state_dir: &Path, slot: HandshakeSlot) -> Option<BoundPort> {
        Self::read(state_dir, slot).ok().flatten()?.bound_port()
    }

    /// Wait for a freshly-spawned server to publish the port it bound.
    ///
    /// Lives here rather than in a CLI subcommand because it touches only this
    /// module's file, slot and parse; it was reached by a fully-qualified
    /// sideways call between two `cli` submodules that already depend on each
    /// other the other way.
    ///
    /// The DEADLINE is the caller's, not a constant here. Startup does config
    /// validation, host-facts detection, database migration and cache init
    /// before it binds, so this is the slow phase and it must carry the
    /// generous share of a spawn budget; the health check that usually follows
    /// is confirming a socket that already exists.
    pub async fn await_published(
        state_dir: &Path,
        slot: HandshakeSlot,
        pid: u32,
        deadline: std::time::Instant,
    ) -> Result<BoundPort, PublishFailure> {
        /// Fast enough that a sub-second bind is not rounded up to a poll tick.
        const POLL: std::time::Duration = std::time::Duration::from_millis(50);

        loop {
            // Only this process's own record counts: a stale handshake from a
            // previous server would otherwise read as instant success. An
            // unreadable record is transient here (a partial write), so it is
            // treated as "not yet" and the deadline decides.
            if let Some(port) = Self::published_port(state_dir, slot)
                && Self::read(state_dir, slot)
                    .ok()
                    .flatten()
                    .is_some_and(|h| h.pid() == pid)
            {
                return Ok(port);
            }
            if !crate::worker::pool::reaper::process_alive(pid) {
                // One last look: the child may have published and exited
                // between the poll above and this check.
                return match Self::published_port(state_dir, slot) {
                    Some(port) => Ok(port),
                    None => Err(PublishFailure::Exited),
                };
            }
            if std::time::Instant::now() >= deadline {
                return Err(PublishFailure::TimedOut);
            }
            tokio::time::sleep(POLL).await;
        }
    }

    /// Read the handshake, if a server has published one.
    ///
    /// `Ok(None)` means no file, which is the ordinary "no server here" case.
    /// A file that cannot be understood is an error rather than a `None`, so a
    /// caller cannot mistake "I could not read this" for "nothing is running".
    pub fn read(state_dir: &Path, slot: HandshakeSlot) -> Result<Option<Self>, HandshakeError> {
        let path = Self::path_in(state_dir, slot);
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(source) => return Err(HandshakeError::Io { path, source }),
        };
        Self::parse(text.trim(), &path).map(Some)
    }

    /// Interpret the file's bytes.
    fn parse(text: &str, path: &Path) -> Result<Self, HandshakeError> {
        // Legacy form: a bare PID, written by builds before the port was
        // published. A known PID with an unknown port is exactly `Starting`.
        if let Ok(pid) = text.parse::<u32>() {
            return Ok(Self::Starting { pid });
        }
        let record: HandshakeRecord =
            serde_json::from_str(text).map_err(|_| HandshakeError::Unreadable {
                path: path.to_path_buf(),
            })?;
        Ok(Self::Listening {
            pid: record.pid,
            port: BoundPort::reported(record.port),
        })
    }

    /// Record a server that has bound and is listening.
    ///
    /// Called by the server itself, after the bind succeeds, because it is the
    /// only process that knows the answer.
    pub fn publish_listening(
        state_dir: &Path,
        slot: HandshakeSlot,
        pid: u32,
        port: BoundPort,
    ) -> Result<(), HandshakeError> {
        Self::write(state_dir, slot, HandshakeRecord { pid, port: port.0 })
    }

    /// Atomically replace the handshake file.
    fn write(
        state_dir: &Path,
        slot: HandshakeSlot,
        record: HandshakeRecord,
    ) -> Result<(), HandshakeError> {
        let path = Self::path_in(state_dir, slot);
        let io = |source| HandshakeError::Io {
            path: path.clone(),
            source,
        };
        std::fs::create_dir_all(state_dir).map_err(io)?;
        // Temp-and-rename so a concurrent reader never sees a half-written
        // record. The temp file is per-PID so two servers racing to publish
        // cannot truncate each other's temp file mid-write.
        let tmp = path.with_extension(format!("tmp.{}", record.pid));
        let encoded = serde_json::to_string(&record).map_err(|error| HandshakeError::Io {
            path: path.clone(),
            source: std::io::Error::other(error),
        })?;
        std::fs::write(&tmp, encoded).map_err(io)?;
        std::fs::rename(&tmp, &path).map_err(io)?;
        Ok(())
    }

    /// Remove the handshake file. A missing file is not an error.
    pub fn remove(state_dir: &Path, slot: HandshakeSlot) -> Result<(), HandshakeError> {
        let path = Self::path_in(state_dir, slot);
        match std::fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(source) => Err(HandshakeError::Io { path, source }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir() -> tempfile::TempDir {
        tempfile::tempdir().expect("tempdir")
    }

    /// A `BoundPort` for a test, built the way the module itself can.
    ///
    /// In-module construction is legal and needs no public door; the crate's
    /// other tests get theirs from a real listener or from a published record.
    fn bound(port: u16) -> BoundPort {
        BoundPort(NonZeroU16::new(port).expect("test port must be non-zero"))
    }

    /// The two slots are different files, so one server cannot overwrite the
    /// other's record.
    ///
    /// This is the property that made the shared filename a defect once
    /// discovery began trusting the published port.
    #[test]
    fn the_two_slots_are_separate_files() {
        let d = dir();
        let main_port = bound(41001);
        let side_port = bound(41002);
        ServerHandshake::publish_listening(d.path(), HandshakeSlot::Main, 1, main_port)
            .expect("publish main");
        ServerHandshake::publish_listening(d.path(), HandshakeSlot::Sidecar, 2, side_port)
            .expect("publish sidecar");

        let main = ServerHandshake::read(d.path(), HandshakeSlot::Main)
            .expect("read main")
            .expect("present");
        let side = ServerHandshake::read(d.path(), HandshakeSlot::Sidecar)
            .expect("read sidecar")
            .expect("present");
        assert_eq!(main.bound_port(), Some(main_port));
        assert_eq!(side.bound_port(), Some(side_port));
        assert_eq!(main.pid(), 1);
        assert_eq!(side.pid(), 2);
    }

    /// Removing one slot leaves the other alone.
    #[test]
    fn removing_one_slot_leaves_the_other() {
        let d = dir();
        let port = bound(41003);
        ServerHandshake::publish_listening(d.path(), HandshakeSlot::Main, 1, port)
            .expect("publish main");
        ServerHandshake::publish_listening(d.path(), HandshakeSlot::Sidecar, 2, port)
            .expect("publish sidecar");
        ServerHandshake::remove(d.path(), HandshakeSlot::Main).expect("remove main");

        assert_eq!(
            ServerHandshake::read(d.path(), HandshakeSlot::Main).expect("read main"),
            None
        );
        assert!(
            ServerHandshake::read(d.path(), HandshakeSlot::Sidecar)
                .expect("read sidecar")
                .is_some(),
            "the sidecar record must survive removing the main one"
        );
    }

    /// Absent file is "no server", not an error.
    #[test]
    fn absent_handshake_reads_as_none() {
        let d = dir();
        assert_eq!(
            ServerHandshake::read(d.path(), HandshakeSlot::Main).expect("read"),
            None
        );
    }

    /// A listening server round-trips through the file with its port intact.
    #[test]
    fn a_listening_server_round_trips() {
        let d = dir();
        let port = bound(54321);
        ServerHandshake::publish_listening(d.path(), HandshakeSlot::Main, 4242, port)
            .expect("publish");
        let read = ServerHandshake::read(d.path(), HandshakeSlot::Main)
            .expect("read")
            .expect("present");
        assert_eq!(read, ServerHandshake::Listening { pid: 4242, port });
        assert_eq!(read.bound_port(), Some(port));
        assert_eq!(read.pid(), 4242);
    }

    /// A bare PID from an older build is a legal state, not a parse failure.
    ///
    /// This is the compatibility case that matters on a fleet mid-deploy: an
    /// old server is running and a new CLI must still find and stop it.
    #[test]
    fn a_legacy_bare_pid_reads_as_starting() {
        let d = dir();
        std::fs::write(
            ServerHandshake::path_in(d.path(), HandshakeSlot::Main),
            "1234\n",
        )
        .expect("write legacy");
        let read = ServerHandshake::read(d.path(), HandshakeSlot::Main)
            .expect("read")
            .expect("present");
        assert_eq!(read, ServerHandshake::Starting { pid: 1234 });
    }

    /// Unreadable content is an error, not a silent "nothing is running".
    ///
    /// The readers this replaces deleted the file and returned "no server",
    /// which throws away the only evidence that a server may still be up and
    /// invites a second one onto the same port.
    #[test]
    fn unreadable_content_is_reported_not_swallowed() {
        let d = dir();
        std::fs::write(
            ServerHandshake::path_in(d.path(), HandshakeSlot::Main),
            "not a pid",
        )
        .expect("write junk");
        let error =
            ServerHandshake::read(d.path(), HandshakeSlot::Main).expect_err("should not parse");
        assert!(matches!(error, HandshakeError::Unreadable { .. }));
        assert!(
            ServerHandshake::path_in(d.path(), HandshakeSlot::Main).exists(),
            "a file we could not parse must survive for an operator to inspect"
        );
    }

    /// A port the OS never assigned cannot become a `BoundPort`.
    #[test]
    fn port_zero_is_not_a_bound_port() {
        let addr: std::net::SocketAddr = "127.0.0.1:0".parse().expect("addr");
        assert_eq!(BoundPort::from_listener_addr(addr), None);
    }

    /// A real listener yields the port it actually got.
    #[test]
    fn a_real_listener_yields_its_port() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("local addr");
        let bound = BoundPort::from_listener_addr(addr).expect("a bound listener has a port");
        assert_eq!(bound.get(), addr.port());
    }
}
