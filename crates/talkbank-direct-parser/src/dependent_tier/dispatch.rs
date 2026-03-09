use talkbank_model::ParseOutcome;
use talkbank_model::dependent_tier::DependentTier;
use talkbank_model::model::{NonEmptyString, ParseHealthTier};
use talkbank_model::{ErrorSink, Span};

use crate::{gra_tier, mor_tier, pho_tier, sin_tier, text_tier, wor_tier};

use super::helpers::{
    classify_dependent_tier_parse_health, make_unsupported_tier, make_user_defined_tier,
    parse_non_empty_text_tier, report_invalid_dependent_tier, split_tier_label_and_content,
    wrap_clean_parse,
};

/// Result of parsing a dependent tier with recovery taint information.
#[derive(Debug)]
pub(crate) enum TierParseResult {
    /// Tier parsed cleanly without errors.
    Clean(DependentTier),
    /// Tier parsed with some items recovered (skipped bad items).
    Recovered(DependentTier, ParseHealthTier),
    /// Tier parsing failed entirely.
    Failed(Option<ParseHealthTier>),
}

/// Parse a dependent tier with recovery taint propagation.
///
/// For structured tiers (%mor, %gra, %pho, %mod), uses the recovering
/// variants that skip bad items. Returns taint info for the file-level
/// grouper to use.
pub(crate) fn parse_dependent_tier_internal(
    input: &str,
    offset: usize,
    errors: &impl ErrorSink,
) -> TierParseResult {
    let (label, content, content_start) = match split_tier_label_and_content(input) {
        Some(parts) => parts,
        None => {
            report_invalid_dependent_tier(
                input,
                Span::from_usize(offset, offset + input.len()),
                errors,
                "Dependent tier must have format %name:\t<content>",
            );
            return TierParseResult::Failed(classify_dependent_tier_parse_health(input));
        }
    };

    let content_offset = offset + content_start;
    let label_offset = offset + 1;

    match label.as_bytes() {
        b"mor" => {
            let (outcome, had_errors) = mor_tier::parse_mor_tier_recovering(
                content,
                content_offset,
                errors,
                talkbank_model::dependent_tier::MorTierType::Mor,
            );
            match outcome {
                ParseOutcome::Parsed(tier) if had_errors => {
                    TierParseResult::Recovered(DependentTier::Mor(tier), ParseHealthTier::Mor)
                }
                ParseOutcome::Parsed(tier) => TierParseResult::Clean(DependentTier::Mor(tier)),
                ParseOutcome::Rejected => TierParseResult::Failed(Some(ParseHealthTier::Mor)),
            }
        }
        b"gra" => {
            let (outcome, had_errors) = gra_tier::parse_gra_tier_recovering(
                content,
                content_offset,
                errors,
                talkbank_model::dependent_tier::GraTierType::Gra,
            );
            match outcome {
                ParseOutcome::Parsed(tier) if had_errors => {
                    TierParseResult::Recovered(DependentTier::Gra(tier), ParseHealthTier::Gra)
                }
                ParseOutcome::Parsed(tier) => TierParseResult::Clean(DependentTier::Gra(tier)),
                ParseOutcome::Rejected => TierParseResult::Failed(Some(ParseHealthTier::Gra)),
            }
        }
        b"pho" => {
            let (outcome, had_errors) = pho_tier::parse_pho_tier_recovering(
                content,
                content_offset,
                errors,
                talkbank_model::dependent_tier::PhoTierType::Pho,
            );
            match outcome {
                ParseOutcome::Parsed(tier) if had_errors => {
                    TierParseResult::Recovered(DependentTier::Pho(tier), ParseHealthTier::Pho)
                }
                ParseOutcome::Parsed(tier) => TierParseResult::Clean(DependentTier::Pho(tier)),
                ParseOutcome::Rejected => TierParseResult::Failed(Some(ParseHealthTier::Pho)),
            }
        }
        b"mod" => {
            let (outcome, had_errors) = pho_tier::parse_pho_tier_recovering(
                content,
                content_offset,
                errors,
                talkbank_model::dependent_tier::PhoTierType::Mod,
            );
            match outcome {
                ParseOutcome::Parsed(tier) if had_errors => {
                    TierParseResult::Recovered(DependentTier::Mod(tier), ParseHealthTier::Mod)
                }
                ParseOutcome::Parsed(tier) => TierParseResult::Clean(DependentTier::Mod(tier)),
                ParseOutcome::Rejected => TierParseResult::Failed(Some(ParseHealthTier::Mod)),
            }
        }
        b"sin" => wrap_clean_parse(
            sin_tier::parse_sin_tier_content(content, content_offset, errors)
                .map(DependentTier::Sin),
        ),
        b"com" => wrap_clean_parse(
            text_tier::parse_com_tier_content(content, content_offset, errors)
                .map(DependentTier::Com),
        ),
        b"exp" => wrap_clean_parse(
            text_tier::parse_exp_tier_content(content, content_offset, errors)
                .map(DependentTier::Exp),
        ),
        b"add" => wrap_clean_parse(
            text_tier::parse_add_tier_content(content, content_offset, errors)
                .map(DependentTier::Add),
        ),
        b"gpx" => wrap_clean_parse(
            text_tier::parse_gpx_tier_content(content, content_offset, errors)
                .map(DependentTier::Gpx),
        ),
        b"int" => wrap_clean_parse(
            text_tier::parse_int_tier_content(content, content_offset, errors)
                .map(DependentTier::Int),
        ),
        b"spa" => wrap_clean_parse(
            text_tier::parse_spa_tier_content(content, content_offset, errors)
                .map(DependentTier::Spa),
        ),
        b"sit" => wrap_clean_parse(
            text_tier::parse_sit_tier_content(content, content_offset, errors)
                .map(DependentTier::Sit),
        ),
        b"act" => wrap_clean_parse(
            text_tier::parse_act_tier_content(content, content_offset, errors)
                .map(DependentTier::Act),
        ),
        b"cod" => wrap_clean_parse(
            text_tier::parse_cod_tier_content(content, content_offset, errors)
                .map(DependentTier::Cod),
        ),
        b"wor" => wrap_clean_parse(
            wor_tier::parse_wor_tier_content(content, content_offset, errors)
                .map(DependentTier::Wor),
        ),
        b"alt" => wrap_clean_parse(parse_non_empty_text_tier(
            input,
            "alt",
            content,
            content_offset,
            errors,
            DependentTier::Alt,
        )),
        b"coh" => wrap_clean_parse(parse_non_empty_text_tier(
            input,
            "coh",
            content,
            content_offset,
            errors,
            DependentTier::Coh,
        )),
        b"def" => wrap_clean_parse(parse_non_empty_text_tier(
            input,
            "def",
            content,
            content_offset,
            errors,
            DependentTier::Def,
        )),
        b"eng" => wrap_clean_parse(parse_non_empty_text_tier(
            input,
            "eng",
            content,
            content_offset,
            errors,
            DependentTier::Eng,
        )),
        b"err" => wrap_clean_parse(parse_non_empty_text_tier(
            input,
            "err",
            content,
            content_offset,
            errors,
            DependentTier::Err,
        )),
        b"fac" => wrap_clean_parse(parse_non_empty_text_tier(
            input,
            "fac",
            content,
            content_offset,
            errors,
            DependentTier::Fac,
        )),
        b"flo" => wrap_clean_parse(parse_non_empty_text_tier(
            input,
            "flo",
            content,
            content_offset,
            errors,
            DependentTier::Flo,
        )),
        b"gls" => wrap_clean_parse(parse_non_empty_text_tier(
            input,
            "gls",
            content,
            content_offset,
            errors,
            DependentTier::Gls,
        )),
        b"ort" => wrap_clean_parse(parse_non_empty_text_tier(
            input,
            "ort",
            content,
            content_offset,
            errors,
            DependentTier::Ort,
        )),
        b"par" => wrap_clean_parse(parse_non_empty_text_tier(
            input,
            "par",
            content,
            content_offset,
            errors,
            DependentTier::Par,
        )),
        b"tim" => {
            let Some(value) = NonEmptyString::new(content) else {
                report_invalid_dependent_tier(
                    input,
                    Span::from_usize(content_offset, content_offset + content.len()),
                    errors,
                    "%tim tier content cannot be empty",
                );
                return TierParseResult::Failed(None);
            };
            TierParseResult::Clean(DependentTier::Tim(
                talkbank_model::dependent_tier::TimTier::from_text(value),
            ))
        }
        b"xmodsyl" => {
            let words = talkbank_model::dependent_tier::parse_syl_content(content);
            TierParseResult::Clean(DependentTier::Modsyl(
                talkbank_model::dependent_tier::SylTier::new(
                    talkbank_model::dependent_tier::SylTierType::Modsyl,
                    words,
                ),
            ))
        }
        b"xphosyl" => {
            let words = talkbank_model::dependent_tier::parse_syl_content(content);
            TierParseResult::Clean(DependentTier::Phosyl(
                talkbank_model::dependent_tier::SylTier::new(
                    talkbank_model::dependent_tier::SylTierType::Phosyl,
                    words,
                ),
            ))
        }
        b"xphoaln" => match talkbank_model::dependent_tier::parse_phoaln_content(content) {
            Ok(words) => TierParseResult::Clean(DependentTier::Phoaln(
                talkbank_model::dependent_tier::PhoalnTier::new(words),
            )),
            Err(e) => {
                report_invalid_dependent_tier(
                    input,
                    Span::from_usize(content_offset, content_offset + content.len()),
                    errors,
                    format!("malformed %xphoaln content: {}", e),
                );
                TierParseResult::Failed(None)
            }
        },
        _ if label.starts_with('x') => wrap_clean_parse(make_user_defined_tier(
            input,
            label,
            label_offset,
            content,
            content_offset,
            errors,
        )),
        _ => wrap_clean_parse(make_unsupported_tier(
            input,
            label,
            label_offset,
            content,
            content_offset,
            errors,
        )),
    }
}
