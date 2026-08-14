//! V2 execute dispatch helpers, task mapping and engine override resolution.
//!
//! These pure functions bridge the V2 execute protocol (typed per-backend
//! requests) to the pool's typed worker-key abstraction. Extracted from
//! `mod.rs` for browsability.

use crate::api::WorkerLanguage;
use crate::types::engines::{AsrEngineName, EngineOverrides, FaEngineName};
use crate::types::worker_v2::{
    AsrBackendV2, ExecuteRequestV2, FaBackendV2, InferenceTaskV2, TaskRequestV2,
};
use crate::worker::error::WorkerError;
use crate::worker::{InferTask, WorkerBootstrapMode, WorkerTarget};

use super::{EngineSelection, WorkerKey};

/// Map a V2 inference task enum to the pool's infer-task vocabulary.
pub(super) fn infer_task_for_execute_v2(task: InferenceTaskV2) -> Result<InferTask, WorkerError> {
    match task {
        InferenceTaskV2::Morphosyntax => Ok(InferTask::Morphosyntax),
        InferenceTaskV2::Utseg => Ok(InferTask::Utseg),
        InferenceTaskV2::Translate => Ok(InferTask::Translate),
        InferenceTaskV2::Coref => Ok(InferTask::Coref),
        InferenceTaskV2::Asr => Ok(InferTask::Asr),
        InferenceTaskV2::ForcedAlignment => Ok(InferTask::Fa),
        InferenceTaskV2::Speaker => Ok(InferTask::Speaker),
        InferenceTaskV2::Opensmile => Ok(InferTask::Opensmile),
        InferenceTaskV2::Avqi => Ok(InferTask::Avqi),
    }
}

/// Derive the worker-pool key for one V2 execute request.
pub(super) fn execute_v2_worker_key(
    lang: WorkerLanguage,
    request: &ExecuteRequestV2,
    bootstrap_mode: WorkerBootstrapMode,
) -> Result<WorkerKey, WorkerError> {
    let infer_task = infer_task_for_execute_v2(request.task)?;
    let target = WorkerTarget::from_infer_task(infer_task, bootstrap_mode);

    // In LazyProfile mode, all GPU tasks for a language share ONE worker
    // process. Engine overrides are applied via ensure_task IPC, not by
    // creating separate workers per override. This prevents the memory guard
    // deadlock where pre-scale creates key "" and FA dispatch looks for
    // {"fa":"wave2vec"} (a user incident 2026-04-02).
    let engine_selection =
        if bootstrap_mode == WorkerBootstrapMode::LazyProfile && target.is_concurrent() {
            EngineSelection::none()
        } else {
            EngineSelection::from_execute_request(request)
        };

    Ok(WorkerKey {
        target,
        language: lang,
        engine_selection,
    })
}

impl EngineSelection {
    /// Derive worker configuration from a typed V2 request.
    ///
    /// This is the only V2 route into a pool engine identity. The typed
    /// `EngineOverrides` retains every opaque ASR extra, including
    /// `qwen_model`, until it is serialized for Python worker startup.
    pub(super) fn from_execute_request(request: &ExecuteRequestV2) -> Self {
        let overrides = match &request.payload {
            TaskRequestV2::Asr(request) => EngineOverrides {
                asr: Some(asr_backend_engine(request.backend)),
                extras: request.extras.clone(),
                ..EngineOverrides::default()
            },
            TaskRequestV2::ForcedAlignment(request) => EngineOverrides {
                fa: Some(fa_backend_engine(request.backend)),
                ..EngineOverrides::default()
            },
            _ => EngineOverrides::default(),
        };
        Self::from_overrides(overrides)
    }
}

/// Extract the ensure_task parameters (task name + engine overrides map) from a
/// V2 execute request, without the JSON round-trip.
///
/// Returns `None` for tasks that don't need model loading (e.g., text tasks in
/// eager-profile mode). Used by the LazyProfile dispatch path.
pub(super) fn ensure_task_params(
    request: &ExecuteRequestV2,
) -> Result<(String, Option<std::collections::BTreeMap<String, String>>), WorkerError> {
    let task = infer_task_for_execute_v2(request.task)?;
    let task_name = crate::worker::target::task_name(task).to_string();

    let selection = EngineSelection::from_execute_request(request);
    let overrides = (!selection.is_none()).then(|| selection.overrides().dispatch_overrides());

    Ok((task_name, overrides))
}

fn asr_backend_engine(backend: AsrBackendV2) -> AsrEngineName {
    match backend {
        AsrBackendV2::LocalWhisper => AsrEngineName::Whisper,
        AsrBackendV2::WhisperHub => AsrEngineName::WhisperHub,
        AsrBackendV2::HkTencent => AsrEngineName::HkTencent,
        AsrBackendV2::HkAliyun => AsrEngineName::HkAliyun,
        AsrBackendV2::HkFunaudio => AsrEngineName::HkFunaudio,
        AsrBackendV2::HkQwen => AsrEngineName::HkQwen,
        AsrBackendV2::Revai => AsrEngineName::RevAi,
    }
}

fn fa_backend_engine(backend: FaBackendV2) -> FaEngineName {
    match backend {
        FaBackendV2::Whisper => FaEngineName::Whisper,
        FaBackendV2::Wave2vec => FaEngineName::Wave2Vec,
        FaBackendV2::Wav2vecCanto => FaEngineName::Wav2vecCanto,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::LanguageCode3;
    use crate::types::worker_v2::{
        AsrInputV2, AsrRequestV2, FaTextModeV2, ForcedAlignmentRequestV2, MorphosyntaxRequestV2,
        PreparedAudioInputV2, WorkerArtifactIdV2, WorkerRequestIdV2,
    };
    use crate::worker::{WorkerBootstrapMode, WorkerProfile, WorkerTarget};

    fn request_with_payload(task: InferenceTaskV2, payload: TaskRequestV2) -> ExecuteRequestV2 {
        ExecuteRequestV2 {
            request_id: WorkerRequestIdV2::from("req-1"),
            task,
            payload,
            attachments: Vec::new(),
        }
    }

    #[test]
    fn maps_forced_alignment_execute_v2_to_fa_worker_profile() {
        assert_eq!(
            infer_task_for_execute_v2(InferenceTaskV2::ForcedAlignment).unwrap(),
            InferTask::Fa
        );
    }

    #[test]
    fn execute_v2_asr_worker_key_uses_request_backend_override() {
        let request = request_with_payload(
            InferenceTaskV2::Asr,
            TaskRequestV2::Asr(AsrRequestV2 {
                lang: WorkerLanguage::from(LanguageCode3::fra()),
                backend: AsrBackendV2::LocalWhisper,
                input: AsrInputV2::PreparedAudio(PreparedAudioInputV2 {
                    audio_ref_id: WorkerArtifactIdV2::from("audio-1"),
                }),
                extras: std::collections::BTreeMap::new(),
            }),
        );

        let key = execute_v2_worker_key(
            WorkerLanguage::from(LanguageCode3::fra()),
            &request,
            WorkerBootstrapMode::Profile,
        )
        .unwrap();

        assert_eq!(key.target, WorkerTarget::profile(WorkerProfile::Gpu));
        assert_eq!(key.language, WorkerLanguage::from(LanguageCode3::fra()));
        assert_eq!(
            key.engine_selection.worker_config_json(),
            r#"{"asr":"whisper"}"#
        );
    }

    #[test]
    fn execute_v2_engine_overrides_preserves_asr_extras_through_dispatch() {
        // Regression pin for the 2026-05-27 root-cause bug: prior to
        // adding ``extras`` to ``AsrRequestV2`` and threading them
        // through here, the pool key + worker spawn argv silently
        // dropped every per-engine knob the user passed via
        // ``--engine-overrides``. A request with
        // ``qwen_model=Qwen/Qwen3-ASR-0.6B`` was serialized as bare
        // ``{"asr":"qwen"}`` and the worker defaulted to 1.7B,
        // costing hours of wasted compute.
        //
        // This test is the seam test the per-knob Fix 1 CLI-parse
        // test SHOULD have been: it asserts the user's extras reach
        // the worker spawn argv JSON. Adding any new per-engine knob
        // (funaudio_*, future engines) is automatically covered
        // because the assertion is "every key in input.extras
        // appears in output JSON", not a fixed allowlist.
        let mut extras = std::collections::BTreeMap::new();
        extras.insert("qwen_model".to_owned(), "Qwen/Qwen3-ASR-0.6B".to_owned());
        extras.insert("qwen_device".to_owned(), "cpu".to_owned());

        let request = request_with_payload(
            InferenceTaskV2::Asr,
            TaskRequestV2::Asr(AsrRequestV2 {
                lang: WorkerLanguage::from(LanguageCode3::yue()),
                backend: AsrBackendV2::HkQwen,
                input: AsrInputV2::ProviderMedia(crate::types::worker_v2::ProviderMediaInputV2 {
                    media_path: "/dev/null".into(),
                    num_speakers: crate::api::NumSpeakers(1),
                }),
                extras: extras.clone(),
            }),
        );

        let json = EngineSelection::from_execute_request(&request).worker_config_json();
        let parsed: std::collections::BTreeMap<String, String> =
            serde_json::from_str(&json).expect("engine_overrides JSON must round-trip");

        assert_eq!(parsed.get("asr").map(String::as_str), Some("qwen"));
        for (k, v) in &extras {
            assert_eq!(
                parsed.get(k),
                Some(v),
                "extras key {k:?} (value {v:?}) was dropped at the V2 dispatch boundary, \
                 pool key + worker spawn argv would lose it"
            );
        }
    }

    #[test]
    fn execute_v2_non_worker_asr_does_not_fragment_keys_but_keeps_extras() {
        let mut extras = std::collections::BTreeMap::new();
        extras.insert("provider_region".to_owned(), "us-east".to_owned());
        let request = request_with_payload(
            InferenceTaskV2::Asr,
            TaskRequestV2::Asr(AsrRequestV2 {
                lang: WorkerLanguage::from(LanguageCode3::eng()),
                backend: AsrBackendV2::Revai,
                input: AsrInputV2::PreparedAudio(PreparedAudioInputV2 {
                    audio_ref_id: WorkerArtifactIdV2::from("audio-1"),
                }),
                extras: extras.clone(),
            }),
        );

        let selection = EngineSelection::from_execute_request(&request);
        assert_eq!(selection.overrides().asr, None);
        assert_eq!(selection.overrides().extras, extras);
        assert_eq!(
            selection.worker_config_json(),
            r#"{"provider_region":"us-east"}"#
        );
    }

    #[test]
    fn execute_v2_fa_worker_key_uses_request_backend_override() {
        let request = request_with_payload(
            InferenceTaskV2::ForcedAlignment,
            TaskRequestV2::ForcedAlignment(ForcedAlignmentRequestV2 {
                backend: FaBackendV2::Wave2vec,
                payload_ref_id: WorkerArtifactIdV2::from("payload-1"),
                audio_ref_id: WorkerArtifactIdV2::from("audio-1"),
                text_mode: FaTextModeV2::SpaceJoined,
            }),
        );

        let key = execute_v2_worker_key(
            WorkerLanguage::from(LanguageCode3::eng()),
            &request,
            WorkerBootstrapMode::Profile,
        )
        .unwrap();

        assert_eq!(key.target, WorkerTarget::profile(WorkerProfile::Gpu));
        assert_eq!(key.language, WorkerLanguage::from(LanguageCode3::eng()));
        assert_eq!(
            key.engine_selection.worker_config_json(),
            r#"{"fa":"wave2vec"}"#
        );
    }

    #[test]
    fn execute_v2_worker_key_uses_task_target_when_requested() {
        let request = request_with_payload(
            InferenceTaskV2::Morphosyntax,
            TaskRequestV2::Morphosyntax(MorphosyntaxRequestV2 {
                lang: LanguageCode3::eng(),
                payload_ref_id: WorkerArtifactIdV2::from("payload-1"),
                item_count: 1,
                retokenize: false,
            }),
        );

        let key = execute_v2_worker_key(
            WorkerLanguage::from(LanguageCode3::eng()),
            &request,
            WorkerBootstrapMode::Task,
        )
        .unwrap();

        assert_eq!(
            key.target,
            WorkerTarget::infer_task(InferTask::Morphosyntax)
        );
    }

    #[test]
    fn lazy_profile_gpu_key_drops_engine_overrides() {
        // In LazyProfile mode, ALL GPU tasks for a language share one worker.
        // Engine overrides are loaded via ensure_task, not baked into the key.
        let fa_request = request_with_payload(
            InferenceTaskV2::ForcedAlignment,
            TaskRequestV2::ForcedAlignment(ForcedAlignmentRequestV2 {
                backend: FaBackendV2::Wave2vec,
                payload_ref_id: WorkerArtifactIdV2::from("payload-1"),
                audio_ref_id: WorkerArtifactIdV2::from("audio-1"),
                text_mode: FaTextModeV2::SpaceJoined,
            }),
        );

        let asr_request = request_with_payload(
            InferenceTaskV2::Asr,
            TaskRequestV2::Asr(AsrRequestV2 {
                lang: WorkerLanguage::from(LanguageCode3::eng()),
                backend: AsrBackendV2::LocalWhisper,
                input: AsrInputV2::PreparedAudio(PreparedAudioInputV2 {
                    audio_ref_id: WorkerArtifactIdV2::from("audio-1"),
                }),
                extras: std::collections::BTreeMap::new(),
            }),
        );

        let fa_key = execute_v2_worker_key(
            WorkerLanguage::from(LanguageCode3::eng()),
            &fa_request,
            WorkerBootstrapMode::LazyProfile,
        )
        .unwrap();

        let asr_key = execute_v2_worker_key(
            WorkerLanguage::from(LanguageCode3::eng()),
            &asr_request,
            WorkerBootstrapMode::LazyProfile,
        )
        .unwrap();

        // Both use no engine selection, so they share one worker key.
        assert!(fa_key.engine_selection.is_none());
        assert!(asr_key.engine_selection.is_none());
        // Same target and language → same worker.
        assert_eq!(fa_key.target, asr_key.target);
        assert_eq!(fa_key.language, asr_key.language);
    }
}
