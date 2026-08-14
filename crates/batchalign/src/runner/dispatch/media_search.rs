//! Where forced alignment looks for a transcript's recording.
//!
//! The corpus root and the media root can differ on one host, and differ again
//! between deployment modes, so resolution tries several ROOTS. The name is the
//! same in every one of them: the transcript's stem, which valid CHAT requires
//! to equal the `@Media` basename (chatter reports a mismatch as an error).
//!
//! **Looking IS recording, and that is the point of this module.** The rungs
//! used to be a chain of `if original_audio_path.is_none()` blocks with a
//! `searched.push(format!(...))` beside each, which is two things a human keeps
//! in step. They did not stay in step: one rung searched and recorded nothing,
//! so the failure message omitted the last place it had looked, while the
//! comment above the chain asserted that could not happen. Before that, the
//! message reconstructed the list from the same inputs that drove the search,
//! in a second if/else that could name only ONE place however many had run.
//!
//! [`MediaSearch::try_place`] is the only way to search, and it takes the place
//! by value, so a rung that looks somewhere has recorded it by construction.
//! There is no `push`, and nothing to forget.

use std::fmt;
use std::path::{Path, PathBuf};

use crate::media::MediaExtensions;

/// One place resolution looked.
///
/// A variant per rung rather than a rendered sentence. A `Vec<String>` of
/// prose can be printed and nothing else: no test can ask whether every rung
/// recorded itself, which is exactly the property that broke. These can be
/// compared, counted and asserted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SearchedPlace {
    /// An explicit `--media-dir` root, with the corpus subdir mapped under it.
    MediaDirRoot { root: PathBuf, subdir: String },
    /// Beside the transcript on the execution host, optionally preceded by
    /// `--media-dir` itself.
    TranscriptAdjacent {
        transcript: PathBuf,
        media_dir: Option<PathBuf>,
    },
    /// The client's shared source directory.
    ClientSourceDir {
        source_dir: String,
        transcript: PathBuf,
        media_dir: Option<PathBuf>,
    },
    /// A media mapping configured on this host.
    LocalMediaMapping {
        key: String,
        root: PathBuf,
        subdir: String,
    },
    /// A media mapping inferred from the client's source path.
    InferredMediaMapping { dir: PathBuf },
    /// One of the server's configured media roots.
    ServerMediaRoot { root: PathBuf },
}

impl SearchedPlace {
    /// Every directory this place means, in the order to try them.
    ///
    /// The whole difference between the rungs lives here. Two of them are
    /// "`--media-dir` first, then beside the file", which is what
    /// `resolve_audio_for_chat_with_media_dir` did; the rest are a single
    /// root, optionally with a subdir joined under it.
    fn directories(&self) -> Vec<PathBuf> {
        // A subdir joined only when non-empty, matching what the previous
        // `find_media_in_root` did: an empty subdir means the root itself.
        fn under(root: &Path, subdir: &str) -> PathBuf {
            if subdir.is_empty() {
                root.to_path_buf()
            } else {
                root.join(subdir)
            }
        }
        // `--media-dir` is consulted before the file's own directory, which is
        // the order the previous helper used.
        fn beside(media_dir: Option<&PathBuf>, transcript: &Path) -> Vec<PathBuf> {
            media_dir
                .cloned()
                .into_iter()
                .chain(transcript.parent().map(Path::to_path_buf))
                .collect()
        }
        match self {
            Self::MediaDirRoot { root, subdir } => vec![under(root, subdir)],
            Self::TranscriptAdjacent {
                transcript,
                media_dir,
            } => beside(media_dir.as_ref(), transcript),
            Self::ClientSourceDir {
                transcript,
                media_dir,
                ..
            } => beside(media_dir.as_ref(), transcript),
            Self::LocalMediaMapping { root, subdir, .. } => vec![under(root, subdir)],
            Self::InferredMediaMapping { dir } => vec![dir.clone()],
            Self::ServerMediaRoot { root } => vec![root.clone()],
        }
    }
}

impl fmt::Display for SearchedPlace {
    /// The operator-facing sentence, derived from the value rather than
    /// written at the call site, so it cannot describe a place that was not
    /// actually tried.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MediaDirRoot { root, subdir } => {
                write!(f, "--media-dir root '{}' subdir '{subdir}'", root.display())
            }
            Self::TranscriptAdjacent {
                transcript,
                media_dir: Some(dir),
            } => write!(
                f,
                "--media-dir '{}' and alongside '{}'",
                dir.display(),
                transcript.display()
            ),
            Self::TranscriptAdjacent {
                transcript,
                media_dir: None,
            } => write!(f, "alongside '{}'", transcript.display()),
            Self::ClientSourceDir { source_dir, .. } => {
                write!(f, "shared source directory '{source_dir}'")
            }
            Self::LocalMediaMapping { key, root, subdir } => write!(
                f,
                "local media mapping '{key}' root '{}' subdir '{subdir}'",
                root.display()
            ),
            Self::InferredMediaMapping { dir } => {
                write!(f, "inferred media mapping '{}'", dir.display())
            }
            Self::ServerMediaRoot { root } => {
                write!(f, "server media root '{}'", root.display())
            }
        }
    }
}

/// A search in progress: the stem being looked for, and everywhere looked.
pub(crate) struct MediaSearch {
    stem: String,
    searched: Vec<SearchedPlace>,
}

impl MediaSearch {
    pub(crate) fn for_stem(stem: impl Into<String>) -> Self {
        Self {
            stem: stem.into(),
            searched: Vec::new(),
        }
    }

    /// Search one place, recording it.
    ///
    /// The ONLY search primitive. It takes the place by value and records it
    /// before looking, so "every place searched appears in the report" is true
    /// by construction rather than by everyone remembering a `push`.
    ///
    /// A place already tried is skipped rather than searched again. That is
    /// only expressible because places are values: in paths_mode with no
    /// `--media-dir`, the adjacency rung and the final fallback are the same
    /// directory, and the old chain probed it twice under nine extensions.
    pub(crate) async fn try_place(&mut self, place: SearchedPlace) -> Option<PathBuf> {
        if self.searched.contains(&place) {
            return None;
        }
        let directories = place.directories();
        self.searched.push(place);
        for directory in directories {
            if let Some(found) = MediaExtensions::find_in(&directory, &self.stem).await {
                return Some(found);
            }
        }
        None
    }

    /// Everywhere this search looked.
    ///
    /// Test-only, and that is the point rather than an omission: production
    /// wants the rendered sentence, while a test wants to assert that a rung
    /// RECORDED itself. The old `Vec<String>` could not answer the second
    /// question at all, which is why one rung searched silently for as long as
    /// it did.
    #[cfg(test)]
    pub(crate) fn places(&self) -> &[SearchedPlace] {
        &self.searched
    }

    /// The rendered list, joined for an error message.
    pub(crate) fn describe(&self) -> String {
        self.searched
            .iter()
            .map(SearchedPlace::to_string)
            .collect::<Vec<_>>()
            .join("; ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(path: &Path) {
        std::fs::create_dir_all(path.parent().expect("has a parent")).expect("mkdir");
        std::fs::write(path, b"media").expect("write");
    }

    #[tokio::test]
    async fn a_place_that_is_searched_is_recorded() {
        // The property the previous shape could not hold: one rung searched
        // and recorded nothing, and no test could notice, because the record
        // was a `Vec<String>` a caller had to remember to push to.
        let dir = tempfile::tempdir().expect("tempdir");
        let mut search = MediaSearch::for_stem("absent");
        let place = SearchedPlace::ServerMediaRoot {
            root: dir.path().to_path_buf(),
        };
        assert_eq!(search.try_place(place.clone()).await, None);
        assert_eq!(search.places(), [place]);
    }

    #[tokio::test]
    async fn a_repeated_place_is_not_searched_twice() {
        let dir = tempfile::tempdir().expect("tempdir");
        let transcript = dir.path().join("talk.cha");
        write(&transcript);
        let place = SearchedPlace::TranscriptAdjacent {
            transcript: transcript.clone(),
            media_dir: None,
        };
        let mut search = MediaSearch::for_stem("talk");
        assert!(search.try_place(place.clone()).await.is_none());
        assert!(search.try_place(place).await.is_none());
        assert_eq!(search.places().len(), 1, "the second attempt was skipped");
    }

    #[tokio::test]
    async fn adjacency_finds_media_beside_the_transcript() {
        // Covers what `resolve_audio_for_chat` used to test before it became
        // dead: the commonest case, a recording sitting next to its transcript.
        let dir = tempfile::tempdir().expect("tempdir");
        write(&dir.path().join("talk.wav"));
        let mut search = MediaSearch::for_stem("talk");
        let found = search
            .try_place(SearchedPlace::TranscriptAdjacent {
                transcript: dir.path().join("talk.cha"),
                media_dir: None,
            })
            .await;
        assert_eq!(found, Some(dir.path().join("talk.wav")));
    }

    #[tokio::test]
    async fn a_server_media_root_is_searched_flat() {
        // The media_roots fallback, likewise.
        let root = tempfile::tempdir().expect("tempdir");
        write(&root.path().join("ACWT01a.wav"));
        let mut search = MediaSearch::for_stem("ACWT01a");
        let found = search
            .try_place(SearchedPlace::ServerMediaRoot {
                root: root.path().to_path_buf(),
            })
            .await;
        assert_eq!(found, Some(root.path().join("ACWT01a.wav")));
    }

    #[tokio::test]
    async fn media_dir_is_tried_before_the_transcript_directory() {
        let media = tempfile::tempdir().expect("tempdir");
        let corpus = tempfile::tempdir().expect("tempdir");
        write(&media.path().join("talk.mp3"));
        write(&corpus.path().join("talk.wav"));

        let mut search = MediaSearch::for_stem("talk");
        let found = search
            .try_place(SearchedPlace::TranscriptAdjacent {
                transcript: corpus.path().join("talk.cha"),
                media_dir: Some(media.path().to_path_buf()),
            })
            .await;

        assert_eq!(found, Some(media.path().join("talk.mp3")));
    }

    #[tokio::test]
    async fn a_subdir_is_joined_under_the_root() {
        let root = tempfile::tempdir().expect("tempdir");
        write(&root.path().join("French/Newcastle/d01.mp3"));
        let mut search = MediaSearch::for_stem("d01");
        let found = search
            .try_place(SearchedPlace::MediaDirRoot {
                root: root.path().to_path_buf(),
                subdir: "French/Newcastle".to_owned(),
            })
            .await;
        assert_eq!(found, Some(root.path().join("French/Newcastle/d01.mp3")));
    }

    #[test]
    fn every_place_renders_a_distinct_sentence() {
        // The message is derived from the value, so a place cannot describe
        // itself as somewhere it did not look.
        let places = [
            SearchedPlace::MediaDirRoot {
                root: PathBuf::from("/m"),
                subdir: "s".to_owned(),
            },
            SearchedPlace::TranscriptAdjacent {
                transcript: PathBuf::from("/c/talk.cha"),
                media_dir: None,
            },
            SearchedPlace::ServerMediaRoot {
                root: PathBuf::from("/r"),
            },
        ];
        let rendered: Vec<String> = places.iter().map(SearchedPlace::to_string).collect();
        assert_eq!(rendered.len(), 3);
        assert!(rendered.iter().all(|text| !text.is_empty()));
        assert_eq!(
            rendered[1], "alongside '/c/talk.cha'",
            "adjacency without --media-dir names only the transcript"
        );
    }
}
