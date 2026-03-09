//! ChatFile-level validation entry points and orchestration.
//!
//! References:
//! - <https://talkbank.org/0info/manuals/CHAT.html#File_Headers>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Main_Line>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Dependent_Tiers>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Media_Header>

use std::collections::HashSet;

use super::ChatFile;
use crate::validation::{Validate, ValidationConfig, ValidationState};
use crate::{ConfigurableErrorSink, ErrorSink, ParseError};
use crate::{Header, Line};

/// Build file-level validation context from headers and participant IDs.
///
/// Header-derived settings (languages/options) are computed once and shared
/// across header, utterance, and cross-utterance validators.
/// This prevents repeated header scanning in downstream validation passes.
fn build_validation_context(
    participant_ids: HashSet<crate::model::SpeakerCode>,
    languages: &crate::model::LanguageCodes,
    headers: &[&Header],
    config: ValidationConfig,
) -> crate::validation::ValidationContext {
    let declared_languages = languages.as_slice();
    let default_language = declared_languages.first();

    let ca_mode = file_uses_ca_mode(headers);
    let bullets_mode = file_uses_bullets_mode(headers);

    crate::validation::ValidationContext::from_shared(std::sync::Arc::new(
        crate::validation::SharedValidationData {
            participant_ids,
            default_language: default_language.cloned(),
            declared_languages: declared_languages.to_vec(),
            ca_mode,
            enable_quotation_validation: false, // Disabled by default
            bullets_mode,
            config,
        },
    ))
}

impl<S: ValidationState> ChatFile<S> {
    /// Run header-only validation and return the derived context.
    ///
    /// Useful for callers that need validated header-derived configuration
    /// before running utterance-level checks.
    pub fn validate_headers_only(
        &self,
        errors: &impl ErrorSink,
        filename: Option<&str>,
    ) -> crate::validation::ValidationContext {
        use crate::validation::header;

        let headers_with_spans: Vec<(&Header, crate::Span)> = self.headers_with_spans().collect();
        let headers: Vec<&Header> = headers_with_spans.iter().map(|(h, _)| *h).collect();

        // Extract participant IDs from parsed participant map.
        let participant_ids: HashSet<crate::model::SpeakerCode> =
            self.participants.keys().cloned().collect();

        // Validate header-set invariants (duplicates, required headers).
        let source_len = self.lines.last().map(|l| l.span().end as usize);
        header::structure::check_headers(&headers_with_spans, errors, source_len);

        let context = build_validation_context(
            participant_ids,
            &self.languages,
            &headers,
            ValidationConfig::new(),
        );

        // Validate each header payload.
        for (header, span) in &headers_with_spans {
            header::check_header(header, *span, &context, errors);
        }

        // E531: Validate media filename matches file name (if provided)
        if let Some(file_name) = filename {
            check_media_filename_match(&headers_with_spans, file_name, errors);
        }

        context
    }

    /// Run tier alignment checks on all utterances, respecting ParseHealth flags.
    ///
    /// Returns any alignment errors found (count mismatches between tiers).
    /// Tainted tiers (from lenient parse error recovery) are skipped to
    /// prevent false positives on pre-existing data quality issues.
    ///
    /// This is a lightweight check intended for use as a pre-serialization gate:
    /// it catches corrupted output (e.g. mismatched %mor/%gra counts) without
    /// running full file-level validation.
    pub fn validate_alignments(&self) -> Vec<ParseError> {
        use crate::alignment::{
            align_main_to_mor, align_main_to_pho, align_main_to_sin, align_main_to_wor,
            align_mor_to_gra,
        };
        use crate::model::file::utterance::ParseHealth;

        let mut errors = Vec::new();

        for utt in self.utterances() {
            let health = utt.parse_health.unwrap_or(ParseHealth::default());

            // Main → %mor alignment
            if health.can_align_main_to_mor()
                && let Some(mor) = utt.mor_tier()
            {
                let alignment = align_main_to_mor(&utt.main, mor);
                errors.extend(alignment.errors);
            }

            // %mor → %gra alignment
            if health.can_align_mor_to_gra()
                && let (Some(mor), Some(gra)) = (utt.mor_tier(), utt.gra_tier())
            {
                let alignment = align_mor_to_gra(mor, gra);
                errors.extend(alignment.errors);
            }

            // Main → %wor alignment
            if health.can_align_main_to_wor()
                && let Some(wor) = utt.wor_tier()
            {
                let alignment = align_main_to_wor(&utt.main, wor);
                errors.extend(alignment.errors);
            }

            // Main → %pho alignment
            if health.can_align_main_to_pho()
                && let Some(pho) = utt.pho_tier()
            {
                let alignment = align_main_to_pho(&utt.main, pho);
                errors.extend(alignment.errors);
            }

            // Main → %sin alignment
            if health.can_align_main_to_sin()
                && let Some(sin) = utt.sin_tier()
            {
                let alignment = align_main_to_sin(&utt.main, sin);
                errors.extend(alignment.errors);
            }
        }

        errors
    }

    /// Validate this CHAT file with streaming error output.
    ///
    /// Errors are reported to the `errors` sink as they're discovered, enabling:
    /// - Early cancellation when user has seen enough errors
    /// - Real-time error display in GUI applications
    /// - Memory-efficient processing of large files
    ///
    /// # Parameters
    ///
    /// * `errors` - Error sink for streaming validation errors
    /// * `filename` - Optional filename (without extension) for E531 validation
    ///
    /// # Example
    ///
    /// ```ignore
    /// use talkbank_model::{ChatFile, ErrorCollector, ErrorSink};
    ///
    /// let sink = ErrorCollector::new();
    /// chat_file.validate(&sink, Some("myfile"));
    /// let errors = sink.into_vec();
    /// ```
    #[tracing::instrument(skip(self, errors), fields(lines = self.lines.len()))]
    pub fn validate(&self, errors: &impl crate::ErrorSink, filename: Option<&str>) {
        use crate::validation::{cross_utterance, header};

        let header_count = self.header_count();
        let utterance_count = self.utterance_count();
        tracing::debug!(
            "Validating CHAT file ({} headers, {} utterances) with streaming",
            header_count,
            utterance_count
        );

        let headers_with_spans: Vec<(&Header, crate::Span)> = self.headers_with_spans().collect();
        let headers: Vec<&Header> = headers_with_spans.iter().map(|(h, _)| *h).collect();
        let participant_ids: HashSet<crate::model::SpeakerCode> =
            self.participants.keys().cloned().collect();
        let context = build_validation_context(
            participant_ids,
            &self.languages,
            &headers,
            ValidationConfig::new(),
        );

        // Validate header collection (duplicates, required headers) - stream immediately
        let source_len = self.lines.last().map(|l| l.span().end as usize);
        header::structure::check_headers(&headers_with_spans, errors, source_len);

        // Validate individual headers - stream errors directly
        for (header, span) in &headers_with_spans {
            header::check_header(header, *span, &context, errors);
        }

        // Validate utterances - stream errors directly
        for utt in self.utterances() {
            utt.validate(&context, errors);
        }

        // Validate cross-utterance patterns
        let utterances_vec: Vec<crate::model::Utterance> = self.utterances().cloned().collect();
        cross_utterance::check_cross_utterance_patterns_with_sink(
            &utterances_vec,
            &context,
            errors,
        );

        // E362: Validate bullet timestamp monotonicity across utterances
        // Skip monotonicity check if bullets mode is enabled
        let bullets: Vec<&crate::model::Bullet> = self
            .utterances()
            .filter_map(|utt| utt.main.content.bullet.as_ref())
            .collect();
        if !bullets.is_empty() && !context.shared.bullets_mode {
            crate::validation::check_bullet_monotonicity(&bullets, errors);
        }

        // E701, E704: Validate temporal constraints on media bullets
        // - E701 (CLAN Error 83): Global timeline monotonicity
        // - E704 (CLAN Error 133): Per-speaker overlap with 500ms tolerance
        crate::validation::temporal::validate_temporal_constraints(
            self,
            context.shared.ca_mode,
            errors,
        );

        // E531: Validate media filename matches file name (if provided)
        if let Some(file_name) = filename {
            check_media_filename_match(&headers_with_spans, file_name, errors);
        }

        tracing::debug!("Streaming validation complete");
    }

    /// Validate this CHAT file with custom per-code severity configuration.
    ///
    /// This allows configuring validation behavior:
    /// - Downgrade errors to warnings
    /// - Disable specific error codes
    /// - Upgrade warnings to errors
    ///
    /// # Parameters
    ///
    /// * `config` - Validation configuration (severity overrides, disabled errors)
    /// * `errors` - Error sink for streaming validation errors
    /// * `filename` - Optional filename (without extension) for E531 validation
    ///
    /// # Example
    ///
    /// ```ignore
    /// use talkbank_model::{ChatFile, ErrorCollector, ValidationConfig};
    /// use talkbank_model::{ErrorCode, Severity};
    ///
    /// let config = ValidationConfig::new()
    ///     .downgrade(ErrorCode::IllegalUntranscribed, Severity::Warning)
    ///     .disable(ErrorCode::InvalidOverlapIndex);
    ///
    /// let errors = ErrorCollector::new();
    /// chat_file.validate_with_config(config, &errors, Some("myfile"));
    /// ```
    #[tracing::instrument(skip(self, errors), fields(lines = self.lines.len()))]
    pub fn validate_with_config(
        &self,
        config: ValidationConfig,
        errors: &impl crate::ErrorSink,
        filename: Option<&str>,
    ) {
        use crate::validation::{cross_utterance, header};

        let header_count = self.header_count();
        let utterance_count = self.utterance_count();
        tracing::debug!(
            "Validating CHAT file ({} headers, {} utterances) with custom config",
            header_count,
            utterance_count
        );

        // Apply severity/disable overrides at sink boundary.
        let configurable_sink = ConfigurableErrorSink::new(errors, config.clone());

        let headers_with_spans: Vec<(&Header, crate::Span)> = self.headers_with_spans().collect();
        let headers: Vec<&Header> = headers_with_spans.iter().map(|(h, _)| *h).collect();
        let participant_ids: HashSet<crate::model::SpeakerCode> =
            self.participants.keys().cloned().collect();
        let context = build_validation_context(participant_ids, &self.languages, &headers, config);

        // Validate header-set invariants.
        let source_len = self.lines.last().map(|l| l.span().end as usize);
        header::structure::check_headers(&headers_with_spans, &configurable_sink, source_len);

        // Validate each header payload.
        for (header, span) in &headers_with_spans {
            header::check_header(header, *span, &context, &configurable_sink);
        }

        // Validate utterances.
        for utt in self.utterances() {
            utt.validate(&context, &configurable_sink);
        }

        // Validate cross-utterance patterns
        let utterances_vec: Vec<crate::model::Utterance> = self.utterances().cloned().collect();
        cross_utterance::check_cross_utterance_patterns_with_sink(
            &utterances_vec,
            &context,
            &configurable_sink,
        );

        // E362: Validate bullet timestamp monotonicity across utterances
        // Skip monotonicity check if bullets mode is enabled
        let bullets: Vec<&crate::model::Bullet> = self
            .utterances()
            .filter_map(|utt| utt.main.content.bullet.as_ref())
            .collect();
        if !bullets.is_empty() && !context.shared.bullets_mode {
            crate::validation::check_bullet_monotonicity(&bullets, &configurable_sink);
        }

        // E701, E704: Validate temporal constraints on media bullets
        // - E701 (CLAN Error 83): Global timeline monotonicity
        // - E704 (CLAN Error 133): Per-speaker overlap with 500ms tolerance
        crate::validation::temporal::validate_temporal_constraints(
            self,
            context.shared.ca_mode,
            &configurable_sink,
        );

        // E531: Validate media filename matches file name (if provided)
        if let Some(file_name) = filename {
            check_media_filename_match(&headers_with_spans, file_name, &configurable_sink);
        }

        tracing::debug!("Streaming validation with config complete");
    }

    /// Validate this CHAT file including alignment/language precomputation.
    ///
    /// This first computes per-utterance alignment and language metadata, then
    /// runs the normal streaming validation pipeline.
    ///
    /// # Parameters
    ///
    /// * `errors` - Error sink for streaming validation errors
    /// * `filename` - Optional filename (without extension) for E531 validation
    #[tracing::instrument(skip(self, errors), fields(lines = self.lines.len()))]
    pub fn validate_with_alignment(
        &mut self,
        errors: &impl crate::ErrorSink,
        filename: Option<&str>,
    ) {
        let utterance_count = self.utterance_count();
        tracing::debug!(
            "Computing tier alignments for {} utterances",
            utterance_count
        );

        // Build shared context once for metadata precomputation.
        let headers: Vec<&Header> = self.headers().collect();
        let participant_ids: HashSet<crate::model::SpeakerCode> =
            self.participants.keys().cloned().collect();
        let context = build_validation_context(
            participant_ids,
            &self.languages,
            &headers,
            ValidationConfig::new(),
        );

        let default_language = context.shared.default_language.as_ref();
        let declared_languages = context.shared.declared_languages.as_slice();

        // Compute alignment and language metadata for all utterances.
        for line in &mut self.lines {
            if let Line::Utterance(utterance) = line {
                utterance.compute_alignments(&context);
                utterance.compute_language_metadata(default_language, declared_languages);
            }
        }

        tracing::debug!("Tier alignments computed, running streaming validation");

        // Run streaming validation
        self.validate(errors, filename)
    }
}

/// Return whether any `@Options` header enables CA mode.
///
/// CA mode relaxes some structural constraints and is propagated into the
/// shared validation context for downstream checks.
fn file_uses_ca_mode(headers: &[&Header]) -> bool {
    headers.iter().any(|header| match header {
        Header::Options { options } => options
            .iter()
            .any(crate::model::ChatOptionFlag::enables_ca_mode),
        _ => false,
    })
}

/// Return whether any `@Options` header enables `bullets` mode.
///
/// Bullets mode disables timestamp monotonicity assumptions in temporal checks.
fn file_uses_bullets_mode(headers: &[&Header]) -> bool {
    headers.iter().any(|header| match header {
        Header::Options { options } => options
            .iter()
            .any(|opt| matches!(opt, crate::model::ChatOptionFlag::Bullets)),
        _ => false,
    })
}

/// E531: validate `@Media` filename against the caller-provided file basename.
fn check_media_filename_match(
    headers: &[(&Header, crate::Span)],
    file_name: &str,
    errors: &impl crate::ErrorSink,
) {
    use crate::{ErrorCode, ErrorContext, ParseError, Severity, SourceLocation};

    // Find @Media header
    for (header, span) in headers {
        if let Header::Media(media_header) = header {
            let media_filename = media_header.filename.as_str();

            // Compare media filename with provided filename (case-insensitive)
            if !media_filename.eq_ignore_ascii_case(file_name) {
                let media_type_str = media_header.media_type.as_str();

                let mut err = ParseError::new(
                    ErrorCode::MediaFilenameMismatch,
                    Severity::Error,
                    SourceLocation::at_offset(span.start as usize),
                    ErrorContext::new(media_filename, 0..media_filename.len(), "media_filename"),
                    format!(
                        "Media filename '{}' does not match file name '{}' (case-insensitive comparison)",
                        media_filename, file_name
                    ),
                )
                .with_suggestion(format!(
                    "Update @Media header to: @Media:\t{}, {}",
                    file_name, media_type_str
                ));
                err.location.span = *span;
                errors.report(err);
            }

            // Only check the first @Media header
            break;
        }
    }
}

// Implement Validate trait for ChatFile (all states)
impl<S: ValidationState> Validate for ChatFile<S> {
    /// Delegates trait-based validation to full ChatFile validation pipeline.
    fn validate(&self, _context: &crate::validation::ValidationContext, errors: &impl ErrorSink) {
        // Delegate to the full validation method (without filename check)
        // The filename parameter is only used for E531 media filename validation,
        // which is optional and only relevant when validating from a file path.
        self.validate(errors, None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Span;
    use crate::model::{
        GraTier, GrammaticalRelation, Header, LanguageCode, MainTier, Mor, MorTier, MorWord,
        PosCategory, Terminator, Utterance, UtteranceContent, Word,
    };

    /// Build a minimal ChatFile wrapping one utterance.
    fn chat_with_utterance(utt: Utterance) -> ChatFile {
        ChatFile::new(vec![
            Line::header(Header::Utf8),
            Line::header(Header::Begin),
            Line::header(Header::Languages {
                codes: vec![LanguageCode::new("eng")].into(),
            }),
            Line::utterance(utt),
            Line::header(Header::End),
        ])
    }

    /// Builds a minimal main tier from word strings.
    fn simple_main_tier(words: &[&str]) -> MainTier {
        let content: Vec<UtteranceContent> = words
            .iter()
            .map(|w| UtteranceContent::Word(Box::new(Word::new_unchecked(*w, *w))))
            .collect();
        MainTier::new("CHI", content, Terminator::Period { span: Span::DUMMY })
    }

    /// Builds a minimal `%mor` tier from `(pos, lemma)` tuples.
    fn simple_mor_tier(items: &[(&str, &str)]) -> MorTier {
        let mors: Vec<Mor> = items
            .iter()
            .map(|(pos, lemma)| Mor::new(MorWord::new(PosCategory::new(*pos), *lemma)))
            .collect();
        MorTier::new_mor(mors).with_terminator(Some(".".into()))
    }

    /// Builds a synthetic `%gra` tier with `count` relations.
    fn simple_gra_tier(count: usize) -> GraTier {
        let mut rels = Vec::new();
        for i in 0..count {
            if i == 0 {
                rels.push(GrammaticalRelation::new(1, 0, "ROOT"));
            } else {
                rels.push(GrammaticalRelation::new(i + 1, 1, "MOD"));
            }
        }
        GraTier::new_gra(rels)
    }

    /// Alignment check passes when `%mor`/`%gra` cardinalities are consistent.
    #[test]
    fn validate_alignments_no_errors_for_matching_tiers() {
        let main = simple_main_tier(&["I", "go"]);
        let mor = simple_mor_tier(&[("pro", "I"), ("v", "go")]);
        // 2 words + terminator = 3 mor chunks → need 3 gra relations
        let gra = simple_gra_tier(3);
        let utt = Utterance::new(main).with_mor(mor).with_gra(gra);
        let chat = chat_with_utterance(utt);

        let errors = chat.validate_alignments();
        assert!(errors.is_empty(), "Expected no errors, got: {:?}", errors);
    }

    /// Alignment check reports mismatch when `%gra` has too few relations.
    #[test]
    fn validate_alignments_catches_mor_gra_mismatch() {
        let main = simple_main_tier(&["I", "go"]);
        let mor = simple_mor_tier(&[("pro", "I"), ("v", "go")]);
        // Intentionally wrong: 2 gra relations for 3 mor chunks (2 words + terminator)
        let gra = simple_gra_tier(2);
        let utt = Utterance::new(main).with_mor(mor).with_gra(gra);
        let chat = chat_with_utterance(utt);

        let errors = chat.validate_alignments();
        assert!(
            !errors.is_empty(),
            "Expected alignment errors for mor/gra mismatch"
        );
    }

    /// Tainted tier domains are skipped during alignment validation.
    #[test]
    fn validate_alignments_skips_tainted_tiers() {
        use crate::model::ParseHealthTier;

        let main = simple_main_tier(&["I", "go"]);
        let mor = simple_mor_tier(&[("pro", "I"), ("v", "go")]);
        // Intentionally wrong: 2 gra relations for 3 mor chunks (2 words + terminator)
        let gra = simple_gra_tier(2);

        let mut utt = Utterance::new(main).with_mor(mor).with_gra(gra);
        // Taint the gra tier — validation should skip mor→gra check
        utt.mark_parse_taint(ParseHealthTier::Gra);
        let chat = chat_with_utterance(utt);

        let errors = chat.validate_alignments();
        // Mor→gra check is skipped because gra is tainted, so no errors from that check.
        // Main→mor is still checked but should pass (2 words, 2 mor items).
        assert!(
            errors.is_empty(),
            "Expected no errors when gra is tainted, got: {:?}",
            errors
        );
    }

    /// Alignment check reports mismatch when main-word and `%mor` counts diverge.
    #[test]
    fn validate_alignments_catches_main_mor_mismatch() {
        // 3 words but only 2 mor items
        let main = simple_main_tier(&["I", "go", "home"]);
        let mor = simple_mor_tier(&[("pro", "I"), ("v", "go")]);
        let utt = Utterance::new(main).with_mor(mor);
        let chat = chat_with_utterance(utt);

        let errors = chat.validate_alignments();
        assert!(
            !errors.is_empty(),
            "Expected alignment errors for main/mor mismatch"
        );
    }
}
