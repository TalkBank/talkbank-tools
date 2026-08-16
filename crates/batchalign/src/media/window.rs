//! A window of a media file, which cannot be empty.
//!
//! Its own module because it is a MEDIA primitive, not a transcode detail: the
//! CHAT-analysis side produces windows (`chat_ops::fa::find_untimed_windows`)
//! and the transcode side consumes them, so putting it under either would make
//! the other depend on a neighbour it has no business knowing.
//!
//! # Why it exists
//!
//! The same `end <= start` comparison was written in THREE places, none of them
//! where the window originates: `validate_fa_infer_item` checked it, then
//! `extract_prepared_audio_segment_f32le` checked it again on the same numbers,
//! and `extract_audio_segment` checked it a third time for the UTR path. The
//! producer, `find_untimed_windows`, returned bare `(u64, u64)` tuples and
//! checked nothing, while being the one place that could actually build an
//! inverted one.

use std::ffi::OsString;

use crate::time::FileMs;

/// A non-empty half-open window of a source file.
///
/// Constructible only when `end` follows `start`, so a caller holding one has
/// already proved the window can contain audio. `artifacts_v2` used to check
/// this inline and report the failure as `io::ErrorKind::InvalidInput`, which
/// described an invalid ARGUMENT as a failure of the filesystem.
/// The bounds are [`FileMs`] because they are POSITIONS, not lengths; see
/// [`crate::time`] for why that distinction has its own module.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MediaWindow {
    start: FileMs,
    end: FileMs,
}

/// A window whose end does not follow its start.
#[derive(Debug, thiserror::Error)]
#[error("media window end {end} must be greater than start {start}")]
pub struct EmptyWindow {
    /// Requested start.
    pub start: FileMs,
    /// Requested end.
    pub end: FileMs,
}

impl MediaWindow {
    /// The window from `start` to `end`, or [`EmptyWindow`] if it holds nothing.
    pub fn new(start: FileMs, end: FileMs) -> Result<Self, EmptyWindow> {
        if end <= start {
            return Err(EmptyWindow { start, end });
        }
        Ok(Self { start, end })
    }

    /// Start, as the position it is.
    ///
    /// There is no `start_ms() -> u64` beside this. There was, and adding
    /// `start()` without removing it meant the change added two accessors and
    /// removed nothing, leaving the untyped route out with the shorter name so
    /// a new call site would reach for it. Callers that genuinely need the
    /// integer (a cache key, a `Display`, tracing fields) spell the lowering
    /// themselves with `.get()`, which is one visible step rather than a second
    /// sanctioned way to ask the same question.
    #[must_use]
    pub fn start(self) -> FileMs {
        self.start
    }

    /// End, as the position it is.
    #[must_use]
    pub fn end(self) -> FileMs {
        self.end
    }

    /// `ffmpeg`'s seconds-with-milliseconds spelling of this window.
    pub(in crate::media) fn as_seek_args(self) -> [OsString; 4] {
        [
            OsString::from("-ss"),
            OsString::from(format!("{:.3}", self.start.get() as f64 / 1000.0)),
            OsString::from("-to"),
            OsString::from(format!("{:.3}", self.end.get() as f64 / 1000.0)),
        ]
    }
}

/// A media window that yielded no audio.
///
/// `ffmpeg` exits 0 and writes zero bytes when the requested window falls past
/// the end of the source, so this is the one failure a SUCCESSFUL exit reports,
/// and it has to travel: the data plane detects it, the request builder turns
/// it into a skip, and the transport layer acts on it three layers up.
///
/// # Why it is a type
///
/// It was `{ path: String, start_ms, end_ms }` declared in THREE error enums,
/// with a field-by-field rebuild at each layer boundary whose only job was to
/// move three values between structurally identical variants. The two loose
/// integers also disagreed about their own type: `u64` in the two worker enums
/// and `DurationMs` in `ServerError`, converted in passing by one of the
/// rebuilds. Carrying the [`MediaWindow`] that was ASKED FOR removes both
/// problems: one declaration, and the window keeps the type it was proven at.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EmptySegment {
    /// Source media the window was requested from.
    pub path: String,
    /// The window that produced nothing.
    pub window: MediaWindow,
}

impl std::fmt::Display for EmptySegment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{}ms..{}ms) in {}",
            self.window.start().get(),
            self.window.end().get(),
            self.path
        )
    }
}
