//! CLAN analysis-command adapters for the `chatter` CLI.
//!
//! Each match arm here now performs only CLI-facing adaptation: convert parsed
//! clap arguments into a typed
//! [`talkbank_clan::service::AnalysisCommandName`] plus
//! [`talkbank_clan::service::AnalysisOptions`], then delegate defaults,
//! validation, and execution to the library-owned builder and service
//! boundaries. Keep command construction and shared output policy inside
//! `talkbank-clan`; keep CLI argument mapping and terminal UX here.

use crate::cli::ClanCommands;
use talkbank_clan::commands::chains::ChainsConfig;
use talkbank_clan::commands::codes::CodesConfig;
use talkbank_clan::commands::corelex::CorelexConfig;
use talkbank_clan::commands::dss::DssConfig;
use talkbank_clan::commands::ipsyn::IpsynConfig;
use talkbank_clan::commands::keymap::KeymapConfig;
use talkbank_clan::commands::maxwd::MaxwdConfig;
use talkbank_clan::commands::rely::RelyConfig;
use talkbank_clan::commands::trnfix::TrnfixConfig;
use talkbank_clan::service_types::{AnalysisCommandName, AnalysisOptions};

use super::helpers::{run_analysis_and_print, run_paired_analysis_and_print};

pub(super) fn dispatch(command: ClanCommands) -> Result<(), ClanCommands> {
    match command {
        ClanCommands::Freq {
            path, mor, common, ..
        } => {
            run_analysis_and_print(
                AnalysisCommandName::Freq,
                AnalysisOptions {
                    mor,
                    ..AnalysisOptions::default()
                },
                &path,
                &common,
            );
        }
        ClanCommands::Mlu {
            path,
            words,
            common,
            ..
        } => {
            run_analysis_and_print(
                AnalysisCommandName::Mlu,
                AnalysisOptions {
                    words,
                    ..AnalysisOptions::default()
                },
                &path,
                &common,
            );
        }
        ClanCommands::Mlt { path, common, .. } => run_analysis_and_print(
            AnalysisCommandName::Mlt,
            AnalysisOptions::default(),
            &path,
            &common,
        ),
        ClanCommands::Wdlen { path, common, .. } => run_analysis_and_print(
            AnalysisCommandName::Wdlen,
            AnalysisOptions::default(),
            &path,
            &common,
        ),
        ClanCommands::Wdsize {
            path,
            main_tier,
            common,
            ..
        } => {
            run_analysis_and_print(
                AnalysisCommandName::Wdsize,
                AnalysisOptions {
                    main_tier,
                    ..AnalysisOptions::default()
                },
                &path,
                &common,
            );
        }
        ClanCommands::Maxwd {
            path,
            limit,
            common,
            ..
        } => {
            run_analysis_and_print(
                AnalysisCommandName::Maxwd,
                AnalysisOptions {
                    limit: option_if_not_default(limit, MaxwdConfig::default().limit),
                    ..AnalysisOptions::default()
                },
                &path,
                &common,
            );
        }
        ClanCommands::Freqpos { path, common, .. } => run_analysis_and_print(
            AnalysisCommandName::Freqpos,
            AnalysisOptions::default(),
            &path,
            &common,
        ),
        ClanCommands::Timedur { path, common, .. } => run_analysis_and_print(
            AnalysisCommandName::Timedur,
            AnalysisOptions::default(),
            &path,
            &common,
        ),
        ClanCommands::Kwal {
            path,
            keyword,
            common,
            ..
        } => {
            run_analysis_and_print(
                AnalysisCommandName::Kwal,
                AnalysisOptions {
                    keywords: keyword
                        .into_iter()
                        .map(talkbank_clan::framework::KeywordPattern::from)
                        .collect(),
                    ..AnalysisOptions::default()
                },
                &path,
                &common,
            );
        }
        ClanCommands::Gemlist { path, common, .. } => run_analysis_and_print(
            AnalysisCommandName::Gemlist,
            AnalysisOptions::default(),
            &path,
            &common,
        ),
        ClanCommands::Combo {
            path,
            search,
            common,
            ..
        } => {
            run_analysis_and_print(
                AnalysisCommandName::Combo,
                AnalysisOptions {
                    search,
                    ..AnalysisOptions::default()
                },
                &path,
                &common,
            );
        }
        ClanCommands::Cooccur { path, common, .. } => run_analysis_and_print(
            AnalysisCommandName::Cooccur,
            AnalysisOptions::default(),
            &path,
            &common,
        ),
        ClanCommands::Dist { path, common, .. } => run_analysis_and_print(
            AnalysisCommandName::Dist,
            AnalysisOptions::default(),
            &path,
            &common,
        ),
        ClanCommands::Chip { path, common, .. } => run_analysis_and_print(
            AnalysisCommandName::Chip,
            AnalysisOptions::default(),
            &path,
            &common,
        ),
        ClanCommands::Phonfreq { path, common, .. } => run_analysis_and_print(
            AnalysisCommandName::Phonfreq,
            AnalysisOptions::default(),
            &path,
            &common,
        ),
        ClanCommands::Modrep { path, common, .. } => run_analysis_and_print(
            AnalysisCommandName::Modrep,
            AnalysisOptions::default(),
            &path,
            &common,
        ),
        ClanCommands::Vocd { path, common, .. } => run_analysis_and_print(
            AnalysisCommandName::Vocd,
            AnalysisOptions::default(),
            &path,
            &common,
        ),
        ClanCommands::Uniq {
            path, sort, common, ..
        } => {
            run_analysis_and_print(
                AnalysisCommandName::Uniq,
                AnalysisOptions {
                    sort_by_frequency: sort,
                    ..AnalysisOptions::default()
                },
                &path,
                &common,
            );
        }
        ClanCommands::Codes {
            path,
            max_depth,
            common,
            ..
        } => {
            run_analysis_and_print(
                AnalysisCommandName::Codes,
                AnalysisOptions {
                    max_depth: option_if_not_default(max_depth, CodesConfig::default().max_depth),
                    ..AnalysisOptions::default()
                },
                &path,
                &common,
            );
        }
        ClanCommands::Trnfix {
            path,
            tier1,
            tier2,
            common,
            ..
        } => {
            let default = TrnfixConfig::default();
            run_analysis_and_print(
                AnalysisCommandName::Trnfix,
                AnalysisOptions {
                    tier1: option_if_not_default(tier1, default.tier1),
                    tier2: option_if_not_default(tier2, default.tier2),
                    ..AnalysisOptions::default()
                },
                &path,
                &common,
            );
        }
        ClanCommands::Sugar { path, common, .. } => {
            // CLAN's sugar refuses without `+t*<SPK>`. Match its
            // exact stderr message.
            if common.speaker.is_empty() {
                super::helpers::exit_with_clan_refusal(
                    "Please specify at least one speaker tier code with \"+t\" option on command line.",
                );
            }
            run_analysis_and_print(
                AnalysisCommandName::Sugar,
                AnalysisOptions::default(),
                &path,
                &common,
            );
        }
        ClanCommands::Mortable {
            path,
            script,
            common,
            ..
        } => {
            // CLAN's mortable refuses without `+l<script>`. Match
            // CLAN's two-line refusal message exactly.
            let script = script.unwrap_or_else(|| {
                super::helpers::exit_with_clan_refusal(
                    "Please specify language script file name with \"+l\" option.\n\
                     For example, \"mortable +leng\" or \"mortable +leng.cut\".",
                )
            });
            run_analysis_and_print(
                AnalysisCommandName::Mortable,
                AnalysisOptions {
                    script_path: Some(script),
                    ..AnalysisOptions::default()
                },
                &path,
                &common,
            );
        }
        ClanCommands::Chains {
            path, tier, common, ..
        } => {
            // CLAN's chains refuses without `+t<tier>` — emit the
            // same message on stderr and exit non-zero before any
            // banner is printed.
            let tier = tier.unwrap_or_else(|| {
                super::helpers::exit_with_clan_refusal(
                    "Please specify a code tier with \"+t\" option.",
                )
            });
            run_analysis_and_print(
                AnalysisCommandName::Chains,
                AnalysisOptions {
                    tier: Some(tier),
                    ..AnalysisOptions::default()
                },
                &path,
                &common,
            );
        }
        ClanCommands::Complexity { path, common, .. } => run_analysis_and_print(
            AnalysisCommandName::Complexity,
            AnalysisOptions::default(),
            &path,
            &common,
        ),
        ClanCommands::Corelex {
            path,
            threshold,
            common,
            ..
        } => {
            run_analysis_and_print(
                AnalysisCommandName::Corelex,
                AnalysisOptions {
                    threshold: option_if_not_default(
                        threshold,
                        CorelexConfig::default().min_frequency,
                    ),
                    ..AnalysisOptions::default()
                },
                &path,
                &common,
            );
        }
        ClanCommands::Keymap {
            path,
            keyword,
            tier,
            common,
            ..
        } => {
            run_analysis_and_print(
                AnalysisCommandName::Keymap,
                AnalysisOptions {
                    keywords: keyword
                        .into_iter()
                        .map(talkbank_clan::framework::KeywordPattern::from)
                        .collect(),
                    tier: option_if_not_default(tier, KeymapConfig::default().tier),
                    ..AnalysisOptions::default()
                },
                &path,
                &common,
            );
        }
        ClanCommands::Script {
            path,
            template,
            common,
            ..
        } => {
            run_analysis_and_print(
                AnalysisCommandName::Script,
                AnalysisOptions {
                    template_path: Some(template),
                    ..AnalysisOptions::default()
                },
                &path,
                &common,
            );
        }
        ClanCommands::Rely {
            file1,
            file2,
            tier,
            format,
        } => {
            run_paired_analysis_and_print(
                AnalysisCommandName::Rely,
                AnalysisOptions {
                    second_file: Some(file2),
                    tier: option_if_not_default(tier, RelyConfig::default().tier),
                    ..AnalysisOptions::default()
                },
                &file1,
                format,
            );
        }
        ClanCommands::Flucalc { path, common, .. } => run_analysis_and_print(
            AnalysisCommandName::Flucalc,
            AnalysisOptions::default(),
            &path,
            &common,
        ),
        ClanCommands::Dss {
            path,
            rules,
            max_utterances,
            common,
            ..
        } => {
            // CLAN's dss refuses without `+t*<SPK>` (speaker tier).
            // Mirror that here: require at least one `--speaker`.
            if common.speaker.is_empty() {
                super::helpers::exit_with_clan_refusal(
                    "Please specify at least one speaker tier name with \"+t\" option.",
                );
            }
            run_analysis_and_print(
                AnalysisCommandName::Dss,
                AnalysisOptions {
                    rules_path: rules,
                    max_utterances: option_if_not_default(
                        max_utterances,
                        DssConfig::default().max_utterances,
                    ),
                    ..AnalysisOptions::default()
                },
                &path,
                &common,
            );
        }
        ClanCommands::Ipsyn {
            path,
            rules,
            max_utterances,
            common,
            ..
        } => {
            // CLAN's ipsyn refuses without `+l<rules>`. Match the
            // exact two-line message including the example
            // continuation.
            if rules.is_none() {
                super::helpers::exit_with_clan_refusal(
                    "Please specify ipsyn rules file name with \"+l\" option.\n\
                     For example, \"ipsyn +leng\" or \"ipsyn +leng.cut\".",
                );
            }
            run_analysis_and_print(
                AnalysisCommandName::Ipsyn,
                AnalysisOptions {
                    rules_path: rules,
                    max_utterances: option_if_not_default(
                        max_utterances,
                        IpsynConfig::default().max_utterances,
                    ),
                    ..AnalysisOptions::default()
                },
                &path,
                &common,
            );
        }
        ClanCommands::Eval { path, common, .. } => {
            // CLAN's eval refuses without `+t*<SPK>`.
            if common.speaker.is_empty() {
                super::helpers::exit_with_clan_refusal(
                    "Please specify at least one speaker tier code with \"+t\" option on command line.",
                );
            }
            run_analysis_and_print(
                AnalysisCommandName::Eval,
                AnalysisOptions::default(),
                &path,
                &common,
            );
        }
        ClanCommands::Kideval {
            path,
            dss_rules,
            ipsyn_rules,
            common,
            ..
        } => {
            // CLAN's kideval refuses without `+l<script>` (the
            // language file that bundles DSS + IPSYN + EVAL rules
            // for one language). chatter has separate --dss-rules
            // and --ipsyn-rules flags; require at least one so the
            // refusal triggers when neither is given. Match CLAN's
            // exact two-line wording.
            if dss_rules.is_none() && ipsyn_rules.is_none() {
                // CLAN's kideval refusal includes a leading blank
                // line; preserve that quirk for byte-level parity.
                super::helpers::exit_with_clan_refusal(
                    "\nPlease specify language script file name with \"+l\" option.\n\
                     For example, \"kideval +leng\" or \"kideval +leng.cut\".",
                );
            }
            run_analysis_and_print(
                AnalysisCommandName::Kideval,
                AnalysisOptions {
                    dss_rules_path: dss_rules,
                    ipsyn_rules_path: ipsyn_rules,
                    ..AnalysisOptions::default()
                },
                &path,
                &common,
            );
        }
        ClanCommands::EvalD { path, common, .. } => {
            // CLAN's eval-d refuses without `+t*<SPK>` (same as eval).
            if common.speaker.is_empty() {
                super::helpers::exit_with_clan_refusal(
                    "Please specify at least one speaker tier code with \"+t\" option on command line.",
                );
            }
            run_analysis_and_print(
                AnalysisCommandName::EvalDialect,
                AnalysisOptions::default(),
                &path,
                &common,
            );
        }
        other => return Err(other),
    }
    Ok(())
}

fn option_if_not_default<T: PartialEq>(value: T, default: T) -> Option<T> {
    if value == default { None } else { Some(value) }
}
