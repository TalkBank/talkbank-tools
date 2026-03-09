use crate::cli::ClanCommands;
use talkbank_clan::transforms::chstring::ChstringCommand;
use talkbank_clan::transforms::combtier::{CombtierCommand, CombtierConfig};
use talkbank_clan::transforms::compound::{CompoundCommand, CompoundConfig};
use talkbank_clan::transforms::dataclean::run_dataclean;
use talkbank_clan::transforms::dates::DatesCommand;
use talkbank_clan::transforms::delim::DelimCommand;
use talkbank_clan::transforms::fixbullets::{FixbulletsCommand, FixbulletsConfig};
use talkbank_clan::transforms::flo::FloCommand;
use talkbank_clan::transforms::gem::GemCommand;
use talkbank_clan::transforms::lines::{LinesConfig, run_lines};
use talkbank_clan::transforms::lowcase::LowcaseCommand;
use talkbank_clan::transforms::makemod::{MakemodCommand, MakemodConfig};
use talkbank_clan::transforms::ort::{OrtCommand, OrtConfig};
use talkbank_clan::transforms::postmortem::{PostmortemCommand, PostmortemConfig};
use talkbank_clan::transforms::quotes::run_quotes;
use talkbank_clan::transforms::repeat::RepeatCommand;
use talkbank_clan::transforms::retrace::RetraceCommand;
use talkbank_clan::transforms::roles::{RolesCommand, RolesConfig};
use talkbank_clan::transforms::tierorder::TierorderCommand;
use talkbank_clan::transforms::trim::{TrimCommand, TrimConfig};
use talkbank_model::SpeakerCode;

use super::helpers::{exit_with_error, run_transform_or_exit};

pub(super) fn dispatch(command: ClanCommands) -> Result<(), ClanCommands> {
    match command {
        ClanCommands::Flo { path, output } => {
            run_transform_or_exit(&FloCommand, &path, output.as_deref());
        }
        ClanCommands::Lowcase { path, output } => {
            run_transform_or_exit(&LowcaseCommand, &path, output.as_deref());
        }
        ClanCommands::Chstring {
            path,
            changes,
            output,
        } => {
            run_transform_or_exit(&ChstringCommand::new(changes), &path, output.as_deref());
        }
        ClanCommands::Dates { path, output } => {
            run_transform_or_exit(&DatesCommand, &path, output.as_deref());
        }
        ClanCommands::Delim { path, output } => {
            run_transform_or_exit(&DelimCommand, &path, output.as_deref());
        }
        ClanCommands::Fixbullets {
            path,
            offset,
            tier,
            exclude_tier,
            output,
        } => {
            let cmd = FixbulletsCommand::new(FixbulletsConfig {
                offset_ms: offset.unwrap_or(0),
                include_tiers: tier,
                exclude_tiers: exclude_tier,
            })
            .unwrap_or_else(|e| exit_with_error(format!("Error: {e}")));
            run_transform_or_exit(&cmd, &path, output.as_deref());
        }
        ClanCommands::Retrace { path, output } => {
            run_transform_or_exit(&RetraceCommand, &path, output.as_deref());
        }
        ClanCommands::Repeat {
            path,
            speaker,
            output,
        } => {
            let speaker_code = SpeakerCode::from(speaker);
            run_transform_or_exit(&RepeatCommand::new(speaker_code), &path, output.as_deref());
        }
        ClanCommands::Combtier {
            path,
            tier,
            separator,
            output,
        } => {
            let cmd = CombtierCommand::new(CombtierConfig { tier, separator });
            run_transform_or_exit(&cmd, &path, output.as_deref());
        }
        ClanCommands::Compound { path, output } => {
            let cmd = CompoundCommand::new(CompoundConfig { dash_to_plus: true });
            run_transform_or_exit(&cmd, &path, output.as_deref());
        }
        ClanCommands::Tierorder { path, output } => {
            run_transform_or_exit(&TierorderCommand, &path, output.as_deref());
        }
        ClanCommands::Lines {
            path,
            remove,
            output,
        } => {
            let config = LinesConfig { remove };
            if let Err(e) = run_lines(&config, &path, output.as_deref()) {
                exit_with_error(format!("Error: {e}"));
            }
        }
        ClanCommands::Dataclean { path, output } => {
            if let Err(e) = run_dataclean(&path, output.as_deref()) {
                exit_with_error(format!("Error: {e}"));
            }
        }
        ClanCommands::Quotes { path, output } => {
            if let Err(e) = run_quotes(&path, output.as_deref()) {
                exit_with_error(format!("Error: {e}"));
            }
        }
        ClanCommands::Ort {
            path,
            dictionary,
            output,
        } => {
            let cmd = OrtCommand::new(OrtConfig {
                dictionary_path: dictionary,
            })
            .unwrap_or_else(|e| exit_with_error(format!("Error: {e}")));
            run_transform_or_exit(&cmd, &path, output.as_deref());
        }
        ClanCommands::Postmortem {
            path,
            rules,
            target_tier,
            output,
        } => {
            let cmd = PostmortemCommand::new(PostmortemConfig {
                rules_path: rules,
                target_tier,
            })
            .unwrap_or_else(|e| exit_with_error(format!("Error: {e}")));
            run_transform_or_exit(&cmd, &path, output.as_deref());
        }
        ClanCommands::Makemod {
            path,
            lexicon,
            all_alternatives,
            output,
        } => {
            let cmd = MakemodCommand::new(MakemodConfig {
                lexicon_path: lexicon,
                all_alternatives,
            })
            .unwrap_or_else(|e| exit_with_error(format!("Error: {e}")));
            run_transform_or_exit(&cmd, &path, output.as_deref());
        }
        ClanCommands::Gem { path, gem, output } => {
            let cmd = GemCommand::new(gem);
            run_transform_or_exit(&cmd, &path, output.as_deref());
        }
        ClanCommands::Trim {
            path,
            tier,
            exclude_tier,
            output,
        } => {
            let cmd = TrimCommand {
                config: TrimConfig {
                    include_tiers: tier,
                    exclude_tiers: exclude_tier,
                },
            };
            run_transform_or_exit(&cmd, &path, output.as_deref());
        }
        ClanCommands::Roles {
            path,
            rename,
            output,
        } => {
            let renames: Vec<(String, String)> = rename
                .iter()
                .map(|r| {
                    let parts: Vec<&str> = r.splitn(2, '=').collect();
                    if parts.len() != 2 {
                        exit_with_error(
                            "Error: rename must be in format OLD=NEW (e.g., CHI=TARGET)".to_owned(),
                        );
                    }
                    (parts[0].to_owned(), parts[1].to_owned())
                })
                .collect();
            let cmd = RolesCommand {
                config: RolesConfig { renames },
            };
            run_transform_or_exit(&cmd, &path, output.as_deref());
        }
        other => return Err(other),
    }
    Ok(())
}
