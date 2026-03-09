//! Integration-style tests for the send2clan Rust API wrapper.
//!
//! # Related CHAT Manual Sections
//!
//! - <https://talkbank.org/0info/manuals/CHAT.html#File_Format>
//! - <https://talkbank.org/0info/manuals/CHAT.html#File_Headers>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Main_Tier>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Dependent_Tiers>

use super::*;

#[test]
fn test_version() {
    let ver = version();
    assert!(!ver.is_empty());
    assert!(ver.contains('.'));
    println!("send2clan version: {}", ver);
}

#[test]
fn test_is_platform_supported() {
    let supported = is_platform_supported();
    println!("Platform supported: {}", supported);

    // On macOS and Windows, this should return true
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    assert!(supported);

    // On other platforms, this should return false
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    assert!(!supported);
}

#[test]
fn test_is_clan_available() {
    let available = is_clan_available();
    println!("CLAN available: {}", available);
    // We don't assert anything since CLAN may or may not be installed
}

#[test]
fn test_get_capabilities() {
    let caps = get_capabilities();
    println!("Capabilities result: {:?}", caps);

    match caps {
        Ok(c) => {
            println!("Capabilities: 0x{:X}", c);
            println!("  Platform supported: {}", (c & 0x01) != 0);
            println!("  CLAN available: {}", (c & 0x02) != 0);
            println!("  Unicode support: {}", (c & 0x04) != 0);
            println!("  Timeout support: {}", (c & 0x08) != 0);
        }
        Err(e) => {
            println!("  Error: {}", e);
        }
    }
}

#[test]
#[ignore = "This test has a 30-second timeout waiting for CLAN - run manually"]
fn test_send_to_clan_validates_params() {
    // This should fail with InvalidParameter or AppNotFound
    // depending on whether CLAN is installed
    let result = send_to_clan(30, "/nonexistent/file.cha", 1, 1, Some("Test"));
    println!("send_to_clan result: {:?}", result);

    // We expect either an error or success (if CLAN is installed and handles it)
    match result {
        Ok(()) => {
            println!("Unexpectedly succeeded - CLAN might be handling the nonexistent file")
        }
        Err(e) => println!("Expected error: {} (code {})", e, e.error_code()),
    }
}

#[test]
fn test_error_codes() {
    assert_eq!(Error::UnsupportedPlatform.error_code(), 1);
    assert_eq!(Error::LaunchFailed.error_code(), 2);
    assert_eq!(Error::AppNotFound.error_code(), 3);
    assert_eq!(Error::SendFailed.error_code(), 4);
    assert_eq!(Error::Timeout.error_code(), 5);
    assert_eq!(Error::InvalidParameter("test".to_string()).error_code(), 6);
    assert_eq!(Error::Unknown.error_code(), 99);
}

#[test]
fn test_error_is_recoverable() {
    assert!(Error::LaunchFailed.is_recoverable());
    assert!(Error::SendFailed.is_recoverable());
    assert!(Error::Timeout.is_recoverable());

    assert!(!Error::UnsupportedPlatform.is_recoverable());
    assert!(!Error::AppNotFound.is_recoverable());
    assert!(!Error::InvalidParameter("test".to_string()).is_recoverable());
    assert!(!Error::Unknown.is_recoverable());
}

#[test]
fn test_null_byte_in_path() {
    let result = send_to_clan(30, "/path/with\0null", 1, 1, None);
    assert!(matches!(result, Err(Error::NulByteInString)));
}

#[test]
fn test_null_byte_in_message() {
    let result = send_to_clan(30, "/path/to/file.cha", 1, 1, Some("message\0with\0nulls"));
    assert!(matches!(result, Err(Error::NulByteInString)));
}

#[test]
fn test_error_display() {
    assert!(
        Error::UnsupportedPlatform
            .to_string()
            .contains("Platform not supported")
    );
    assert!(Error::AppNotFound.to_string().contains("not found"));
    assert!(Error::Timeout.to_string().contains("timed out"));
}
