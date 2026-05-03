//! Morphosyntax pipeline built on the internal stage runner.

use crate::chat_ops::morphosyntax_ops::{
    BatchItemWithPosition, MultilingualPolicy, MwtDict, TokenizationMode, clear_morphosyntax,
    collect_payloads, declared_languages, inject_results, l2,
    remove_empty_morphosyntax_placeholders, validate_mor_alignment,
};
use crate::chat_ops::nlp::UdResponse;
use crate::chat_ops::{ChatFile, LanguageCode};
use talkbank_transform::parse::{is_ca, is_dummy, parse_lenient};
use talkbank_transform::serialize::to_chat_string;
use talkbank_transform::validate::{ValidityLevel, validate_output, validate_to_level};
use tracing::warn;

use crate::api::LanguageCode3;
use crate::error::ServerError;
use crate::morphosyntax::infer_batch;
use crate::pipeline::PipelineServices;
use crate::pipeline::plan::{PipelinePlan, StageFuture, StageId, StageSpec, run_plan};

/// Per-file morphosyntax pipeline state.
pub(crate) struct MorphosyntaxPipelineContext<'a> {
    /// Shared services for the run.
    pub services: PipelineServices<'a>,
    /// Original chat text.
    pub chat_text: &'a str,
    /// Job language.
    pub lang: &'a LanguageCode3,
    /// Injection tokenization mode.
    pub tokenization_mode: TokenizationMode,
    /// Multilingual payload collection policy.
    pub multilingual_policy: MultilingualPolicy,
    /// MWT lexicon for retokenization overrides.
    pub mwt: &'a MwtDict,
    /// [Experimental] Route @s words to secondary language Stanza models.
    pub l2_morphotag: bool,
    /// Parsed chat file.
    pub chat_file: Option<ChatFile>,
    /// Structured parse errors from lenient parse; drives the L0 pre-validation gate.
    pub parse_errors: Vec<crate::chat_ops::ParseError>,
    /// Whether the file is a dummy transcript (`@Options: dummy`).
    pub is_dummy: bool,
    /// Whether the file is a Conversation Analysis transcript (`@Options: CA`).
    /// Mirrors `is_no_align` in the align pipeline: CA files are pass-through
    /// for morphosyntax — no clear, no infer, no inject, no provenance.
    pub is_ca: bool,
    /// Collected worker payloads.
    pub batch_items: Vec<BatchItemWithPosition>,
    /// Inferred worker responses.
    pub ud_responses: Vec<UdResponse>,
    /// Final serialized output.
    pub final_chat_text: Option<String>,
}

impl<'a> MorphosyntaxPipelineContext<'a> {
    fn new(
        chat_text: &'a str,
        lang: &'a LanguageCode3,
        services: PipelineServices<'a>,
        tokenization_mode: TokenizationMode,
        multilingual_policy: MultilingualPolicy,
        mwt: &'a MwtDict,
        l2_morphotag: bool,
    ) -> Self {
        Self {
            services,
            chat_text,
            lang,
            tokenization_mode,
            multilingual_policy,
            mwt,
            l2_morphotag,
            chat_file: None,
            parse_errors: Vec::new(),
            is_dummy: false,
            is_ca: false,
            batch_items: Vec::new(),
            ud_responses: Vec::new(),
            final_chat_text: None,
        }
    }

    /// True when the file should bypass all morphosyntax inference stages
    /// (parse + serialize round-trip only). Set by `stage_parse` based on
    /// `@Options: dummy` or `@Options: CA`.
    fn should_skip_inference(&self) -> bool {
        self.is_dummy || self.is_ca
    }
}

/// Run the morphosyntax pipeline for a single CHAT file.
pub(crate) async fn run_morphosyntax_pipeline(
    chat_text: &str,
    lang: &LanguageCode3,
    services: PipelineServices<'_>,
    tokenization_mode: TokenizationMode,
    multilingual_policy: MultilingualPolicy,
    mwt: &MwtDict,
    l2_morphotag: bool,
) -> Result<String, ServerError> {
    let plan = morphosyntax_plan();
    let mut ctx = MorphosyntaxPipelineContext::new(
        chat_text,
        lang,
        services,
        tokenization_mode,
        multilingual_policy,
        mwt,
        l2_morphotag,
    );
    let _ = run_plan("morphotag", &plan, &mut ctx, None).await?;
    ctx.final_chat_text.ok_or_else(|| {
        ServerError::Validation("morphotag pipeline completed without output".to_string())
    })
}

fn morphosyntax_plan<'a>() -> PipelinePlan<MorphosyntaxPipelineContext<'a>> {
    PipelinePlan::new(vec![
        StageSpec::new(StageId::Parse, vec![], always_enabled, stage_parse),
        StageSpec::new(
            StageId::PreValidate,
            vec![StageId::Parse],
            always_enabled,
            stage_prevalidate,
        ),
        StageSpec::new(
            StageId::ClearExisting,
            vec![StageId::PreValidate],
            always_enabled,
            stage_clear_existing,
        ),
        StageSpec::new(
            StageId::CollectPayloads,
            vec![StageId::ClearExisting],
            always_enabled,
            stage_collect_payloads,
        ),
        StageSpec::new(
            StageId::Infer,
            vec![StageId::CollectPayloads],
            always_enabled,
            stage_infer,
        ),
        StageSpec::new(
            StageId::ApplyResults,
            vec![StageId::Infer],
            always_enabled,
            stage_apply_results,
        ),
        StageSpec::new(
            StageId::PostValidate,
            vec![StageId::ApplyResults],
            always_enabled,
            stage_postvalidate,
        ),
        StageSpec::new(
            StageId::Serialize,
            vec![StageId::PostValidate],
            always_enabled,
            stage_serialize,
        ),
    ])
}

fn always_enabled(_: &MorphosyntaxPipelineContext<'_>) -> bool {
    true
}

/// Refuse to morphotag a file whose primary `@Languages:` is not in
/// the Stanza-supported set.
///
/// **BA2 parity.** BA2 fed each file's `doc.langs` (parsed from
/// `@Languages:` at `formats/chat/parser.py:228-229` of the BA2 archive)
/// directly to `stanza.Pipeline`, which raised on missing models. The
/// 2026-05-03 morphotag rerun on ming silently rewrote 736+ Stanza-
/// unsupported-language files (Serbian, etc.) with empty `%mor` and a
/// falsified `lang=eng` provenance comment because BA3's job-level
/// `MorphosyntaxParams.lang` (`--lang` falling through to
/// `default_lang: eng`) overrode the file's actual language. This check
/// restores the BA2 hard-error behaviour.
///
/// Returning `Err` from here triggers `TextBatchFileResult::err`, which
/// `execution::text_io::write_text_results` does NOT write to disk — the
/// file stays untouched on the user's filesystem (no provenance stamp,
/// no empty `%mor`).
///
/// Empty `@Languages` is intentionally NOT errored here: BA2's
/// behaviour for missing-header files is to default to `["eng"]` and
/// proceed. That fallback path stays unchanged for now.
pub(crate) fn check_primary_language_supported(chat_file: &ChatFile) -> Result<(), ServerError> {
    if let Some(primary) = chat_file.languages.0.first() {
        if !crate::chat_ops::morphosyntax_ops::is_stanza_supported(primary) {
            return Err(ServerError::Validation(format!(
                "morphotag: file's primary @Languages '{}' is not supported by Stanza. \
                 Supported ISO-639-3 codes: {}. \
                 (BA2 parity: this is a hard error, not a silent rewrite.)",
                primary,
                talkbank_transform::morphosyntax::supported_iso3_codes().join(", ")
            )));
        }
    }
    Ok(())
}

fn stage_parse<'a, 'ctx>(ctx: &'a mut MorphosyntaxPipelineContext<'ctx>) -> StageFuture<'a> {
    Box::pin(async move {
        let parser = crate::chat_parser();
        let (chat_file, parse_errors) = parse_lenient(&parser, ctx.chat_text);
        if !parse_errors.is_empty() {
            warn!(
                num_errors = parse_errors.len(),
                "Parse errors in morphosyntax input (continuing with recovery)"
            );
        }
        ctx.parse_errors = parse_errors;
        ctx.is_dummy = is_dummy(&chat_file);
        ctx.is_ca = is_ca(&chat_file);

        if !ctx.is_dummy && !ctx.is_ca {
            check_primary_language_supported(&chat_file)?;
        }

        ctx.chat_file = Some(chat_file);
        Ok(())
    })
}

fn stage_prevalidate<'a, 'ctx>(ctx: &'a mut MorphosyntaxPipelineContext<'ctx>) -> StageFuture<'a> {
    Box::pin(async move {
        if ctx.should_skip_inference() {
            return Ok(());
        }
        let chat_file = ctx.chat_file.as_ref().ok_or_else(|| {
            ServerError::Validation("Parsed chat missing before morphotag pre-validation".into())
        })?;
        if let Err(errors) =
            validate_to_level(chat_file, &ctx.parse_errors, ValidityLevel::MainTierValid)
        {
            let msgs: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
            return Err(ServerError::Validation(format!(
                "morphotag pre-validation failed: {}",
                msgs.join("; ")
            )));
        }
        Ok(())
    })
}

fn stage_clear_existing<'a, 'ctx>(
    ctx: &'a mut MorphosyntaxPipelineContext<'ctx>,
) -> StageFuture<'a> {
    Box::pin(async move {
        if ctx.should_skip_inference() {
            return Ok(());
        }
        let chat_file = ctx.chat_file.as_mut().ok_or_else(|| {
            ServerError::Validation("Parsed chat missing before clearing morphosyntax".into())
        })?;
        clear_morphosyntax(chat_file);
        Ok(())
    })
}

fn stage_collect_payloads<'a, 'ctx>(
    ctx: &'a mut MorphosyntaxPipelineContext<'ctx>,
) -> StageFuture<'a> {
    Box::pin(async move {
        if ctx.should_skip_inference() {
            return Ok(());
        }
        let primary_lang = LanguageCode::new(ctx.lang.as_ref());
        let chat_file = ctx.chat_file.as_ref().ok_or_else(|| {
            ServerError::Validation("Parsed chat missing before payload collection".into())
        })?;
        let langs = declared_languages(chat_file, &primary_lang);
        let collected = collect_payloads(chat_file, &primary_lang, &langs, ctx.multilingual_policy);
        ctx.batch_items = collected.batch_items;
        // Wave 5: thread collected.not_applicable into per-file outcome reporting.
        Ok(())
    })
}

fn stage_infer<'a, 'ctx>(ctx: &'a mut MorphosyntaxPipelineContext<'ctx>) -> StageFuture<'a> {
    Box::pin(async move {
        if ctx.batch_items.is_empty() {
            return Ok(());
        }
        let lang_code = ctx.lang.clone();
        let retokenize = ctx.tokenization_mode == TokenizationMode::StanzaRetokenize;
        ctx.ud_responses = infer_batch(
            ctx.services.pool,
            &ctx.batch_items,
            &lang_code,
            ctx.mwt,
            retokenize,
            None,
        )
        .await?;
        Ok(())
    })
}

fn stage_apply_results<'a, 'ctx>(
    ctx: &'a mut MorphosyntaxPipelineContext<'ctx>,
) -> StageFuture<'a> {
    Box::pin(async move {
        if ctx.batch_items.is_empty() {
            return Ok(());
        }

        // Extract L2 deferred positions BEFORE inject_results takes
        // ownership of items/responses (same pattern as batch.rs).
        let l2_deferred = if ctx.l2_morphotag {
            l2::extract_l2_deferred_positions(&ctx.batch_items, &ctx.ud_responses)
        } else {
            Vec::new()
        };

        let chat_file = ctx.chat_file.as_mut().ok_or_else(|| {
            ServerError::Validation("Parsed chat missing before result injection".into())
        })?;
        let lang_code = LanguageCode::new(ctx.lang.as_ref());
        let parser = crate::chat_parser();
        let _injection_result = inject_results(
            &parser,
            chat_file,
            std::mem::take(&mut ctx.batch_items),
            std::mem::take(&mut ctx.ud_responses),
            &lang_code,
            ctx.tokenization_mode,
            ctx.mwt,
        )
        .map_err(|e| ServerError::Validation(format!("Result injection failed: {e}")))?;

        // Secondary L2 dispatch (experimental): route @s words to
        // secondary language workers and splice real morphology.
        if !l2_deferred.is_empty() {
            crate::morphosyntax::dispatch_secondary_l2(
                chat_file,
                &l2_deferred,
                ctx.services,
                "single-file",
            )
            .await;
        }

        let alignment_warnings = validate_mor_alignment(chat_file);
        for warning in &alignment_warnings {
            warn!(warning = %warning, "Morphosyntax alignment mismatch");
        }
        Ok(())
    })
}

fn stage_postvalidate<'a, 'ctx>(ctx: &'a mut MorphosyntaxPipelineContext<'ctx>) -> StageFuture<'a> {
    Box::pin(async move {
        if ctx.should_skip_inference() {
            return Ok(());
        }
        let chat_file = ctx.chat_file.as_ref().ok_or_else(|| {
            ServerError::Validation("Parsed chat missing before morphotag post-validation".into())
        })?;
        if let Err(errors) = validate_output(chat_file, "morphotag") {
            let msgs: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
            warn!(errors = ?msgs, "morphotag post-validation warnings (non-fatal)");
        }
        Ok(())
    })
}

fn stage_serialize<'a, 'ctx>(ctx: &'a mut MorphosyntaxPipelineContext<'ctx>) -> StageFuture<'a> {
    Box::pin(async move {
        let chat_file = ctx.chat_file.as_mut().ok_or_else(|| {
            ServerError::Validation("Parsed chat missing before morphotag serialize".into())
        })?;

        // CA pass-through: serialize the parsed file as-is, no provenance,
        // no placeholder sweep. Mirrors the NoAlign branch in `fa/mod.rs`
        // (`if is_no_align(&chat_file) { return ... to_chat_string ... }`).
        // Dummy files still get provenance (existing behavior, intentional).
        if ctx.is_ca {
            ctx.final_chat_text = Some(to_chat_string(chat_file));
            return Ok(());
        }

        // Inject processing provenance comment.
        let provenance = crate::provenance::morphotag_provenance(
            ctx.lang.as_ref(),
            ctx.services.engine_version.as_ref(),
            ctx.tokenization_mode
                == crate::chat_ops::morphosyntax_ops::TokenizationMode::StanzaRetokenize,
            false, // incremental is handled separately
        );
        crate::provenance::inject_provenance(chat_file, &provenance);

        // Sweep any unfilled %mor/%gra placeholders left by clear_morphosyntax
        // for utterances whose response produced no UD sentence. Empty
        // placeholders preserve tier order during injection; this cleanup
        // prevents them from leaking into the serialized output.
        remove_empty_morphosyntax_placeholders(chat_file);

        ctx.final_chat_text = Some(to_chat_string(chat_file));
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    //! Tests for the per-file Stanza-supported language gate. The full
    //! pipeline is exercised by integration tests against a worker pool;
    //! these only cover the predicate logic that decides whether a parsed
    //! CHAT file may proceed to morphotag inference.
    use super::check_primary_language_supported;
    use crate::error::ServerError;
    use talkbank_transform::parse::parse_lenient;

    fn parse(text: &str) -> talkbank_model::model::ChatFile {
        let parser = crate::chat_parser();
        let (chat_file, _) = parse_lenient(&parser, text);
        chat_file
    }

    #[test]
    fn unsupported_primary_language_hard_errors() {
        let chat = "@UTF8\n\
                    @PID:\t11312/c-test\n\
                    @Begin\n\
                    @Languages:\tsrp\n\
                    @Participants:\tCHI Target_Child\n\
                    @ID:\tsrp|test|CHI||female|||Target_Child|||\n\
                    *CHI:\tnešto .\n\
                    @End\n";
        let chat_file = parse(chat);
        let result = check_primary_language_supported(&chat_file);
        match result {
            Err(ServerError::Validation(msg)) => {
                assert!(
                    msg.contains("srp"),
                    "error must name the unsupported lang: {msg}"
                );
                assert!(
                    msg.contains("not supported by Stanza"),
                    "error must say why: {msg}"
                );
            }
            Ok(()) => {
                panic!("expected hard error for srp, got Ok — unsupported-language gate is broken")
            }
            Err(other) => panic!("expected Validation error, got {other:?}"),
        }
    }

    #[test]
    fn supported_primary_language_passes() {
        let chat = "@UTF8\n\
                    @PID:\t11312/c-test\n\
                    @Begin\n\
                    @Languages:\teng\n\
                    @Participants:\tCHI Target_Child\n\
                    @ID:\teng|test|CHI||female|||Target_Child|||\n\
                    *CHI:\thello .\n\
                    @End\n";
        let chat_file = parse(chat);
        assert!(
            check_primary_language_supported(&chat_file).is_ok(),
            "eng must pass the Stanza-supported gate",
        );
    }

    #[test]
    fn empty_languages_header_passes_for_ba2_compat() {
        // BA2 defaulted to ["eng"] when no @Languages was present and proceeded.
        // The gate intentionally allows this — files lacking @Languages are not
        // hard-errored; they fall through to the dispatch's default-lang path.
        let chat = "@UTF8\n\
                    @PID:\t11312/c-test\n\
                    @Begin\n\
                    @Participants:\tCHI Target_Child\n\
                    @ID:\teng|test|CHI||female|||Target_Child|||\n\
                    *CHI:\thello .\n\
                    @End\n";
        let chat_file = parse(chat);
        assert!(
            check_primary_language_supported(&chat_file).is_ok(),
            "missing @Languages must NOT hard-error (BA2 parity: defaults to eng)",
        );
    }

    #[test]
    fn supported_language_with_unsupported_secondary_passes() {
        // A bilingual file where primary is supported (eng) and secondary is
        // not Stanza-supported (e.g., a non-Stanza tongue). The gate looks at
        // primary only; multilingual policy handles per-utterance routing.
        let chat = "@UTF8\n\
                    @PID:\t11312/c-test\n\
                    @Begin\n\
                    @Languages:\teng, srp\n\
                    @Participants:\tCHI Target_Child\n\
                    @ID:\teng|test|CHI||female|||Target_Child|||\n\
                    *CHI:\thello .\n\
                    @End\n";
        let chat_file = parse(chat);
        assert!(
            check_primary_language_supported(&chat_file).is_ok(),
            "primary=eng with secondary=srp must pass (gate is on primary only)",
        );
    }
}
