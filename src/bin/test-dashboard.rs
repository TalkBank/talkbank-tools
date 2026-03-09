//! TUI dashboard binary for corpus testing with live progress visualization.

/// Run the split dashboard implementation.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    talkbank_tools::test_dashboard::run()
}
