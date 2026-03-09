//! Wxxx: Warnings
//!
//! Generates warning corpus files for various warning codes.

use std::fs;
use std::path::Path;

use crate::ChatFileBuilder;

use super::{GenResult, write_file};

//
// Wxxx: Warnings (5 missing)
//

/// Generates wxxx warnings.
pub fn generate_wxxx_warnings(root: &Path) -> GenResult {
    let dir = root.join("warnings");
    fs::create_dir_all(&dir)?;

    let mut count = 0;

    // W108: SpeakerNotFoundInParticipants
    write_file(
        &dir.join("W108_speaker_not_in_participants.cha"),
        "@UTF8\n@Begin\n@Languages:\teng\n\
         @Participants:\tCHI Child\n\
         @ID:\teng|corpus|CHI|||||Child|||\n\
         *MOT:\thello .\n\
         @End\n"
            .to_string(),
    )?;
    count += 1;

    // W210: MissingWhitespaceBeforeContent
    write_file(
        &dir.join("W210_missing_whitespace_before.cha"),
        ChatFileBuilder::new()
            .speaker("CHI", "Target_Child")
            .utterance("CHI", "hello.")
            .build(),
    )?;
    count += 1;

    // W211: MissingWhitespaceAfterOverlap
    write_file(
        &dir.join("W211_missing_whitespace_after_overlap.cha"),
        ChatFileBuilder::new()
            .speaker("CHI", "Target_Child")
            .utterance("CHI", "hello \u{2308}world\u{2309}.")
            .build(),
    )?;
    count += 1;

    // W724: GraRootHeadNotSelf
    write_file(
        &dir.join("W724_gra_root_head_not_self.cha"),
        ChatFileBuilder::new()
            .speaker("CHI", "Target_Child")
            .custom_header("@Comment:\tExpected warning: W724 (GRA ROOT head not self)")
            .custom_header(
                "@Comment:\tTrigger: ROOT relation where head index does not point to self",
            )
            .utterance("CHI", "I want .")
            .dependent_tier("mor", "pro|I v|want .")
            .dependent_tier("gra", "1|2|SUBJ 2|1|ROOT .")
            .build(),
    )?;
    count += 1;

    // W999: LegacyWarning
    write_file(
        &dir.join("W999_legacy_warning.cha"),
        ChatFileBuilder::new()
            .speaker("CHI", "Target_Child")
            .custom_header("@OldHeader:\tDeprecated")
            .utterance("CHI", "hello .")
            .build(),
    )?;
    count += 1;

    Ok(count)
}
