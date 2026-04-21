//! Golden-XML parity harness: Rust CHAT → XML vs Java Chatter output.
//!
//! For each `.xml` file under `corpus/reference-xml/` we expect a sibling
//! `.cha` file at the matching path under `corpus/reference/`. The test:
//!
//! 1. Loads the CHAT source.
//! 2. Parses + validates it with the production pipeline.
//! 3. Emits XML via `talkbank_transform::xml::write_chat_xml`.
//! 4. Structurally compares the emission to the golden XML.
//!
//! Comparison is **structural**, not byte-for-byte: whitespace between
//! elements is normalized, attribute ordering is normalized, and
//! CDATA/text content is compared after trimming surrounding whitespace.
//! That matches what external XML consumers (NLTK's ElementTree,
//! childes-db's `xml.etree.ElementTree`, Phon's StAX reader) actually
//! observe; byte-exact comparison would over-constrain the emitter for
//! no downstream benefit.
//!
//! All tests are expected to be RED while the emitter is being built up
//! stage by stage. Each stage flips a subset GREEN. This is deliberate
//! per the TDD policy (`CLAUDE.md §12`).
//!
//! # Running
//!
//! ```bash
//! cargo nextest run -p talkbank-parser-tests --test xml_golden
//! ```

use std::path::{Path, PathBuf};

use rstest::rstest;
use talkbank_model::ParseValidateOptions;
use talkbank_transform::parse_and_validate;
use talkbank_transform::xml::write_chat_xml;

mod xml_support;
use xml_support::{XmlStructuralDiff, assert_xml_structurally_equal};

/// Parameterised golden test, one invocation per `*.xml` in `corpus/reference-xml/`.
///
/// Intentionally named `xml_golden_parity` so the test output surfaces
/// the parameter file path next to the intent.
#[rstest]
fn xml_golden_parity(#[files("../../corpus/reference-xml/**/*.xml")] golden_path: PathBuf) {
    let cha_path = paired_chat_path(&golden_path);

    let cha_source = match std::fs::read_to_string(&cha_path) {
        Ok(s) => s,
        Err(e) => panic!(
            "failed to read paired CHAT source {}: {e}",
            cha_path.display()
        ),
    };
    let golden_xml = match std::fs::read_to_string(&golden_path) {
        Ok(s) => s,
        Err(e) => panic!("failed to read golden XML {}: {e}", golden_path.display()),
    };

    // Parse with validation. A parse/validation failure on a reference
    // corpus file is itself a regression — surface it directly rather
    // than muddling it with XML-emitter diagnostics.
    let chat_file = match parse_and_validate(
        &cha_source,
        ParseValidateOptions::default().with_validation(),
    ) {
        Ok(file) => file,
        Err(e) => panic!(
            "parse_and_validate failed for paired CHAT {}: {e}",
            cha_path.display()
        ),
    };

    // Emit XML. While the port is in progress this will commonly error
    // with `FeatureNotImplemented`. We fail the test with that error
    // text visible so the harness doubles as a coverage dashboard.
    let emitted = match write_chat_xml(&chat_file) {
        Ok(xml) => xml,
        Err(e) => panic!(
            "write_chat_xml refused to emit for {}: {e}",
            cha_path.display()
        ),
    };

    if let Err(diff) = assert_xml_structurally_equal(&emitted, &golden_xml) {
        panic!(
            "XML structural mismatch for {}\n--- diff ---\n{diff}",
            golden_path.display(),
        );
    }
    // Silence dead-code warning if the diff type becomes unused.
    let _ = std::marker::PhantomData::<XmlStructuralDiff>;
}

/// Map `corpus/reference-xml/<rel>.xml` → `corpus/reference/<rel>.cha`.
///
/// The two trees mirror each other: Franklin generated the XML tree by
/// running Java Chatter over the CHAT reference tree, preserving the
/// relative paths.
fn paired_chat_path(golden_xml_path: &Path) -> PathBuf {
    // Replace the last "reference-xml" path component with "reference".
    // Swap the extension.
    let mut out = PathBuf::new();
    let mut swapped = false;
    for component in golden_xml_path.components() {
        let os = component.as_os_str();
        if !swapped && os == "reference-xml" {
            out.push("reference");
            swapped = true;
        } else {
            out.push(os);
        }
    }
    if !swapped {
        // Defensive: if the layout ever changes, loud failure beats
        // silently pointing at the wrong file.
        panic!(
            "golden XML path does not contain a 'reference-xml' component: {}",
            golden_xml_path.display()
        );
    }
    out.set_extension("cha");
    out
}
