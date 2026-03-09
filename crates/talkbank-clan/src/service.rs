//! High-level library service for executing CLAN analyses.
//!
//! `talkbank-clan` already exposes low-level command traits and runner plumbing in
//! [`crate::framework`], but editor integrations should not need to import every
//! individual command type just to execute a named analysis. This module keeps the
//! higher-level analysis execution boundary inside the library so CLI and LSP
//! wrappers can stay focused on adapting outer request shapes. That boundary
//! now also includes a typed analysis-command identifier so outer adapters do
//! not need to route analysis work through raw command-name strings.

use std::fmt;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::commands::chains::{ChainsCommand, ChainsConfig};
use crate::commands::chip::ChipCommand;
use crate::commands::codes::{CodesCommand, CodesConfig};
use crate::commands::combo::{ComboCommand, ComboConfig};
use crate::commands::complexity::ComplexityCommand;
use crate::commands::cooccur::CooccurCommand;
use crate::commands::corelex::{CorelexCommand, CorelexConfig};
use crate::commands::dist::DistCommand;
use crate::commands::dss::{DssCommand, DssConfig};
use crate::commands::eval::{EvalCommand, EvalConfig};
use crate::commands::flucalc::{FlucalcCommand, FlucalcConfig};
use crate::commands::freq::{FreqCommand, FreqConfig};
use crate::commands::freqpos::FreqposCommand;
use crate::commands::gemlist::GemlistCommand;
use crate::commands::ipsyn::{IpsynCommand, IpsynConfig};
use crate::commands::keymap::{KeymapCommand, KeymapConfig};
use crate::commands::kideval::{KidevalCommand, KidevalConfig};
use crate::commands::kwal::{KwalCommand, KwalConfig};
use crate::commands::maxwd::{MaxwdCommand, MaxwdConfig};
use crate::commands::mlt::MltCommand;
use crate::commands::mlu::{MluCommand, MluConfig};
use crate::commands::modrep::ModrepCommand;
use crate::commands::mortable::{MortableCommand, MortableConfig};
use crate::commands::phonfreq::PhonfreqCommand;
use crate::commands::rely::{RelyConfig, run_rely};
use crate::commands::script::{ScriptCommand, ScriptConfig};
use crate::commands::sugar::{SugarCommand, SugarConfig};
use crate::commands::timedur::TimedurCommand;
use crate::commands::trnfix::{TrnfixCommand, TrnfixConfig};
use crate::commands::uniq::{UniqCommand, UniqConfig};
use crate::commands::vocd::VocdCommand;
use crate::commands::wdlen::WdlenCommand;
use crate::commands::wdsize::{WdsizeCommand, WdsizeConfig};
use crate::framework::{
    AnalysisCommand, AnalysisRunner, CommandOutput, FilterConfig, OutputFormat, RunnerError,
    TransformError,
};

/// Typed identifier for one supported CLAN analysis command.
///
/// Outer adapters such as the CLI and LSP should parse raw command-name
/// strings into this enum at their boundary, then pass the typed identifier
/// through the shared builder and service layers.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum AnalysisCommandName {
    /// `freq`
    Freq,
    /// `mlu`
    Mlu,
    /// `mlt`
    Mlt,
    /// `wdlen`
    Wdlen,
    /// `wdsize`
    Wdsize,
    /// `maxwd`
    Maxwd,
    /// `freqpos`
    Freqpos,
    /// `timedur`
    Timedur,
    /// `kwal`
    Kwal,
    /// `gemlist`
    Gemlist,
    /// `combo`
    Combo,
    /// `cooccur`
    Cooccur,
    /// `dist`
    Dist,
    /// `chip`
    Chip,
    /// `phonfreq`
    Phonfreq,
    /// `modrep`
    Modrep,
    /// `vocd`
    Vocd,
    /// `codes`
    Codes,
    /// `chains`
    Chains,
    /// `complexity`
    Complexity,
    /// `corelex`
    Corelex,
    /// `dss`
    Dss,
    /// `eval`
    Eval,
    /// `eval-d`
    #[serde(rename = "eval-d")]
    EvalDialect,
    /// `flucalc`
    Flucalc,
    /// `ipsyn`
    Ipsyn,
    /// `keymap`
    Keymap,
    /// `kideval`
    Kideval,
    /// `mortable`
    Mortable,
    /// `rely`
    Rely,
    /// `script`
    Script,
    /// `sugar`
    Sugar,
    /// `trnfix`
    Trnfix,
    /// `uniq`
    Uniq,
}

impl AnalysisCommandName {
    /// Return the stable wire-format name used by CLI and editor adapters.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Freq => "freq",
            Self::Mlu => "mlu",
            Self::Mlt => "mlt",
            Self::Wdlen => "wdlen",
            Self::Wdsize => "wdsize",
            Self::Maxwd => "maxwd",
            Self::Freqpos => "freqpos",
            Self::Timedur => "timedur",
            Self::Kwal => "kwal",
            Self::Gemlist => "gemlist",
            Self::Combo => "combo",
            Self::Cooccur => "cooccur",
            Self::Dist => "dist",
            Self::Chip => "chip",
            Self::Phonfreq => "phonfreq",
            Self::Modrep => "modrep",
            Self::Vocd => "vocd",
            Self::Codes => "codes",
            Self::Chains => "chains",
            Self::Complexity => "complexity",
            Self::Corelex => "corelex",
            Self::Dss => "dss",
            Self::Eval => "eval",
            Self::EvalDialect => "eval-d",
            Self::Flucalc => "flucalc",
            Self::Ipsyn => "ipsyn",
            Self::Keymap => "keymap",
            Self::Kideval => "kideval",
            Self::Mortable => "mortable",
            Self::Rely => "rely",
            Self::Script => "script",
            Self::Sugar => "sugar",
            Self::Trnfix => "trnfix",
            Self::Uniq => "uniq",
        }
    }
}

impl fmt::Display for AnalysisCommandName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Error returned when a raw outer-layer command name is not recognized.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
#[error("Unknown analysis command: {command_name}")]
pub struct ParseAnalysisCommandNameError {
    command_name: String,
}

impl FromStr for AnalysisCommandName {
    type Err = ParseAnalysisCommandNameError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "freq" => Ok(Self::Freq),
            "mlu" => Ok(Self::Mlu),
            "mlt" => Ok(Self::Mlt),
            "wdlen" => Ok(Self::Wdlen),
            "wdsize" => Ok(Self::Wdsize),
            "maxwd" => Ok(Self::Maxwd),
            "freqpos" => Ok(Self::Freqpos),
            "timedur" => Ok(Self::Timedur),
            "kwal" => Ok(Self::Kwal),
            "gemlist" => Ok(Self::Gemlist),
            "combo" => Ok(Self::Combo),
            "cooccur" => Ok(Self::Cooccur),
            "dist" => Ok(Self::Dist),
            "chip" => Ok(Self::Chip),
            "phonfreq" => Ok(Self::Phonfreq),
            "modrep" => Ok(Self::Modrep),
            "vocd" => Ok(Self::Vocd),
            "codes" => Ok(Self::Codes),
            "chains" => Ok(Self::Chains),
            "complexity" => Ok(Self::Complexity),
            "corelex" => Ok(Self::Corelex),
            "dss" => Ok(Self::Dss),
            "eval" => Ok(Self::Eval),
            "eval-d" => Ok(Self::EvalDialect),
            "flucalc" => Ok(Self::Flucalc),
            "ipsyn" => Ok(Self::Ipsyn),
            "keymap" => Ok(Self::Keymap),
            "kideval" => Ok(Self::Kideval),
            "mortable" => Ok(Self::Mortable),
            "rely" => Ok(Self::Rely),
            "script" => Ok(Self::Script),
            "sugar" => Ok(Self::Sugar),
            "trnfix" => Ok(Self::Trnfix),
            "uniq" => Ok(Self::Uniq),
            _ => Err(ParseAnalysisCommandNameError {
                command_name: value.to_owned(),
            }),
        }
    }
}

impl<'de> Deserialize<'de> for AnalysisCommandName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        value.parse().map_err(serde::de::Error::custom)
    }
}

/// Typed, library-owned request for a CLAN analysis command.
///
/// This enum is the stable integration boundary for higher-level consumers such
/// as the CLI and LSP. It keeps command-specific configuration typed without
/// forcing those outer layers to import and execute each command type directly.
#[derive(Debug)]
pub enum AnalysisRequest {
    /// `freq`
    Freq(FreqConfig),
    /// `mlu`
    Mlu(MluConfig),
    /// `mlt`
    Mlt,
    /// `wdlen`
    Wdlen,
    /// `wdsize`
    Wdsize(WdsizeConfig),
    /// `maxwd`
    Maxwd(MaxwdConfig),
    /// `freqpos`
    Freqpos,
    /// `timedur`
    Timedur,
    /// `kwal`
    Kwal(KwalConfig),
    /// `gemlist`
    Gemlist,
    /// `combo`
    Combo(ComboConfig),
    /// `cooccur`
    Cooccur,
    /// `dist`
    Dist,
    /// `chip`
    Chip,
    /// `phonfreq`
    Phonfreq,
    /// `modrep`
    Modrep,
    /// `vocd`
    Vocd,
    /// `codes`
    Codes(CodesConfig),
    /// `chains`
    Chains(ChainsConfig),
    /// `complexity`
    Complexity,
    /// `corelex`
    Corelex(CorelexConfig),
    /// `dss`
    Dss(DssConfig),
    /// `eval`
    Eval(EvalConfig),
    /// `flucalc`
    Flucalc(FlucalcConfig),
    /// `ipsyn`
    Ipsyn(IpsynConfig),
    /// `keymap`
    Keymap(KeymapConfig),
    /// `kideval`
    Kideval(KidevalConfig),
    /// `mortable`
    Mortable(MortableConfig),
    /// `script`
    Script(ScriptConfig),
    /// `sugar`
    Sugar(SugarConfig),
    /// `trnfix`
    Trnfix(TrnfixConfig),
    /// `uniq`
    Uniq(UniqConfig),
}

/// Raw analysis options supplied by outer adapters before defaults are applied.
#[derive(Debug, Clone, Default)]
pub struct AnalysisOptions {
    /// Whether to run `freq` against `%mor`.
    pub mor: bool,
    /// Whether `mlu` should count words instead of morphemes.
    pub words: bool,
    /// Whether `wdsize` should read from the main tier.
    pub main_tier: bool,
    /// Result limit for commands such as `maxwd`.
    pub limit: Option<usize>,
    /// Keyword list for `kwal` and `keymap`.
    pub keywords: Vec<String>,
    /// Search expressions for `combo`.
    pub search: Vec<String>,
    /// Maximum code depth for `codes`.
    pub max_depth: Option<usize>,
    /// Tier selector used by several analysis commands.
    pub tier: Option<String>,
    /// Frequency threshold for `corelex`.
    pub threshold: Option<u64>,
    /// Shared max-utterance limit used by several analyzers.
    pub max_utterances: Option<usize>,
    /// Optional normative database path.
    pub database_path: Option<PathBuf>,
    /// Optional normative database demographic filter.
    pub database_filter: Option<crate::database::DatabaseFilter>,
    /// Whether `flucalc` should use syllable mode.
    pub syllable_mode: bool,
    /// KidEval DSS utterance cap.
    pub dss_max_utterances: Option<usize>,
    /// KidEval IPSyn utterance cap.
    pub ipsyn_max_utterances: Option<usize>,
    /// Shared rules path for analyzers such as `dss` and `ipsyn`.
    pub rules_path: Option<PathBuf>,
    /// Kideval DSS rules path.
    pub dss_rules_path: Option<PathBuf>,
    /// Kideval IPSyn rules path.
    pub ipsyn_rules_path: Option<PathBuf>,
    /// Mortable script path.
    pub script_path: Option<PathBuf>,
    /// Secondary file for `rely`.
    pub second_file: Option<PathBuf>,
    /// Template file path for `script`.
    pub template_path: Option<PathBuf>,
    /// Minimum utterance count for `sugar`.
    pub min_utterances: Option<usize>,
    /// First tier for `trnfix`.
    pub tier1: Option<String>,
    /// Second tier for `trnfix`.
    pub tier2: Option<String>,
    /// Whether `uniq` should sort by frequency.
    pub sort_by_frequency: bool,
}

/// Built analysis plan after library-owned defaults and validation are applied.
#[derive(Debug)]
pub enum AnalysisPlan {
    /// Standard request executed through [`AnalysisService`].
    Service(AnalysisRequest),
    /// `rely` still uses an explicit two-file execution path.
    Rely(RelyRequest),
}

/// Typed request for `rely`.
#[derive(Debug)]
pub struct RelyRequest {
    /// Parsed secondary file path.
    pub secondary_file: PathBuf,
    /// Validated RELY configuration.
    pub config: RelyConfig,
}

/// Builder that turns raw outer-layer options into typed library requests.
pub struct AnalysisRequestBuilder {
    command_name: AnalysisCommandName,
    options: AnalysisOptions,
}

impl AnalysisRequestBuilder {
    /// Create a builder for one named CLAN analysis command.
    pub fn new(command_name: AnalysisCommandName, options: AnalysisOptions) -> Self {
        Self {
            command_name,
            options,
        }
    }

    /// Validate, apply library defaults, and build the analysis plan.
    pub fn build(self) -> Result<AnalysisPlan, AnalysisServiceError> {
        let command_name = self.command_name;
        let options = self.options;

        match command_name {
            AnalysisCommandName::Freq => {
                Ok(AnalysisPlan::Service(AnalysisRequest::Freq(FreqConfig {
                    use_mor: options.mor,
                })))
            }
            AnalysisCommandName::Mlu => {
                Ok(AnalysisPlan::Service(AnalysisRequest::Mlu(MluConfig {
                    words_only: options.words,
                })))
            }
            AnalysisCommandName::Mlt => Ok(AnalysisPlan::Service(AnalysisRequest::Mlt)),
            AnalysisCommandName::Wdlen => Ok(AnalysisPlan::Service(AnalysisRequest::Wdlen)),
            AnalysisCommandName::Wdsize => Ok(AnalysisPlan::Service(AnalysisRequest::Wdsize(
                WdsizeConfig {
                    use_main_tier: options.main_tier,
                },
            ))),
            AnalysisCommandName::Maxwd => {
                let default = MaxwdConfig::default();
                Ok(AnalysisPlan::Service(AnalysisRequest::Maxwd(MaxwdConfig {
                    limit: options.limit.unwrap_or(default.limit),
                })))
            }
            AnalysisCommandName::Freqpos => Ok(AnalysisPlan::Service(AnalysisRequest::Freqpos)),
            AnalysisCommandName::Timedur => Ok(AnalysisPlan::Service(AnalysisRequest::Timedur)),
            AnalysisCommandName::Kwal => Ok(AnalysisPlan::Service(AnalysisRequest::kwal(
                options.keywords,
            )?)),
            AnalysisCommandName::Gemlist => Ok(AnalysisPlan::Service(AnalysisRequest::Gemlist)),
            AnalysisCommandName::Combo => {
                let search: Vec<crate::commands::combo::SearchExpr> = options
                    .search
                    .iter()
                    .map(|expr| crate::commands::combo::SearchExpr::parse(expr))
                    .collect();
                if search.is_empty() {
                    return Err(AnalysisServiceError::InvalidRequest(
                        "combo requires at least one search expression".to_owned(),
                    ));
                }
                Ok(AnalysisPlan::Service(AnalysisRequest::Combo(ComboConfig {
                    search,
                })))
            }
            AnalysisCommandName::Cooccur => Ok(AnalysisPlan::Service(AnalysisRequest::Cooccur)),
            AnalysisCommandName::Dist => Ok(AnalysisPlan::Service(AnalysisRequest::Dist)),
            AnalysisCommandName::Chip => Ok(AnalysisPlan::Service(AnalysisRequest::Chip)),
            AnalysisCommandName::Phonfreq => Ok(AnalysisPlan::Service(AnalysisRequest::Phonfreq)),
            AnalysisCommandName::Modrep => Ok(AnalysisPlan::Service(AnalysisRequest::Modrep)),
            AnalysisCommandName::Vocd => Ok(AnalysisPlan::Service(AnalysisRequest::Vocd)),
            AnalysisCommandName::Codes => {
                Ok(AnalysisPlan::Service(AnalysisRequest::Codes(CodesConfig {
                    max_depth: options.max_depth.unwrap_or_default(),
                })))
            }
            AnalysisCommandName::Chains => {
                let default = ChainsConfig::default();
                Ok(AnalysisPlan::Service(AnalysisRequest::Chains(
                    ChainsConfig {
                        tier: options.tier.unwrap_or(default.tier),
                    },
                )))
            }
            AnalysisCommandName::Complexity => {
                Ok(AnalysisPlan::Service(AnalysisRequest::Complexity))
            }
            AnalysisCommandName::Corelex => {
                let default = CorelexConfig::default();
                Ok(AnalysisPlan::Service(AnalysisRequest::Corelex(
                    CorelexConfig {
                        min_frequency: options.threshold.unwrap_or(default.min_frequency),
                    },
                )))
            }
            AnalysisCommandName::Dss => {
                let default = DssConfig::default();
                Ok(AnalysisPlan::Service(AnalysisRequest::Dss(DssConfig {
                    rules_path: options.rules_path,
                    max_utterances: options.max_utterances.unwrap_or(default.max_utterances),
                })))
            }
            AnalysisCommandName::Eval => {
                let default = EvalConfig::default();
                Ok(AnalysisPlan::Service(AnalysisRequest::Eval(EvalConfig {
                    database_path: options.database_path,
                    database_filter: options.database_filter,
                    ..default
                })))
            }
            AnalysisCommandName::EvalDialect => {
                Ok(AnalysisPlan::Service(AnalysisRequest::Eval(EvalConfig {
                    database_path: options.database_path,
                    database_filter: options.database_filter,
                    variant: crate::commands::eval::EvalVariant::Dialect,
                })))
            }
            AnalysisCommandName::Flucalc => Ok(AnalysisPlan::Service(AnalysisRequest::Flucalc(
                FlucalcConfig {
                    syllable_mode: options.syllable_mode,
                },
            ))),
            AnalysisCommandName::Ipsyn => {
                let default = IpsynConfig::default();
                Ok(AnalysisPlan::Service(AnalysisRequest::Ipsyn(IpsynConfig {
                    rules_path: options.rules_path,
                    max_utterances: options.max_utterances.unwrap_or(default.max_utterances),
                })))
            }
            AnalysisCommandName::Keymap => {
                let tier = options.tier.unwrap_or_else(|| KeymapConfig::default().tier);
                Ok(AnalysisPlan::Service(AnalysisRequest::keymap(
                    options.keywords,
                    tier,
                )?))
            }
            AnalysisCommandName::Kideval => {
                let default = KidevalConfig::default();
                Ok(AnalysisPlan::Service(AnalysisRequest::Kideval(
                    KidevalConfig {
                        dss_rules_path: options.dss_rules_path,
                        ipsyn_rules_path: options.ipsyn_rules_path,
                        dss_max_utterances: options
                            .dss_max_utterances
                            .unwrap_or(default.dss_max_utterances),
                        ipsyn_max_utterances: options
                            .ipsyn_max_utterances
                            .unwrap_or(default.ipsyn_max_utterances),
                        database_path: options.database_path,
                        database_filter: options.database_filter,
                    },
                )))
            }
            AnalysisCommandName::Mortable => {
                let script_path = options.script_path.ok_or_else(|| {
                    AnalysisServiceError::InvalidRequest(
                        "mortable requires a scriptPath option".to_owned(),
                    )
                })?;
                Ok(AnalysisPlan::Service(AnalysisRequest::Mortable(
                    MortableConfig { script_path },
                )))
            }
            AnalysisCommandName::Rely => {
                let secondary_file = options.second_file.ok_or_else(|| {
                    AnalysisServiceError::InvalidRequest(
                        "rely requires a secondFile option".to_owned(),
                    )
                })?;
                let tier = options.tier.unwrap_or_else(|| RelyConfig::default().tier);
                Ok(AnalysisPlan::Rely(RelyRequest {
                    secondary_file,
                    config: RelyConfig { tier },
                }))
            }
            AnalysisCommandName::Script => {
                let template_path = options.template_path.ok_or_else(|| {
                    AnalysisServiceError::InvalidRequest(
                        "script requires a templatePath option".to_owned(),
                    )
                })?;
                Ok(AnalysisPlan::Service(AnalysisRequest::Script(
                    ScriptConfig { template_path },
                )))
            }
            AnalysisCommandName::Sugar => {
                let default = SugarConfig::default();
                Ok(AnalysisPlan::Service(AnalysisRequest::Sugar(SugarConfig {
                    min_utterances: options.min_utterances.unwrap_or(default.min_utterances),
                })))
            }
            AnalysisCommandName::Trnfix => {
                let default = TrnfixConfig::default();
                Ok(AnalysisPlan::Service(AnalysisRequest::Trnfix(
                    TrnfixConfig {
                        tier1: options.tier1.unwrap_or(default.tier1),
                        tier2: options.tier2.unwrap_or(default.tier2),
                    },
                )))
            }
            AnalysisCommandName::Uniq => {
                Ok(AnalysisPlan::Service(AnalysisRequest::Uniq(UniqConfig {
                    sort_by_frequency: options.sort_by_frequency,
                })))
            }
        }
    }
}

impl AnalysisRequest {
    /// Validate and construct a `kwal` request.
    pub fn kwal(keywords: Vec<String>) -> Result<Self, AnalysisServiceError> {
        if keywords.is_empty() {
            return Err(AnalysisServiceError::InvalidRequest(
                "kwal requires at least one keyword".to_owned(),
            ));
        }

        Ok(Self::Kwal(KwalConfig { keywords }))
    }

    /// Validate and construct a `keymap` request.
    pub fn keymap(keywords: Vec<String>, tier: String) -> Result<Self, AnalysisServiceError> {
        if keywords.is_empty() {
            return Err(AnalysisServiceError::InvalidRequest(
                "keymap requires at least one keyword".to_owned(),
            ));
        }

        Ok(Self::Keymap(KeymapConfig { keywords, tier }))
    }
}

/// Error from the high-level analysis service boundary.
#[derive(Debug, Error)]
pub enum AnalysisServiceError {
    /// Invalid request shape or unsupported option combination.
    #[error("{0}")]
    InvalidRequest(String),
    /// Underlying transform failure used by non-runner commands such as `rely`.
    #[error(transparent)]
    Transform(#[from] TransformError),
    /// Underlying runner failure.
    #[error(transparent)]
    Runner(#[from] RunnerError),
}

/// Library-owned analysis execution service.
///
/// The service owns one [`AnalysisRunner`] plus output-shape policy for common
/// integration scenarios. CLI and editor adapters should translate their outer
/// request types into [`AnalysisRequest`] values and then delegate execution
/// here instead of open-coding command construction and output handling.
pub struct AnalysisService {
    runner: AnalysisRunner,
}

impl AnalysisService {
    /// Construct a service with default pass-through filtering.
    pub fn new() -> Self {
        Self {
            runner: AnalysisRunner::new(),
        }
    }

    /// Construct a service with the given filter configuration.
    pub fn with_filter(filter: FilterConfig) -> Self {
        Self {
            runner: AnalysisRunner::with_filter(filter),
        }
    }

    /// Execute one analysis request and return structured JSON output.
    pub fn execute_json(
        &self,
        request: AnalysisRequest,
        files: &[PathBuf],
    ) -> Result<Value, AnalysisServiceError> {
        match request {
            AnalysisRequest::Freq(config) => self.run_json(&FreqCommand::new(config), files),
            AnalysisRequest::Mlu(config) => self.run_json(&MluCommand::new(config), files),
            AnalysisRequest::Mlt => self.run_json(&MltCommand, files),
            AnalysisRequest::Wdlen => self.run_json(&WdlenCommand, files),
            AnalysisRequest::Wdsize(config) => self.run_json(&WdsizeCommand::new(config), files),
            AnalysisRequest::Maxwd(config) => self.run_json(&MaxwdCommand::new(config), files),
            AnalysisRequest::Freqpos => self.run_json(&FreqposCommand, files),
            AnalysisRequest::Timedur => self.run_json(&TimedurCommand, files),
            AnalysisRequest::Kwal(config) => self.run_json(&KwalCommand::new(config), files),
            AnalysisRequest::Gemlist => self.run_json(&GemlistCommand, files),
            AnalysisRequest::Combo(config) => self.run_json(&ComboCommand::new(config), files),
            AnalysisRequest::Cooccur => self.run_json(&CooccurCommand, files),
            AnalysisRequest::Dist => self.run_json(&DistCommand, files),
            AnalysisRequest::Chip => self.run_json(&ChipCommand, files),
            AnalysisRequest::Phonfreq => self.run_json(&PhonfreqCommand, files),
            AnalysisRequest::Modrep => self.run_json(&ModrepCommand, files),
            AnalysisRequest::Vocd => self.run_json(&VocdCommand::default(), files),
            AnalysisRequest::Codes(config) => self.run_json(&CodesCommand::new(config), files),
            AnalysisRequest::Chains(config) => self.run_json(&ChainsCommand::new(config), files),
            AnalysisRequest::Complexity => self.run_json(&ComplexityCommand, files),
            AnalysisRequest::Corelex(config) => self.run_json(&CorelexCommand::new(config), files),
            AnalysisRequest::Dss(config) => {
                let command = DssCommand::new(config)
                    .map_err(|error| AnalysisServiceError::InvalidRequest(error.to_string()))?;
                self.run_json(&command, files)
            }
            AnalysisRequest::Eval(config) => self.run_json(&EvalCommand::new(config), files),
            AnalysisRequest::Flucalc(config) => self.run_json(&FlucalcCommand::new(config), files),
            AnalysisRequest::Ipsyn(config) => {
                let command = IpsynCommand::new(config)
                    .map_err(|error| AnalysisServiceError::InvalidRequest(error.to_string()))?;
                self.run_json(&command, files)
            }
            AnalysisRequest::Keymap(config) => self.run_json(&KeymapCommand::new(config), files),
            AnalysisRequest::Kideval(config) => {
                let command = KidevalCommand::new(config)
                    .map_err(|error| AnalysisServiceError::InvalidRequest(error.to_string()))?;
                self.run_json(&command, files)
            }
            AnalysisRequest::Mortable(config) => {
                let command = MortableCommand::new(config)
                    .map_err(|error| AnalysisServiceError::InvalidRequest(error.to_string()))?;
                self.run_json(&command, files)
            }
            AnalysisRequest::Script(config) => {
                let command = ScriptCommand::new(config)
                    .map_err(|error| AnalysisServiceError::InvalidRequest(error.to_string()))?;
                self.run_json(&command, files)
            }
            AnalysisRequest::Sugar(config) => self.run_json(&SugarCommand::new(config), files),
            AnalysisRequest::Trnfix(config) => self.run_json(&TrnfixCommand::new(config), files),
            AnalysisRequest::Uniq(config) => self.run_json(&UniqCommand::new(config), files),
        }
    }

    /// Execute one `rely` request and return structured JSON output.
    pub fn execute_rely_json(
        &self,
        request: RelyRequest,
        primary_file: &Path,
    ) -> Result<Value, AnalysisServiceError> {
        let result = run_rely(&request.config, primary_file, &request.secondary_file)?;
        serde_json::to_value(&result).map_err(|error| {
            AnalysisServiceError::InvalidRequest(format!(
                "Failed to serialize rely result: {error}"
            ))
        })
    }

    /// Execute one analysis request and render aggregate output in the requested format.
    pub fn execute_rendered(
        &self,
        request: AnalysisRequest,
        files: &[PathBuf],
        format: OutputFormat,
    ) -> Result<String, AnalysisServiceError> {
        match request {
            AnalysisRequest::Freq(config) => {
                self.run_rendered(&FreqCommand::new(config), files, format)
            }
            AnalysisRequest::Mlu(config) => {
                self.run_rendered(&MluCommand::new(config), files, format)
            }
            AnalysisRequest::Mlt => self.run_rendered(&MltCommand, files, format),
            AnalysisRequest::Wdlen => self.run_rendered(&WdlenCommand, files, format),
            AnalysisRequest::Wdsize(config) => {
                self.run_rendered(&WdsizeCommand::new(config), files, format)
            }
            AnalysisRequest::Maxwd(config) => {
                self.run_rendered(&MaxwdCommand::new(config), files, format)
            }
            AnalysisRequest::Freqpos => self.run_rendered(&FreqposCommand, files, format),
            AnalysisRequest::Timedur => self.run_rendered(&TimedurCommand, files, format),
            AnalysisRequest::Kwal(config) => {
                self.run_rendered(&KwalCommand::new(config), files, format)
            }
            AnalysisRequest::Gemlist => self.run_rendered(&GemlistCommand, files, format),
            AnalysisRequest::Combo(config) => {
                self.run_rendered(&ComboCommand::new(config), files, format)
            }
            AnalysisRequest::Cooccur => self.run_rendered(&CooccurCommand, files, format),
            AnalysisRequest::Dist => self.run_rendered(&DistCommand, files, format),
            AnalysisRequest::Chip => self.run_rendered(&ChipCommand, files, format),
            AnalysisRequest::Phonfreq => self.run_rendered(&PhonfreqCommand, files, format),
            AnalysisRequest::Modrep => self.run_rendered(&ModrepCommand, files, format),
            AnalysisRequest::Vocd => self.run_rendered(&VocdCommand::default(), files, format),
            AnalysisRequest::Codes(config) => {
                self.run_rendered(&CodesCommand::new(config), files, format)
            }
            AnalysisRequest::Chains(config) => {
                self.run_rendered(&ChainsCommand::new(config), files, format)
            }
            AnalysisRequest::Complexity => self.run_rendered(&ComplexityCommand, files, format),
            AnalysisRequest::Corelex(config) => {
                self.run_rendered(&CorelexCommand::new(config), files, format)
            }
            AnalysisRequest::Dss(config) => {
                let command = DssCommand::new(config)
                    .map_err(|error| AnalysisServiceError::InvalidRequest(error.to_string()))?;
                self.run_rendered(&command, files, format)
            }
            AnalysisRequest::Eval(config) => {
                self.run_rendered(&EvalCommand::new(config), files, format)
            }
            AnalysisRequest::Flucalc(config) => {
                self.run_rendered(&FlucalcCommand::new(config), files, format)
            }
            AnalysisRequest::Ipsyn(config) => {
                let command = IpsynCommand::new(config)
                    .map_err(|error| AnalysisServiceError::InvalidRequest(error.to_string()))?;
                self.run_rendered(&command, files, format)
            }
            AnalysisRequest::Keymap(config) => {
                self.run_rendered(&KeymapCommand::new(config), files, format)
            }
            AnalysisRequest::Kideval(config) => {
                let command = KidevalCommand::new(config)
                    .map_err(|error| AnalysisServiceError::InvalidRequest(error.to_string()))?;
                self.run_rendered(&command, files, format)
            }
            AnalysisRequest::Mortable(config) => {
                let command = MortableCommand::new(config)
                    .map_err(|error| AnalysisServiceError::InvalidRequest(error.to_string()))?;
                self.run_rendered(&command, files, format)
            }
            AnalysisRequest::Script(config) => {
                let command = ScriptCommand::new(config)
                    .map_err(|error| AnalysisServiceError::InvalidRequest(error.to_string()))?;
                self.run_rendered(&command, files, format)
            }
            AnalysisRequest::Sugar(config) => {
                self.run_rendered(&SugarCommand::new(config), files, format)
            }
            AnalysisRequest::Trnfix(config) => {
                self.run_rendered(&TrnfixCommand::new(config), files, format)
            }
            AnalysisRequest::Uniq(config) => {
                self.run_rendered(&UniqCommand::new(config), files, format)
            }
        }
    }

    /// Execute one `rely` request and render output in the requested format.
    pub fn execute_rely_rendered(
        &self,
        request: RelyRequest,
        primary_file: &Path,
        format: OutputFormat,
    ) -> Result<String, AnalysisServiceError> {
        let result = run_rely(&request.config, primary_file, &request.secondary_file)?;
        Ok(result.render(format))
    }

    /// Execute one analysis request in per-file mode and render each result.
    pub fn execute_rendered_per_file(
        &self,
        request: AnalysisRequest,
        files: &[PathBuf],
        format: OutputFormat,
    ) -> Result<Vec<(PathBuf, String)>, AnalysisServiceError> {
        match request {
            AnalysisRequest::Freq(config) => {
                self.run_rendered_per_file(&FreqCommand::new(config), files, format)
            }
            AnalysisRequest::Mlu(config) => {
                self.run_rendered_per_file(&MluCommand::new(config), files, format)
            }
            AnalysisRequest::Mlt => self.run_rendered_per_file(&MltCommand, files, format),
            AnalysisRequest::Wdlen => self.run_rendered_per_file(&WdlenCommand, files, format),
            AnalysisRequest::Wdsize(config) => {
                self.run_rendered_per_file(&WdsizeCommand::new(config), files, format)
            }
            AnalysisRequest::Maxwd(config) => {
                self.run_rendered_per_file(&MaxwdCommand::new(config), files, format)
            }
            AnalysisRequest::Freqpos => self.run_rendered_per_file(&FreqposCommand, files, format),
            AnalysisRequest::Timedur => self.run_rendered_per_file(&TimedurCommand, files, format),
            AnalysisRequest::Kwal(config) => {
                self.run_rendered_per_file(&KwalCommand::new(config), files, format)
            }
            AnalysisRequest::Gemlist => self.run_rendered_per_file(&GemlistCommand, files, format),
            AnalysisRequest::Combo(config) => {
                self.run_rendered_per_file(&ComboCommand::new(config), files, format)
            }
            AnalysisRequest::Cooccur => self.run_rendered_per_file(&CooccurCommand, files, format),
            AnalysisRequest::Dist => self.run_rendered_per_file(&DistCommand, files, format),
            AnalysisRequest::Chip => self.run_rendered_per_file(&ChipCommand, files, format),
            AnalysisRequest::Phonfreq => {
                self.run_rendered_per_file(&PhonfreqCommand, files, format)
            }
            AnalysisRequest::Modrep => self.run_rendered_per_file(&ModrepCommand, files, format),
            AnalysisRequest::Vocd => {
                self.run_rendered_per_file(&VocdCommand::default(), files, format)
            }
            AnalysisRequest::Codes(config) => {
                self.run_rendered_per_file(&CodesCommand::new(config), files, format)
            }
            AnalysisRequest::Chains(config) => {
                self.run_rendered_per_file(&ChainsCommand::new(config), files, format)
            }
            AnalysisRequest::Complexity => {
                self.run_rendered_per_file(&ComplexityCommand, files, format)
            }
            AnalysisRequest::Corelex(config) => {
                self.run_rendered_per_file(&CorelexCommand::new(config), files, format)
            }
            AnalysisRequest::Dss(config) => {
                let command = DssCommand::new(config)
                    .map_err(|error| AnalysisServiceError::InvalidRequest(error.to_string()))?;
                self.run_rendered_per_file(&command, files, format)
            }
            AnalysisRequest::Eval(config) => {
                self.run_rendered_per_file(&EvalCommand::new(config), files, format)
            }
            AnalysisRequest::Flucalc(config) => {
                self.run_rendered_per_file(&FlucalcCommand::new(config), files, format)
            }
            AnalysisRequest::Ipsyn(config) => {
                let command = IpsynCommand::new(config)
                    .map_err(|error| AnalysisServiceError::InvalidRequest(error.to_string()))?;
                self.run_rendered_per_file(&command, files, format)
            }
            AnalysisRequest::Keymap(config) => {
                self.run_rendered_per_file(&KeymapCommand::new(config), files, format)
            }
            AnalysisRequest::Kideval(config) => {
                let command = KidevalCommand::new(config)
                    .map_err(|error| AnalysisServiceError::InvalidRequest(error.to_string()))?;
                self.run_rendered_per_file(&command, files, format)
            }
            AnalysisRequest::Mortable(config) => {
                let command = MortableCommand::new(config)
                    .map_err(|error| AnalysisServiceError::InvalidRequest(error.to_string()))?;
                self.run_rendered_per_file(&command, files, format)
            }
            AnalysisRequest::Script(config) => {
                let command = ScriptCommand::new(config)
                    .map_err(|error| AnalysisServiceError::InvalidRequest(error.to_string()))?;
                self.run_rendered_per_file(&command, files, format)
            }
            AnalysisRequest::Sugar(config) => {
                self.run_rendered_per_file(&SugarCommand::new(config), files, format)
            }
            AnalysisRequest::Trnfix(config) => {
                self.run_rendered_per_file(&TrnfixCommand::new(config), files, format)
            }
            AnalysisRequest::Uniq(config) => {
                self.run_rendered_per_file(&UniqCommand::new(config), files, format)
            }
        }
    }

    fn run_json<C: AnalysisCommand>(
        &self,
        command: &C,
        files: &[PathBuf],
    ) -> Result<Value, AnalysisServiceError>
    where
        C::Output: CommandOutput,
    {
        let output = self.runner.run(command, files)?;
        Ok(output.to_json_value())
    }

    fn run_rendered<C: AnalysisCommand>(
        &self,
        command: &C,
        files: &[PathBuf],
        format: OutputFormat,
    ) -> Result<String, AnalysisServiceError>
    where
        C::Output: CommandOutput,
    {
        let output = self.runner.run(command, files)?;
        Ok(output.render(format))
    }

    fn run_rendered_per_file<C: AnalysisCommand>(
        &self,
        command: &C,
        files: &[PathBuf],
        format: OutputFormat,
    ) -> Result<Vec<(PathBuf, String)>, AnalysisServiceError>
    where
        C::Output: CommandOutput,
    {
        let outputs = self.runner.run_per_file(command, files)?;
        Ok(outputs
            .into_iter()
            .map(|(path, output)| (path, output.render(format)))
            .collect())
    }
}

impl Default for AnalysisService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analysis_command_name_round_trips_wire_names() {
        let commands = [
            AnalysisCommandName::Freq,
            AnalysisCommandName::Mlu,
            AnalysisCommandName::Mlt,
            AnalysisCommandName::Wdlen,
            AnalysisCommandName::Wdsize,
            AnalysisCommandName::Maxwd,
            AnalysisCommandName::Freqpos,
            AnalysisCommandName::Timedur,
            AnalysisCommandName::Kwal,
            AnalysisCommandName::Gemlist,
            AnalysisCommandName::Combo,
            AnalysisCommandName::Cooccur,
            AnalysisCommandName::Dist,
            AnalysisCommandName::Chip,
            AnalysisCommandName::Phonfreq,
            AnalysisCommandName::Modrep,
            AnalysisCommandName::Vocd,
            AnalysisCommandName::Codes,
            AnalysisCommandName::Chains,
            AnalysisCommandName::Complexity,
            AnalysisCommandName::Corelex,
            AnalysisCommandName::Dss,
            AnalysisCommandName::Eval,
            AnalysisCommandName::EvalDialect,
            AnalysisCommandName::Flucalc,
            AnalysisCommandName::Ipsyn,
            AnalysisCommandName::Keymap,
            AnalysisCommandName::Kideval,
            AnalysisCommandName::Mortable,
            AnalysisCommandName::Rely,
            AnalysisCommandName::Script,
            AnalysisCommandName::Sugar,
            AnalysisCommandName::Trnfix,
            AnalysisCommandName::Uniq,
        ];

        for command in commands {
            let parsed = command
                .as_str()
                .parse::<AnalysisCommandName>()
                .expect("command name should parse");
            assert_eq!(parsed, command);
            assert_eq!(parsed.to_string(), command.as_str());
        }
    }

    #[test]
    fn analysis_command_name_rejects_unknown_strings() {
        let error = "not-real"
            .parse::<AnalysisCommandName>()
            .expect_err("unknown command should fail");
        assert_eq!(error.to_string(), "Unknown analysis command: not-real");
    }

    #[test]
    fn builder_uses_corelex_library_default() {
        let plan =
            AnalysisRequestBuilder::new(AnalysisCommandName::Corelex, AnalysisOptions::default())
                .build()
                .expect("corelex should build");

        match plan {
            AnalysisPlan::Service(AnalysisRequest::Corelex(config)) => {
                assert_eq!(config.min_frequency, CorelexConfig::default().min_frequency);
            }
            other => panic!("unexpected plan: {other:?}"),
        }
    }

    #[test]
    fn builder_uses_sugar_library_default() {
        let plan =
            AnalysisRequestBuilder::new(AnalysisCommandName::Sugar, AnalysisOptions::default())
                .build()
                .expect("sugar should build");

        match plan {
            AnalysisPlan::Service(AnalysisRequest::Sugar(config)) => {
                assert_eq!(config.min_utterances, SugarConfig::default().min_utterances);
            }
            other => panic!("unexpected plan: {other:?}"),
        }
    }

    #[test]
    fn builder_uses_default_tiers() {
        let chains =
            AnalysisRequestBuilder::new(AnalysisCommandName::Chains, AnalysisOptions::default())
                .build()
                .expect("chains should build");
        let trnfix =
            AnalysisRequestBuilder::new(AnalysisCommandName::Trnfix, AnalysisOptions::default())
                .build()
                .expect("trnfix should build");

        match chains {
            AnalysisPlan::Service(AnalysisRequest::Chains(config)) => {
                assert_eq!(config.tier, ChainsConfig::default().tier);
            }
            other => panic!("unexpected plan: {other:?}"),
        }

        match trnfix {
            AnalysisPlan::Service(AnalysisRequest::Trnfix(config)) => {
                let default = TrnfixConfig::default();
                assert_eq!(config.tier1, default.tier1);
                assert_eq!(config.tier2, default.tier2);
            }
            other => panic!("unexpected plan: {other:?}"),
        }
    }

    #[test]
    fn builder_requires_rely_second_file() {
        let error =
            AnalysisRequestBuilder::new(AnalysisCommandName::Rely, AnalysisOptions::default())
                .build()
                .expect_err("rely without second file should fail");
        assert!(matches!(
            error,
            AnalysisServiceError::InvalidRequest(message) if message.contains("secondFile")
        ));
    }

    #[test]
    fn builder_uses_rely_default_tier() {
        let options = AnalysisOptions {
            second_file: Some(PathBuf::from("/tmp/other.cha")),
            ..AnalysisOptions::default()
        };
        let plan = AnalysisRequestBuilder::new(AnalysisCommandName::Rely, options)
            .build()
            .expect("rely should build");

        match plan {
            AnalysisPlan::Rely(request) => {
                assert_eq!(request.config.tier, RelyConfig::default().tier);
            }
            other => panic!("unexpected plan: {other:?}"),
        }
    }
}
