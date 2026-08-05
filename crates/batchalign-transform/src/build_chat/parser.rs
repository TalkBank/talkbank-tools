use super::BuildChatError;
use talkbank_model::LanguageCode;
use talkbank_parser::TreeSitterParser;

use super::TranscriptDescription;

/// Shared parser and language defaults for one `build_chat` invocation.
pub(super) struct BuildChatContext {
    parser: TreeSitterParser,
    langs: Vec<LanguageCode>,
    primary_lang: LanguageCode,
}

impl BuildChatContext {
    /// Create the parser and normalize transcript-level language defaults once.
    pub(super) fn new(desc: &TranscriptDescription) -> Result<Self, BuildChatError> {
        let parser =
            TreeSitterParser::new().map_err(|e| BuildChatError::ParserInit(e.to_string()))?;
        let raw_langs = if desc.langs.is_empty() {
            vec!["eng".to_string()]
        } else {
            desc.langs.clone()
        };
        // Parse language codes ONCE at this boundary (chatter 0.3.0 made
        // LanguageCode construction fallible); everything downstream
        // operates on typed codes.
        let langs = raw_langs
            .iter()
            .map(|l| {
                LanguageCode::new(l).map_err(|source| BuildChatError::LanguageCode {
                    code: l.clone(),
                    source,
                })
            })
            .collect::<Result<Vec<_>, BuildChatError>>()?;
        let primary_lang = langs
            .first()
            .cloned()
            .ok_or(BuildChatError::NoParticipants)?;

        Ok(Self {
            parser,
            langs,
            primary_lang,
        })
    }

    pub(super) fn parser(&self) -> &TreeSitterParser {
        &self.parser
    }

    pub(super) fn langs(&self) -> &[LanguageCode] {
        &self.langs
    }

    pub(super) fn primary_lang(&self) -> &LanguageCode {
        &self.primary_lang
    }
}
