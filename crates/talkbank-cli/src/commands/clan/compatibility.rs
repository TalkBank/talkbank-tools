//! Compatibility shims for CLAN-adjacent commands in the CLI.
//!
//! Not every command in this file belongs to the new shared analysis-service
//! boundary. CHECK and transform aliases still have their own execution paths,
//! while compatibility-style analysis entrypoints such as `gemfreq` adapt into
//! the library-owned typed analysis-command surface before delegating
//! execution.

use crate::cli::ClanCommands;
use talkbank_clan::commands::check::{CheckConfig, list_all_errors, run_check};
use talkbank_clan::framework::CommandOutput;
use talkbank_clan::service::{AnalysisCommandName, AnalysisOptions};

use super::helpers::{
    exit_with_error, read_file_or_exit, run_analysis_and_print, run_normalize_alias,
};

pub(super) fn dispatch(command: ClanCommands) -> Result<(), ClanCommands> {
    match command {
        ClanCommands::Check {
            path,
            bullets,
            include_errors,
            exclude_errors,
            list_errors,
            check_target,
            check_id,
            check_unused,
            check_ud,
        } => {
            if list_errors {
                print!("{}", list_all_errors());
                return Ok(());
            }

            let path = match path {
                Some(p) => p,
                None => exit_with_error(
                    "Error: path is required unless --list-errors is used".to_owned(),
                ),
            };

            let config = CheckConfig {
                bullets,
                include_errors: include_errors.into_iter().collect(),
                exclude_errors: exclude_errors.into_iter().collect(),
                list_errors: false,
                check_target_child: check_target,
                check_missing_id: check_id.unwrap_or(true),
                check_unused_speakers: check_unused,
                check_ud_features: check_ud,
            };

            let content = read_file_or_exit(&path);
            let result = run_check(&path, &content, &config);

            if result.errors.is_empty() && !result.has_errors {
                eprintln!("CHECK: no errors found.");
            } else {
                let output = result.render_text();
                if !output.is_empty() {
                    print!("{output}");
                }
                if result.has_errors {
                    eprintln!("Warning: Please repeat CHECK until no error messages are reported!");
                    std::process::exit(1);
                }
            }
        }
        ClanCommands::Fixit { path, output } => run_normalize_alias(&path, output.as_deref()),
        ClanCommands::Indent { path, output } => {
            talkbank_clan::transforms::indent::run_indent(&path, output.as_deref())
                .unwrap_or_else(|e| exit_with_error(format!("Error: {e}")));
        }
        ClanCommands::Longtier { path, output } => {
            talkbank_clan::transforms::longtier::run_longtier(&path, output.as_deref())
                .unwrap_or_else(|e| exit_with_error(format!("Error: {e}")));
        }
        ClanCommands::Gemfreq { path, mor, common } => {
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
        other => return Err(other),
    }
    Ok(())
}
