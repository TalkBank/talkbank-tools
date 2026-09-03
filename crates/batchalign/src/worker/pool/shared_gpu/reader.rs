//! Generic response reader loop shared between stdio and TCP GPU workers.
//!
//! Reads JSON-lines from any `AsyncBufRead`, routing V2 responses by
//! `request_id` to pending oneshot senders, and non-V2 responses
//! (health, capabilities, shutdown, error) to a control channel.

use std::collections::HashMap;
use std::sync::Arc;

use serde_json::Value;
use tokio::sync::oneshot;
use tracing::{debug, error, warn};

use tokio::io::AsyncBufReadExt;

use crate::api::DurationSeconds;
use crate::types::worker_v2::{ExecuteResponseV2, ProtocolErrorCodeV2, WorkerRequestIdV2};
use crate::worker::WorkerPid;

use super::WorkerControlResponse;
use super::envelopes::{
    CapabilitiesResponseEnvelope, EnsureTaskResponseEnvelope, ExecuteResponseV2Envelope,
    HealthResponseEnvelope,
};

/// Who an `op=error` envelope belongs to.
///
/// Naming the two cases is what keeps the routing exhaustive. Written as an
/// `Option<WorkerRequestIdV2>` guarded by an `if let`, the two failure routes
/// out of that shape (a tagged error with no pending entry, and an untagged
/// error with no sequential op waiting) both read as "nothing to do here", and
/// the second of them silently dropped a worker's own diagnosis on the floor.
enum ErrorCorrelation {
    /// The worker named the V2 dispatch this failure belongs to.
    Dispatch(WorkerRequestIdV2),
    /// The worker did not, or could not: the line never parsed, or the failing
    /// op is a sequential one answered through the control channel.
    Uncorrelated,
}

/// Offer an error to a sequential op, and hand it back if none is waiting.
///
/// Returns the message rather than swallowing it, so a caller has to say what
/// happens when nobody was listening. The empty control slot absorbing an
/// error nobody could see is the whole of the 2026-09-02 `speaker-identify`
/// stall.
async fn offer_to_sequential_op(
    control: &Arc<tokio::sync::Mutex<Option<oneshot::Sender<WorkerControlResponse>>>>,
    error_msg: String,
) -> Option<String> {
    // `control` is an async mutex, so the receiver is taken in its own scope
    // and no caller of this function holds the std `pending` lock across it.
    let waiter = {
        let mut ctrl = control.lock().await;
        ctrl.take()
    };
    match waiter {
        Some(tx) => {
            let _ = tx.send(WorkerControlResponse::Error(error_msg));
            None
        }
        None => Some(error_msg),
    }
}

/// Fail every V2 dispatch in flight with one error.
///
/// The right answer only for an error that names NO request: it belongs to one
/// of them and the envelope does not say which, and a stream that has produced
/// an uncorrelated failure cannot be trusted to answer the requests already on
/// it. Deliberately NOT applied to a tagged error whose id has no pending
/// entry, which means that one caller already gave up and says nothing about
/// the others.
fn fail_every_dispatch_in_flight(
    pending: &Arc<std::sync::Mutex<HashMap<String, oneshot::Sender<ExecuteResponseV2>>>>,
    pid: WorkerPid,
    error_msg: &str,
) {
    let stranded: Vec<(String, oneshot::Sender<ExecuteResponseV2>)> = {
        let mut pending = super::super::lock_recovered(pending);
        pending.drain().collect()
    };
    if stranded.is_empty() {
        warn!(
            pid = %pid,
            error = %error_msg,
            "GPU worker: error names no request and nothing is waiting for one"
        );
        return;
    }
    warn!(
        pid = %pid,
        error = %error_msg,
        failed_requests = stranded.len(),
        "GPU worker: error names no request; failing every dispatch in flight"
    );
    for (request_id, tx) in stranded {
        let _ = tx.send(ExecuteResponseV2::failure(
            WorkerRequestIdV2::from(request_id),
            ProtocolErrorCodeV2::InvalidPayload,
            error_msg.to_owned(),
            DurationSeconds(0.0),
        ));
    }
}

/// Generic reader loop that works with any `AsyncBufRead`, shared between
/// stdio ([`super::SharedGpuWorker`]) and TCP ([`super::SharedGpuTcpWorker`]).
pub(crate) async fn reader_loop_generic<R: tokio::io::AsyncBufRead + Unpin>(
    reader: &mut R,
    pending: Arc<std::sync::Mutex<HashMap<String, oneshot::Sender<ExecuteResponseV2>>>>,
    control: Arc<tokio::sync::Mutex<Option<oneshot::Sender<WorkerControlResponse>>>>,
    pid: WorkerPid,
) {
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => {
                debug!(pid = %pid, "GPU worker stream closed (EOF)");
                let mut pending = super::super::lock_recovered(&pending);
                for (id, tx) in pending.drain() {
                    debug!(pid = %pid, request_id = %id, "Failing pending request (worker stream closed)");
                    drop(tx);
                }
                break;
            }
            Ok(_) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }

                let parsed: Value = match serde_json::from_str(trimmed) {
                    Ok(v) => v,
                    Err(e) => {
                        warn!(
                            pid = %pid,
                            line = trimmed,
                            error = %e,
                            "GPU worker: ignoring non-JSON line"
                        );
                        continue;
                    }
                };

                let op = parsed.get("op").and_then(|v| v.as_str()).unwrap_or("");

                match op {
                    "execute_v2" => {
                        match serde_json::from_value::<ExecuteResponseV2Envelope>(parsed.clone()) {
                            Ok(envelope) => {
                                let request_id = envelope.response.request_id().to_string();
                                let mut pending = super::super::lock_recovered(&pending);
                                if let Some(tx) = pending.remove(&request_id) {
                                    let _ = tx.send(envelope.response);
                                } else {
                                    warn!(
                                        pid = %pid,
                                        request_id = %request_id,
                                        "GPU worker: orphaned execute_v2 response"
                                    );
                                }
                            }
                            Err(e) => {
                                // A refused response must still RESOLVE its
                                // pending dispatch, or the caller blocks for
                                // the full per-request timeout (1800 s,
                                // retried) on what is a millisecond-diagnosable
                                // protocol violation. The envelope did not
                                // parse, but the raw JSON is still in hand, so
                                // pull the correlation id from it, exactly as
                                // the tagged op=error arm below already does.
                                error!(
                                    pid = %pid,
                                    error = %e,
                                    "GPU worker: failed to parse execute_v2 response"
                                );
                                let request_id = parsed
                                    .get("response")
                                    .and_then(|r| r.get("request_id"))
                                    .and_then(|v| v.as_str())
                                    .map(WorkerRequestIdV2::from);
                                if let Some(request_id) = request_id {
                                    let routed = {
                                        let mut pending = super::super::lock_recovered(&pending);
                                        pending.remove(request_id.as_ref())
                                    };
                                    if let Some(tx) = routed {
                                        let _ = tx.send(ExecuteResponseV2::failure(
                                            request_id,
                                            ProtocolErrorCodeV2::InvalidPayload,
                                            format!(
                                                "worker sent an execute_v2 response the \
                                                 protocol refuses: {e}"
                                            ),
                                            DurationSeconds(0.0),
                                        ));
                                    }
                                }
                            }
                        }
                    }
                    "health" => {
                        if let Ok(envelope) =
                            serde_json::from_value::<HealthResponseEnvelope>(parsed)
                        {
                            let mut ctrl = control.lock().await;
                            if let Some(tx) = ctrl.take() {
                                let _ = tx.send(WorkerControlResponse::Health(envelope.response));
                            }
                        }
                    }
                    "capabilities" => {
                        if let Ok(envelope) =
                            serde_json::from_value::<CapabilitiesResponseEnvelope>(parsed)
                        {
                            let mut ctrl = control.lock().await;
                            if let Some(tx) = ctrl.take() {
                                let _ =
                                    tx.send(WorkerControlResponse::Capabilities(envelope.response));
                            }
                        }
                    }
                    "ensure_task" => {
                        if let Ok(envelope) =
                            serde_json::from_value::<EnsureTaskResponseEnvelope>(parsed)
                        {
                            let mut ctrl = control.lock().await;
                            if let Some(tx) = ctrl.take() {
                                let _ =
                                    tx.send(WorkerControlResponse::EnsureTask(envelope.response));
                            }
                        }
                    }
                    "shutdown" => {
                        let mut ctrl = control.lock().await;
                        if let Some(tx) = ctrl.take() {
                            let _ = tx.send(WorkerControlResponse::Shutdown);
                        }
                    }
                    "error" => {
                        let error_msg = parsed
                            .get("error")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown error")
                            .to_string();
                        // V2 dispatches register a pending oneshot keyed by
                        // request_id; sequential ops (health / capabilities /
                        // ensure_task) register a single-slot control receiver
                        // instead. The worker tags errors with `request_id`
                        // when the failure belongs to a V2 dispatch, untagged
                        // otherwise, so the correlation is read out of the
                        // envelope and every case of it is written out below.
                        let correlation = match parsed
                            .get("request_id")
                            .and_then(|v| v.as_str())
                            .map(WorkerRequestIdV2::from)
                        {
                            Some(request_id) => ErrorCorrelation::Dispatch(request_id),
                            None => ErrorCorrelation::Uncorrelated,
                        };

                        match correlation {
                            ErrorCorrelation::Dispatch(request_id) => {
                                let routed = {
                                    let mut pending = super::super::lock_recovered(&pending);
                                    pending.remove(request_id.as_ref())
                                };
                                match routed {
                                    Some(tx) => {
                                        debug!(
                                            pid = %pid,
                                            request_id = %request_id,
                                            error = %error_msg,
                                            "GPU worker: routing tagged error to pending V2 \
                                             dispatch"
                                        );
                                        let _ = tx.send(ExecuteResponseV2::failure(
                                            request_id,
                                            ProtocolErrorCodeV2::InvalidPayload,
                                            error_msg,
                                            DurationSeconds(0.0),
                                        ));
                                    }
                                    None => {
                                        // That caller already gave up (its
                                        // per-request timeout removed the
                                        // entry) or was answered. It says
                                        // nothing about the other dispatches
                                        // in flight, so none of them is
                                        // failed on it.
                                        warn!(
                                            pid = %pid,
                                            request_id = %request_id,
                                            error = %error_msg,
                                            "GPU worker: tagged error has no matching pending \
                                             dispatch (worker may have processed it \
                                             asynchronously)"
                                        );
                                        let _ = offer_to_sequential_op(&control, error_msg).await;
                                    }
                                }
                            }
                            ErrorCorrelation::Uncorrelated => {
                                if let Some(unclaimed) =
                                    offer_to_sequential_op(&control, error_msg).await
                                {
                                    fail_every_dispatch_in_flight(&pending, pid, &unclaimed);
                                }
                            }
                        }
                    }
                    _ => {
                        warn!(
                            pid = %pid,
                            op = op,
                            "GPU worker: unexpected response op"
                        );
                    }
                }
            }
            Err(e) => {
                error!(pid = %pid, error = %e, "GPU worker: stream read error");
                // Explicitly fail all pending requests, same as the EOF
                // path. Without this, pending oneshot senders are implicitly
                // dropped when the task exits, causing receivers to see
                // "channel closed" with no useful error context.
                let mut pending = super::super::lock_recovered(&pending);
                let n = pending.len();
                for (id, tx) in pending.drain() {
                    debug!(pid = %pid, request_id = %id, "Failing pending request (I/O error)");
                    drop(tx);
                }
                if n > 0 {
                    error!(
                        pid = %pid,
                        failed_requests = n,
                        error = %e,
                        "GPU worker crashed: failed {n} pending requests"
                    );
                }
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::worker_v2::ExecuteOutcomeRef;
    use tokio::io::BufReader;

    /// `op=error` envelopes tagged with `request_id` must fail the matching
    /// V2 dispatch's pending oneshot immediately, rather than be silently
    /// routed to the empty control channel and leave the dispatch sitting
    /// on its per-request timeout.
    /// An `op=execute_v2` line the strict response parse REFUSES (here a
    /// success with no result, unrepresentable since 2026-08-21) must still
    /// resolve the pending dispatch as a fast typed failure. The first
    /// version of the strict parse only logged the refusal, so the dispatch
    /// sat on its full per-request timeout (1800 s, retried) for a
    /// millisecond-diagnosable protocol violation.
    #[tokio::test]
    async fn refused_execute_v2_response_fails_pending_v2_dispatch() {
        let pid = WorkerPid(12346);
        let pending = Arc::new(std::sync::Mutex::new(HashMap::<
            String,
            oneshot::Sender<ExecuteResponseV2>,
        >::new()));
        let control = Arc::new(tokio::sync::Mutex::new(
            None::<oneshot::Sender<WorkerControlResponse>>,
        ));

        let (tx, rx) = oneshot::channel();
        let request_id = "asr-v2-request-88";
        super::super::super::lock_recovered(&pending).insert(request_id.into(), tx);

        let envelope = format!(
            "{{\"op\":\"execute_v2\",\"response\":{{\"request_id\":\"{request_id}\",             \"outcome\":{{\"kind\":\"success\"}},\"elapsed_s\":0.5}}}}\n",
        );
        let mut reader = BufReader::new(envelope.as_bytes());

        reader_loop_generic(&mut reader, pending.clone(), control.clone(), pid).await;

        let response = rx
            .await
            .expect("pending oneshot must resolve when the response is refused");
        assert_eq!(response.request_id().as_ref(), request_id);
        match response.read() {
            ExecuteOutcomeRef::Failed { code, message } => {
                assert_eq!(code, ProtocolErrorCodeV2::InvalidPayload);
                assert!(
                    message.contains("carried no result payload"),
                    "the refusal detail must reach the caller, got: {message}"
                );
            }
            ExecuteOutcomeRef::Success(_) => {
                panic!("a refused response must resolve as a failure")
            }
        }
    }

    #[tokio::test]
    async fn tagged_error_envelope_fails_pending_v2_dispatch() {
        let pid = WorkerPid(12345);
        let pending = Arc::new(std::sync::Mutex::new(HashMap::<
            String,
            oneshot::Sender<ExecuteResponseV2>,
        >::new()));
        let control = Arc::new(tokio::sync::Mutex::new(
            None::<oneshot::Sender<WorkerControlResponse>>,
        ));

        let (tx, rx) = oneshot::channel();
        let request_id = "asr-v2-request-77";
        super::super::super::lock_recovered(&pending).insert(request_id.into(), tx);

        let envelope = format!(
            "{{\"op\":\"error\",\"error\":\"invalid execute_v2 request: ValidationError\",\
             \"request_id\":\"{request_id}\"}}\n",
        );
        let mut reader = BufReader::new(envelope.as_bytes());

        reader_loop_generic(&mut reader, pending.clone(), control.clone(), pid).await;

        let response = rx
            .await
            .expect("pending oneshot must resolve when error is routed");
        assert_eq!(response.request_id().as_ref(), request_id);
        match response.read() {
            ExecuteOutcomeRef::Failed { code, message } => {
                assert_eq!(code, ProtocolErrorCodeV2::InvalidPayload);
                assert!(
                    message.contains("ValidationError"),
                    "expected worker error message to propagate, got: {message}"
                );
            }
            ExecuteOutcomeRef::Success(_) => {
                panic!("tagged error envelope must produce a Failed outcome")
            }
        }

        // The pending map should have been drained so a retry can rebind
        // the request_id without colliding.
        assert!(super::super::super::lock_recovered(&pending).is_empty());
    }

    /// An untagged `op=error` that NO sequential op is waiting for must still
    /// fail every pending V2 dispatch.
    ///
    /// This is the 2026-09-02 `speaker-identify` stall. The worker raised
    /// during dispatch, `_serve_stdio` emitted `{"op":"error"}` with no
    /// `request_id`, the empty control slot swallowed it, and the caller sat
    /// on its whole per-request timeout for a fault the worker had already
    /// diagnosed and reported two milliseconds after the request was written.
    /// An error nobody is waiting for is not an error nobody needs: a stream
    /// that has produced an uncorrelated failure cannot be trusted to answer
    /// the requests already on it, so every one of them is failed here.
    #[tokio::test]
    async fn untagged_error_with_no_sequential_op_fails_pending_v2_dispatches() {
        let pid = WorkerPid(12347);
        let pending = Arc::new(std::sync::Mutex::new(HashMap::<
            String,
            oneshot::Sender<ExecuteResponseV2>,
        >::new()));
        let control = Arc::new(tokio::sync::Mutex::new(
            None::<oneshot::Sender<WorkerControlResponse>>,
        ));

        let (tx, rx) = oneshot::channel();
        let request_id = "speaker-embedding-v2-request-1";
        super::super::super::lock_recovered(&pending).insert(request_id.into(), tx);

        let envelope = "{\"op\":\"error\",\"error\":\"module has no attribute                         execute_speaker_embedding_request_v2\",\"kind\":\"runtime\"}\n";
        let mut reader = BufReader::new(envelope.as_bytes());

        reader_loop_generic(&mut reader, pending.clone(), control.clone(), pid).await;

        let response = rx
            .await
            .expect("an uncorrelated worker error must still resolve the pending dispatch");
        assert_eq!(response.request_id().as_ref(), request_id);
        match response.read() {
            ExecuteOutcomeRef::Failed { code, message } => {
                assert_eq!(code, ProtocolErrorCodeV2::InvalidPayload);
                assert!(
                    message.contains("execute_speaker_embedding_request_v2"),
                    "the worker's own diagnosis must reach the caller, got: {message}"
                );
            }
            ExecuteOutcomeRef::Success(_) => {
                panic!("an uncorrelated worker error must produce a Failed outcome")
            }
        }

        assert!(super::super::super::lock_recovered(&pending).is_empty());
    }

    /// A TAGGED error for a request nobody is waiting for must NOT take the
    /// other dispatches down with it.
    ///
    /// The adversarial case for the fix above: that caller already gave up on
    /// its own timeout, so its late error says nothing about its neighbours,
    /// and the mass failure that is right for an UNCORRELATED error is wrong
    /// here. Written after a review of the fix found it failing this case.
    #[tokio::test]
    async fn tagged_error_for_an_unknown_request_leaves_other_dispatches_alone() {
        let pid = WorkerPid(12348);
        let pending = Arc::new(std::sync::Mutex::new(HashMap::<
            String,
            oneshot::Sender<ExecuteResponseV2>,
        >::new()));
        let control = Arc::new(tokio::sync::Mutex::new(
            None::<oneshot::Sender<WorkerControlResponse>>,
        ));

        let (tx, rx) = oneshot::channel();
        let survivor = "asr-v2-request-99";
        super::super::super::lock_recovered(&pending).insert(survivor.into(), tx);

        let envelope = "{\"op\":\"error\",\"error\":\"late failure\",\
                        \"request_id\":\"asr-v2-request-98\"}\n";
        let mut reader = BufReader::new(envelope.as_bytes());

        reader_loop_generic(&mut reader, pending.clone(), control.clone(), pid).await;

        // The loop runs to EOF, which drops every still-pending sender, so
        // the surviving dispatch must see a CLOSED channel (the worker went
        // away) and not a routed failure carrying somebody else's error.
        match rx.await {
            Err(_) => {}
            Ok(response) => panic!(
                "an unrelated dispatch was failed on another request's error: {:?}",
                response.read()
            ),
        }
    }

    /// Untagged `op=error` envelopes (no `request_id`) still flow through
    /// the control channel, preserving the sequential-op contract used by
    /// health / capabilities / ensure_task.
    #[tokio::test]
    async fn untagged_error_envelope_flows_to_control_channel() {
        let pid = WorkerPid(12345);
        let pending = Arc::new(std::sync::Mutex::new(HashMap::<
            String,
            oneshot::Sender<ExecuteResponseV2>,
        >::new()));
        let control = Arc::new(tokio::sync::Mutex::new(
            None::<oneshot::Sender<WorkerControlResponse>>,
        ));

        let (ctrl_tx, ctrl_rx) = oneshot::channel();
        {
            let mut slot = control.lock().await;
            *slot = Some(ctrl_tx);
        }

        let envelope = "{\"op\":\"error\",\"error\":\"capabilities-time crash\"}\n";
        let mut reader = BufReader::new(envelope.as_bytes());

        reader_loop_generic(&mut reader, pending.clone(), control.clone(), pid).await;

        let routed = ctrl_rx
            .await
            .expect("control channel must receive untagged error");
        match routed {
            WorkerControlResponse::Error(msg) => {
                assert!(msg.contains("capabilities-time crash"));
            }
            other => panic!("expected WorkerControlResponse::Error, got {other:?}"),
        }
    }
}
