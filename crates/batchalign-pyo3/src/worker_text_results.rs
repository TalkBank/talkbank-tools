//! Rust-owned normalization for worker-protocol V2 text-task batch results.
//!
//! **See also:** [INTERFACE_MAP.md](../../../INTERFACE_MAP.md) section "7. Text Task Result Normalization" for:
//! - Python caller: `batchalign/worker/_text_v2.py`
//! - Full Rust/Python responsibility split and input/output contracts.

use batchalign_transform::coref::CorefRawResponse;
use batchalign_types::worker::{BatchInferResponse, InferResponse};
use batchalign_types::worker_v2::{
    CorefAnnotationV2, CorefChainRefV2, CorefItemResultV2, CorefResultV2, MorphosyntaxItemResultV2,
    MorphosyntaxResultV2, TranslationItemResultV2, TranslationResultV2, UtsegItemResultV2,
    UtsegResultV2,
};
use pyo3::prelude::*;

/// Why a host response could not be normalized into a typed V2 payload.
///
/// A plain message: the CATEGORY is always the same (the host returned a
/// shape the contract does not allow), and the two consumers wrap it
/// differently at their own boundaries (the pyfunction raises it as an
/// internal error; the Rust text executor folds it into
/// `runtime_failure` with the established "invalid <task> host output"
/// prefix the Python test matrix asserts on).
pub(crate) struct TextResultShapeError(pub(crate) String);

fn normalize_result_count<'a>(
    response: &'a BatchInferResponse,
    expected_count: usize,
    task: &str,
) -> Result<&'a [InferResponse], TextResultShapeError> {
    let actual_count = response.results.len();
    if actual_count != expected_count {
        return Err(TextResultShapeError(format!(
            "worker protocol V2 {task} host returned {actual_count} items, expected {expected_count}"
        )));
    }
    Ok(response.results.as_slice())
}

fn response_object<'a>(
    result: Option<&'a serde_json::Value>,
    task: &str,
) -> Result<Option<&'a serde_json::Map<String, serde_json::Value>>, TextResultShapeError> {
    match result {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::Object(obj)) => Ok(Some(obj)),
        Some(_) => Err(TextResultShapeError(format!(
            "{task} V2 expected a JSON-object result"
        ))),
    }
}

fn normalize_morphosyntax_raw_sentences(
    result: Option<&serde_json::Value>,
) -> Result<Option<Vec<serde_json::Value>>, TextResultShapeError> {
    let Some(obj) = response_object(result, "morphosyntax")? else {
        return Ok(None);
    };

    if let Some(raw_sentences) = obj.get("raw_sentences") {
        return match raw_sentences {
            serde_json::Value::Array(items) => Ok(Some(items.clone())),
            _ => Err(TextResultShapeError(
                "morphosyntax V2 raw_sentences must be a list".to_owned(),
            )),
        };
    }

    match obj.get("sentences") {
        Some(serde_json::Value::Array(sentences)) if sentences.is_empty() => Ok(Some(Vec::new())),
        _ => Err(TextResultShapeError(
            "morphosyntax V2 expected raw_sentences in worker result".to_owned(),
        )),
    }
}

fn normalize_string_list(
    result: Option<&serde_json::Value>,
    field_name: &str,
    task: &str,
) -> Result<Option<Vec<String>>, TextResultShapeError> {
    let Some(obj) = response_object(result, task)? else {
        return Ok(None);
    };

    let Some(value) = obj.get(field_name) else {
        return Ok(None);
    };

    match value {
        serde_json::Value::Array(items) if items.iter().all(serde_json::Value::is_string) => {
            Ok(Some(
                items
                    .iter()
                    .map(|item| item.as_str().unwrap_or_default().to_owned())
                    .collect(),
            ))
        }
        _ => Err(TextResultShapeError(format!(
            "{task} V2 field {field_name:?} must be a list[str]"
        ))),
    }
}

fn normalize_usize_list(
    result: Option<&serde_json::Value>,
    field_name: &str,
    task: &str,
) -> Result<Option<Vec<usize>>, TextResultShapeError> {
    let Some(obj) = response_object(result, task)? else {
        return Ok(None);
    };

    let Some(value) = obj.get(field_name) else {
        return Ok(None);
    };

    match value {
        serde_json::Value::Array(items) => items
            .iter()
            .map(|item| match item {
                serde_json::Value::Number(value) => value
                    .as_u64()
                    .and_then(|number| usize::try_from(number).ok())
                    .ok_or_else(|| {
                        TextResultShapeError(format!(
                            "{task} V2 field {field_name:?} values must be non-negative integers"
                        ))
                    }),
                _ => Err(TextResultShapeError(format!(
                    "{task} V2 field {field_name:?} must be a list[usize]"
                ))),
            })
            .collect::<Result<Vec<_>, TextResultShapeError>>()
            .map(Some),
        _ => Err(TextResultShapeError(format!(
            "{task} V2 field {field_name:?} must be a list[usize]"
        ))),
    }
}

fn normalize_string_field(
    result: Option<&serde_json::Value>,
    field_name: &str,
    task: &str,
) -> Result<Option<String>, TextResultShapeError> {
    let Some(obj) = response_object(result, task)? else {
        return Ok(None);
    };

    let Some(value) = obj.get(field_name) else {
        return Ok(None);
    };

    match value {
        serde_json::Value::String(text) => Ok(Some(text.clone())),
        _ => Err(TextResultShapeError(format!(
            "{task} V2 field {field_name:?} must be a string"
        ))),
    }
}

fn normalize_coref_annotations(
    result: Option<&serde_json::Value>,
) -> Result<Option<Vec<CorefAnnotationV2>>, TextResultShapeError> {
    let Some(obj) = response_object(result, "coref")? else {
        return Ok(None);
    };

    let raw: CorefRawResponse = serde_json::from_value(serde_json::Value::Object(obj.clone()))
        .map_err(|error| {
            TextResultShapeError(format!(
                "coref V2 annotations must match CorefRawResponse: {error}"
            ))
        })?;

    Ok(Some(
        raw.annotations
            .into_iter()
            .map(|annotation| CorefAnnotationV2 {
                sentence_idx: annotation.sentence_idx,
                words: annotation
                    .words
                    .into_iter()
                    .map(|word_refs| {
                        word_refs
                            .into_iter()
                            .map(|chain_ref| CorefChainRefV2 {
                                chain_id: chain_ref.chain_id,
                                is_start: chain_ref.is_start,
                                is_end: chain_ref.is_end,
                            })
                            .collect()
                    })
                    .collect(),
            })
            .collect(),
    ))
}

pub(crate) fn normalize_morphosyntax_result(
    response: &BatchInferResponse,
    expected_count: usize,
) -> Result<MorphosyntaxResultV2, TextResultShapeError> {
    let payload = MorphosyntaxResultV2 {
        items: normalize_result_count(response, expected_count, "morphosyntax")?
            .iter()
            .map(|infer_result| {
                Ok(MorphosyntaxItemResultV2 {
                    raw_sentences: normalize_morphosyntax_raw_sentences(
                        infer_result.result.as_ref(),
                    )?,
                    error: infer_result.error.clone(),
                })
            })
            .collect::<Result<Vec<_>, TextResultShapeError>>()?,
    };
    Ok(payload)
}

pub(crate) fn normalize_utseg_result(
    response: &BatchInferResponse,
    expected_count: usize,
) -> Result<UtsegResultV2, TextResultShapeError> {
    let payload = UtsegResultV2 {
        items: normalize_result_count(response, expected_count, "utseg")?
            .iter()
            .map(|infer_result| {
                Ok(UtsegItemResultV2 {
                    assignments: normalize_usize_list(
                        infer_result.result.as_ref(),
                        "assignments",
                        "utseg",
                    )?,
                    trees: normalize_string_list(infer_result.result.as_ref(), "trees", "utseg")?,
                    error: infer_result.error.clone(),
                })
            })
            .collect::<Result<Vec<_>, TextResultShapeError>>()?,
    };
    Ok(payload)
}

pub(crate) fn normalize_translation_result(
    response: &BatchInferResponse,
    expected_count: usize,
) -> Result<TranslationResultV2, TextResultShapeError> {
    let payload = TranslationResultV2 {
        items: normalize_result_count(response, expected_count, "translate")?
            .iter()
            .map(|infer_result| {
                Ok(TranslationItemResultV2 {
                    raw_translation: normalize_string_field(
                        infer_result.result.as_ref(),
                        "raw_translation",
                        "translate",
                    )?,
                    error: infer_result.error.clone(),
                })
            })
            .collect::<Result<Vec<_>, TextResultShapeError>>()?,
    };
    Ok(payload)
}

pub(crate) fn normalize_coref_result(
    response: &BatchInferResponse,
    expected_count: usize,
) -> Result<CorefResultV2, TextResultShapeError> {
    let payload = CorefResultV2 {
        items: normalize_result_count(response, expected_count, "coref")?
            .iter()
            .map(|infer_result| {
                Ok(CorefItemResultV2 {
                    annotations: normalize_coref_annotations(infer_result.result.as_ref())?,
                    error: infer_result.error.clone(),
                })
            })
            .collect::<Result<Vec<_>, TextResultShapeError>>()?,
    };
    Ok(payload)
}

// ---------------------------------------------------------------------------
// Token alignment (used by Python morphosyntax tokenizer realignment)
// ---------------------------------------------------------------------------

/// Align Stanza tokenizer output back to original CHAT words.
///
/// Returns a Python list: plain `str` for unchanged tokens,
/// `(str, bool)` tuples for MWT expansion hints.
#[pyfunction]
pub(crate) fn align_tokens(
    py: Python<'_>,
    original_words: Vec<String>,
    stanza_tokens: Vec<String>,
    alpha2: String,
) -> PyResult<Py<pyo3::types::PyList>> {
    use batchalign_transform::tokenizer_realign::{self, PatchedToken};
    use pyo3::types::{PyBool, PyList, PyString, PyTuple};

    let patched =
        py.detach(|| tokenizer_realign::align_tokens(&original_words, &stanza_tokens, &alpha2));

    let result = PyList::empty(py);
    for tok in &patched {
        match tok {
            PatchedToken::Plain(s) => {
                result.append(PyString::new(py, s))?;
            }
            PatchedToken::Hint(s, expand) => {
                let s_any: Py<PyAny> = PyString::new(py, s).unbind().into_any();
                let b_any: Py<PyAny> = PyBool::new(py, *expand).to_owned().unbind().into_any();
                let tup = PyTuple::new(py, [s_any.bind(py), b_any.bind(py)])?;
                result.append(tup)?;
            }
        }
    }

    Ok(result.unbind())
}
