//! Which file extensions name media batchalign can consume.
//!
//! One owner, and the verb that goes with it. Split out of `media.rs`
//! because that file had grown past the workspace's 400-line guidance and
//! the module already keeps one concept per file (`probe`, `tools`,
//! `transcode`, `window`).

use std::path::{Path, PathBuf};

/// The media formats batchalign can consume, and the two questions asked of
/// them.
///
/// Adding an extension here makes it discoverable via `resolve()`,
/// `list_files()` and the `/media/list` endpoint, AND resolvable by the forced
/// alignment pipeline, because both now ask this type rather than each keeping
/// a list.
///
/// **This used to be two lists, and they disagreed.** A dotted one lived here
/// and an undotted `KNOWN_MEDIA_EXTENSIONS` lived in `runner::util`, and the
/// runner's carried `wma` and `webm` while this one did not. The consequence
/// was reachable: a `.webm` recording could be resolved by the pipeline and
/// transcoded by `ensure_wav` (whose `FORCED_CONVERSION` lists it), yet was
/// invisible to the media walk, so it never appeared in `/media/list`. The doc
/// comment here asserted the opposite the whole time, claiming the list
/// "intentionally mirrors the formats that the engines can consume".
///
/// The dot was the tell. An extension is a bare token; the leading dot belongs
/// to how a FILENAME is spelled, not to what the format is, so storing it
/// dotted made a second representation whose only job was to suit one
/// `ends_with` call. Two of the three callers here had to re-add the dot
/// themselves to compare.
pub struct MediaExtensions;

impl MediaExtensions {
    /// Deliberately PRIVATE. The two questions below are the whole interface;
    /// a public list is an invitation to write a fourth `for ext in ...` loop
    /// building `{stem}.{ext}`, which is how the duplication arose.
    const ALL: &'static [&'static str] = &[
        "wav", "mp3", "mp4", "m4a", "flac", "ogg", "aac", "wma", "webm",
    ];

    /// Whether this filename names a media file batchalign can consume.
    pub fn matches(filename: &str) -> bool {
        Path::new(filename)
            .extension()
            .is_some_and(|extension| Self::is_known(&extension.to_string_lossy()))
    }

    /// Whether this bare extension, with or without a leading dot, is known.
    pub fn is_known(extension: &str) -> bool {
        let bare = extension.trim_start_matches('.').to_lowercase();
        Self::ALL.contains(&bare.as_str())
    }

    /// Every filename this stem could have, in a stable order.
    ///
    /// The order is the declaration order, which puts the formats the corpus
    /// actually uses first: measured over the kept corpus, every one of the
    /// 73,092 resolvable recordings is `mp3`, `mp4` or `wav`.
    pub fn candidates(stem: &str) -> impl Iterator<Item = String> {
        let stem = stem.to_owned();
        Self::ALL
            .iter()
            .map(move |extension| format!("{stem}.{extension}"))
    }

    /// The media file for `stem` in `dir`, if one is there.
    ///
    /// **This is the verb, and it was written out four times**: in
    /// `resolve_audio_for_chat_with_media_dir` (twice), in
    /// `staging::prepare::resolve_adjacent_media`, and in `fa_pipeline`'s
    /// `find_media_in_root`. Each was the same `for ext in LIST { join, then
    /// try_exists }` loop, and each therefore had to be found and edited
    /// separately whenever the list changed, which is how the list came to have
    /// two versions. A public extension list invites the fifth copy; this
    /// function is what callers actually wanted.
    pub async fn find_in(dir: &Path, stem: &str) -> Option<PathBuf> {
        for name in Self::candidates(stem) {
            let candidate = dir.join(name);
            if tokio::fs::try_exists(&candidate).await.unwrap_or(false) {
                return Some(candidate);
            }
        }
        None
    }

    /// [`find_in`](Self::find_in) for callers that are not async.
    ///
    /// `MediaResolver` is sync, so it could not call the async spelling and
    /// wrote the loop out again from `candidates` instead: a fifth copy, in the
    /// file that owns the verb, twenty lines below it. Making the list private
    /// did not prevent that, because `candidates` hands out four fifths of the
    /// same thing. The verb needs both spellings, or the sync caller has no
    /// choice.
    pub fn find_in_blocking(dir: &Path, stem: &str) -> Option<PathBuf> {
        Self::candidates(stem)
            .map(|name| dir.join(name))
            .find(|candidate| candidate.is_file())
    }
}
