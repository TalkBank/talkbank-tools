use std::sync::Arc;

use crate::chat_ops::morphosyntax_ops::{MultilingualPolicy, MwtDict, TokenizationMode};
use async_trait::async_trait;

use crate::api::{EngineVersion, LanguageCode3};
use crate::cache::UtteranceCache;
use crate::error::ServerError;
use crate::params::MorphosyntaxParams;
use crate::pipeline::PipelineServices;
use crate::text_batch::{TextBatchFileInput, TextBatchFileResults};
use crate::worker::pool::WorkerPool;

/// Runtime morphotag options resolved from command options for execution.
///
/// Owned (no borrowed `MwtDict`) so the value can move freely across
/// `tokio::spawn` boundaries used by per-file fanout in
/// `dispatch_morphotag_job`.
#[derive(Clone)]
pub(crate) struct MorphotagRuntimeOptions {
    pub(crate) tokenization_mode: TokenizationMode,
    pub(crate) multilingual_policy: MultilingualPolicy,
    pub(crate) mwt: Arc<MwtDict>,
    pub(crate) l2_morphotag: bool,
    pub(crate) respect_pos_hints: bool,
    pub(crate) should_merge_abbrev: bool,
    /// Review-tier verbosity for the incremental morphotag path
    /// Legacy review-level request retained for stored-job compatibility.
    /// No value emits CHAT decision tiers.
    ///
    /// [`MorphotagOptions`]: crate::options::MorphotagOptions
    pub(crate) review_level: crate::chat_ops::fa::ReviewLevel,
}

/// Worker-system seam consumed by the new execution kernel.
#[async_trait]
pub(crate) trait WorkerGateway: Send + Sync {
    /// Run the compare command's morphosyntax stage on one CHAT input.
    async fn morphotag_for_compare(
        &self,
        chat_text: &str,
        lang: &LanguageCode3,
        mwt: &MwtDict,
    ) -> Result<String, ServerError>;

    /// Run morphotag on one CHAT file.
    ///
    /// `progress` is this file's port into the job's batch-progress reporter, or
    /// `None` where no reporter exists (the CLI's direct path, tests). It is a
    /// separate argument rather than a field on `MorphotagRuntimeOptions`
    /// because it is an output port, not an option: options say what to compute,
    /// this says where to narrate it.
    async fn morphotag_single(
        &self,
        chat_text: &str,
        before_text: Option<&str>,
        lang: &LanguageCode3,
        options: MorphotagRuntimeOptions,
        progress: Option<&crate::execution::morphotag::progress::BackendProgressPort>,
    ) -> Result<String, ServerError>;

    /// Run utterance segmentation over one cross-file batch of CHAT inputs.
    ///
    /// `allow_stanza_fallback` propagates the
    /// `--utseg-fallback-stanza` operator opt-in: when `true`, the
    /// worker engages the legacy Stanza constituency-parser segmenter
    /// for languages without a TalkBank BERT utseg model. When
    /// `false` (default), the worker raises `UtsegModelNotFoundError`
    /// rather than silently substituting one model for another.
    async fn utseg_batch(
        &self,
        files: &[TextBatchFileInput],
        lang: &LanguageCode3,
        allow_stanza_fallback: bool,
    ) -> TextBatchFileResults;

    /// Run translation over one cross-file batch of CHAT inputs.
    async fn translate_batch(
        &self,
        files: &[TextBatchFileInput],
        lang: &LanguageCode3,
    ) -> TextBatchFileResults;

    /// Run coreference resolution over one cross-file batch of CHAT inputs.
    async fn coref_batch(
        &self,
        files: &[TextBatchFileInput],
        lang: &LanguageCode3,
    ) -> TextBatchFileResults;
}

/// Worker gateway backed by the existing worker pool and cache.
#[derive(Clone)]
pub(crate) struct PooledWorkerGateway {
    pool: Arc<WorkerPool>,
    cache: Arc<UtteranceCache>,
    engine_version: EngineVersion,
}

impl PooledWorkerGateway {
    /// Build a pool-backed worker gateway for one execution attempt.
    pub(crate) fn new(
        pool: Arc<WorkerPool>,
        cache: Arc<UtteranceCache>,
        engine_version: EngineVersion,
    ) -> Self {
        Self {
            pool,
            cache,
            engine_version,
        }
    }
}

#[async_trait]
impl WorkerGateway for PooledWorkerGateway {
    async fn morphotag_for_compare(
        &self,
        chat_text: &str,
        lang: &LanguageCode3,
        mwt: &MwtDict,
    ) -> Result<String, ServerError> {
        let params = MorphosyntaxParams {
            lang,
            tokenization_mode: TokenizationMode::Preserve,
            multilingual_policy: MultilingualPolicy::ProcessAll,
            mwt,
            l2_morphotag: false,
            respect_pos_hints: false,
            // Compare's internal morphotag never surfaces review tiers.
            review_level: crate::chat_ops::fa::ReviewLevel::None,
            // Compare runs morphotag on its own inputs, not on the job's files,
            // so there is no file row to report utterance counts against.
            progress: None,
        };
        crate::morphosyntax::process_morphosyntax(
            chat_text,
            PipelineServices::new(&self.pool, &self.cache, &self.engine_version),
            &params,
        )
        .await
    }

    async fn morphotag_single(
        &self,
        chat_text: &str,
        before_text: Option<&str>,
        lang: &LanguageCode3,
        options: MorphotagRuntimeOptions,
        progress: Option<&crate::execution::morphotag::progress::BackendProgressPort>,
    ) -> Result<String, ServerError> {
        let params = MorphosyntaxParams {
            lang,
            tokenization_mode: options.tokenization_mode,
            multilingual_policy: options.multilingual_policy,
            mwt: &options.mwt,
            l2_morphotag: options.l2_morphotag,
            respect_pos_hints: options.respect_pos_hints,
            review_level: options.review_level,
            progress,
        };
        let services = PipelineServices::new(&self.pool, &self.cache, &self.engine_version);
        if let Some(before) = before_text {
            crate::morphosyntax::process_morphosyntax_incremental(
                before, chat_text, services, &params,
            )
            .await
        } else {
            crate::morphosyntax::process_morphosyntax(chat_text, services, &params).await
        }
    }

    async fn utseg_batch(
        &self,
        files: &[TextBatchFileInput],
        lang: &LanguageCode3,
        allow_stanza_fallback: bool,
    ) -> TextBatchFileResults {
        crate::utseg::process_utseg_batch(
            files,
            lang,
            &self.pool,
            &self.cache,
            &self.engine_version,
            allow_stanza_fallback,
        )
        .await
    }

    async fn translate_batch(
        &self,
        files: &[TextBatchFileInput],
        lang: &LanguageCode3,
    ) -> TextBatchFileResults {
        crate::translate::process_translate_batch(
            files,
            lang,
            &self.pool,
            &self.cache,
            &self.engine_version,
        )
        .await
    }

    async fn coref_batch(
        &self,
        files: &[TextBatchFileInput],
        lang: &LanguageCode3,
    ) -> TextBatchFileResults {
        crate::coref::process_coref_batch(files, lang, &self.pool).await
    }
}
