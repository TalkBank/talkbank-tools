//! Rendering of utterance content with span tracking.
//!
//! References:
//! - <https://talkbank.org/0info/manuals/CHAT.html#Scoped_Symbols>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Retracing_and_Repetition>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Main_Tier>

use super::super::detection::is_retrace_annotation;
use super::bracketed;
use crate::Span;
use crate::model::{UtteranceContent, WriteChat};
use std::fmt::Write;

/// Render one utterance-content item while recording retrace annotation spans.
///
/// The emitted text is CHAT-formatted; span offsets are byte indices into the
/// `rendered` buffer and are later consumed by retrace validators.
pub fn render_utterance_content(
    item: &UtteranceContent,
    rendered: &mut String,
    retrace_spans: &mut Vec<Span>,
) {
    match item {
        UtteranceContent::Word(word) => {
            word.write_chat(rendered).ok();
        }
        UtteranceContent::AnnotatedWord(ann) => {
            ann.inner.write_chat(rendered).ok();
            render_scoped_annotations(ann.scoped_annotations.iter(), rendered, retrace_spans);
        }
        UtteranceContent::ReplacedWord(rw) => {
            rw.write_chat(rendered).ok();
            render_scoped_annotations(rw.scoped_annotations.iter(), rendered, retrace_spans);
        }
        UtteranceContent::Event(event) => {
            event.write_chat(rendered).ok();
        }
        UtteranceContent::AnnotatedEvent(ann) => {
            ann.inner.write_chat(rendered).ok();
            render_scoped_annotations(ann.scoped_annotations.iter(), rendered, retrace_spans);
        }
        UtteranceContent::Pause(pause) => {
            pause.write_chat(rendered).ok();
        }
        UtteranceContent::OtherSpokenEvent(event) => {
            event.write_chat(rendered).ok();
        }
        UtteranceContent::Group(group) => {
            rendered.push('<');
            bracketed::render_bracketed_content(&group.content, rendered, retrace_spans);
            if let Some(space) = &group.trailing_space {
                rendered.push_str(space);
            }
            rendered.push('>');
        }
        UtteranceContent::AnnotatedGroup(ann) => {
            rendered.push('<');
            bracketed::render_bracketed_content(&ann.inner.content, rendered, retrace_spans);
            if let Some(space) = &ann.inner.trailing_space {
                rendered.push_str(space);
            }
            rendered.push('>');
            render_scoped_annotations(ann.scoped_annotations.iter(), rendered, retrace_spans);
        }
        UtteranceContent::PhoGroup(pho) => {
            rendered.push('‹');
            bracketed::render_bracketed_content(&pho.content, rendered, retrace_spans);
            rendered.push('›');
        }
        UtteranceContent::SinGroup(sin) => {
            rendered.push('〔');
            bracketed::render_bracketed_content(&sin.content, rendered, retrace_spans);
            rendered.push('〕');
        }
        UtteranceContent::Quotation(quot) => {
            rendered.push('\u{201C}');
            bracketed::render_bracketed_content(&quot.content, rendered, retrace_spans);
            rendered.push('\u{201D}');
        }
        UtteranceContent::Separator(sep) => {
            sep.write_chat(rendered).ok();
        }
        UtteranceContent::OverlapPoint(marker) => {
            marker.write_chat(rendered).ok();
        }
        UtteranceContent::InternalBullet(bullet) => {
            bullet.write_chat(rendered).ok();
        }
        UtteranceContent::Freecode(freecode) => {
            freecode.write_chat(rendered).ok();
        }
        UtteranceContent::LongFeatureBegin(marker) => {
            marker.write_chat(rendered).ok();
        }
        UtteranceContent::LongFeatureEnd(marker) => {
            marker.write_chat(rendered).ok();
        }
        UtteranceContent::UnderlineBegin(_) => {
            write_with_span(rendered, |w| {
                w.write_char('\u{0002}')?;
                w.write_char('\u{0001}')
            });
        }
        UtteranceContent::UnderlineEnd(_) => {
            write_with_span(rendered, |w| {
                w.write_char('\u{0002}')?;
                w.write_char('\u{0002}')
            });
        }
        UtteranceContent::NonvocalBegin(marker) => {
            marker.write_chat(rendered).ok();
        }
        UtteranceContent::NonvocalEnd(marker) => {
            marker.write_chat(rendered).ok();
        }
        UtteranceContent::NonvocalSimple(marker) => {
            marker.write_chat(rendered).ok();
        }
        UtteranceContent::AnnotatedAction(ann) => {
            ann.inner.write_chat(rendered).ok();
            render_scoped_annotations(&ann.scoped_annotations, rendered, retrace_spans);
        }
    }
}

/// Render scoped annotations and capture spans for retrace-like markers.
fn render_scoped_annotations<'a>(
    annotations: impl IntoIterator<Item = &'a crate::model::ScopedAnnotation>,
    rendered: &mut String,
    retrace_spans: &mut Vec<Span>,
) {
    for ann in annotations {
        rendered.push(' ');
        if is_retrace_annotation(ann) {
            let span = write_with_span(rendered, |w| ann.write_chat(w));
            retrace_spans.push(span);
        } else {
            ann.write_chat(rendered).ok();
        }
    }
}

/// Write into `rendered` and return the written byte span.
///
/// This helper centralizes offset bookkeeping so higher-level renderers can
/// record exact source slices for downstream diagnostics.
fn write_with_span<F>(rendered: &mut String, mut write: F) -> Span
where
    F: FnMut(&mut String) -> std::fmt::Result,
{
    let start = rendered.len();
    write(rendered).ok();
    let end = rendered.len();
    Span::from_usize(start, end)
}
