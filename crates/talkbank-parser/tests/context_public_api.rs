use talkbank_model::ChatOptionFlag;
use talkbank_model::FragmentSemanticContext;
use talkbank_parser::{
    parse_main_tier, parse_main_tier_with_context, parse_utterance, parse_utterance_with_context,
};

#[test]
fn public_main_tier_wrapper_rejects_ca_fragment_without_context() {
    let result = parse_main_tier("*CHI:\t(word) .");
    assert!(result.is_err());
}

#[test]
fn public_main_tier_wrapper_accepts_ca_fragment_with_context() {
    let context = FragmentSemanticContext::new().with_option_flag(ChatOptionFlag::Ca);
    let result = parse_main_tier_with_context("*CHI:\t(word) .", &context);
    assert!(result.is_ok());
}

#[test]
fn public_utterance_wrapper_rejects_ca_fragment_without_context() {
    let result = parse_utterance("*CHI:\t(word) .\n%mor:\tv|(word) .\n");
    assert!(result.is_err());
}

#[test]
fn public_utterance_wrapper_accepts_ca_fragment_with_context() {
    let context = FragmentSemanticContext::new().with_option_flag(ChatOptionFlag::Ca);
    let result = parse_utterance_with_context("*CHI:\t(word) .\n", &context);
    assert!(result.is_ok());
}
