//! Test module for test terminator without whitespace in `talkbank-chat`.
//!
//! These tests document expected behavior and regressions.

// Test to isolate the terminator parsing issue when there's no whitespace

use chumsky::prelude::*;

/// Tests or not then ignore then.
#[test]
fn test_or_not_then_ignore_then() {
    // Simulate: ws_parser().or_not().ignore_then(terminator_parser().or_not())

    let ws = just::<_, _, extra::Err<Simple<char>>>(' ')
        .repeated()
        .at_least(1)
        .ignored();
    let terminator = just::<_, _, extra::Err<Simple<char>>>('?').ignored();

    // Test 1: With whitespace before terminator (should work)
    let parser1 = ws.or_not().ignore_then(terminator.or_not());
    let result1 = parser1.parse(" ?").into_result();
    assert!(
        result1.is_ok(),
        "Should parse ' ?' successfully: {:?}",
        result1
    );

    // Test 2: Without whitespace before terminator (THIS IS THE FAILING CASE)
    let parser2 = ws.or_not().ignore_then(terminator.or_not());
    let result2 = parser2.parse("?").into_result();
    println!("Parsing '?' without whitespace: {:?}", result2);
    assert!(
        result2.is_ok(),
        "Should parse '?' successfully: {:?}",
        result2
    );

    // Test 3: Empty input (should work since both are optional)
    let parser3 = ws.or_not().ignore_then(terminator.or_not());
    let result3 = parser3.parse("").into_result();
    assert!(
        result3.is_ok(),
        "Should parse '' successfully: {:?}",
        result3
    );
}

/// Tests direct terminator optional.
#[test]
fn test_direct_terminator_optional() {
    // Just test if optional terminator works by itself
    let terminator = just::<_, _, extra::Err<Simple<char>>>('?').ignored();

    let parser = terminator.or_not();
    let result1 = parser.parse("?").into_result();
    assert!(
        result1.is_ok(),
        "Should parse '?' successfully: {:?}",
        result1
    );

    let result2 = parser.parse("").into_result();
    assert!(
        result2.is_ok(),
        "Should parse '' successfully: {:?}",
        result2
    );
}

/// Tests then vs ignore then.
#[test]
fn test_then_vs_ignore_then() {
    let ws = just::<_, _, extra::Err<Simple<char>>>(' ')
        .repeated()
        .at_least(1)
        .ignored();
    let terminator = just::<_, _, extra::Err<Simple<char>>>('?').to('T');

    // Using .then()
    let parser1 = ws.or_not().then(terminator.or_not());
    let result1 = parser1.parse("?").into_result();
    println!(".then() result for '?': {:?}", result1);

    // Using .ignore_then()
    let parser2 = ws.or_not().ignore_then(terminator.or_not());
    let result2 = parser2.parse("?").into_result();
    println!(".ignore_then() result for '?': {:?}", result2);
}
