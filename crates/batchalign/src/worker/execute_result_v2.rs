//! One reader for worker-protocol V2 execute responses.
//!
//! [`ExecuteResponseV2`] used to split a single fact across two public fields
//! (an outcome beside an `Option<TaskResultV2>`), so every result adapter
//! checked both by hand, in an order nobody wrote down. Four adapters checked
//! the outcome first and two checked the payload first; since a failed request
//! carries no payload, the two that led with the payload reported every typed
//! error response as "missing a result payload" and threw away the protocol
//! code and message the worker had actually sent.
//!
//! The pairing is structural since 2026-08-21 (the type's interior is a
//! validated enum, and deserialization refuses a disagreeing wire value), so
//! `ExecuteResponseV2::read` is total over the two legal shapes. This module
//! keeps only the shared message shape every adapter uses.

use crate::types::worker_v2::{ExecuteOutcomeRef, ExecuteResponseV2, TaskResultV2};

/// Require that one execute response succeeded and produced a result payload.
///
/// `task` names the task in the diagnostics, so the four adapters share these
/// two message shapes rather than each spelling them out.
pub fn require_success_result<'a>(
    response: &'a ExecuteResponseV2,
    task: &str,
) -> Result<&'a TaskResultV2, String> {
    match response.read() {
        ExecuteOutcomeRef::Success(result) => Ok(result),
        ExecuteOutcomeRef::Failed { code, message } => Err(format!(
            "worker protocol V2 {task} request failed with {code:?}: {message}"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::api::DurationSeconds;
    use crate::types::worker_v2::{ProtocolErrorCodeV2, SpeakerResultV2, WorkerRequestIdV2};

    // NOTE (2026-08-21): this module used to also test "success with no
    // payload yields the missing-a-result-payload message". That scenario is
    // no longer WRITABLE: `ExecuteResponseV2` became structural, its interior
    // is a validated enum, and a success without a payload has no constructor
    // and is refused at deserialization. The test went with the state it
    // guarded, per types-over-tests.

    /// The regression this module exists for. A failed response carries no
    /// payload, and the openSMILE and AVQI dispatch paths used to read the
    /// payload first and so answer "missing a result payload". The code and
    /// message must survive instead.
    #[test]
    fn a_failed_response_reports_its_code_and_message_not_a_missing_payload() {
        let failed = ExecuteResponseV2::failure(
            WorkerRequestIdV2::from("req-1".to_owned()),
            ProtocolErrorCodeV2::ModelUnavailable,
            "no speaker host loaded".to_owned(),
            DurationSeconds(0.0),
        );

        let error = require_success_result(&failed, "speaker")
            .expect_err("a failed response must not read as success");

        assert!(error.contains("ModelUnavailable"), "{error}");
        assert!(error.contains("no speaker host loaded"), "{error}");
        assert!(
            !error.contains("missing a result payload"),
            "the reported failure was replaced by the missing-payload message: {error}"
        );
    }

    #[test]
    fn a_successful_response_yields_its_payload() {
        let ok = ExecuteResponseV2::success(
            WorkerRequestIdV2::from("req-1".to_owned()),
            TaskResultV2::SpeakerResult(SpeakerResultV2 {
                evidence: crate::types::worker_v2::SpeakerInferenceEvidenceV2::Pyannote {
                    segments: Vec::new(),
                },
            }),
            DurationSeconds(0.0),
        );

        assert!(matches!(
            require_success_result(&ok, "speaker"),
            Ok(TaskResultV2::SpeakerResult(_))
        ));
    }
}
