//! Build script for compiling and linking the `send2clan` C library.
//!
//! # Related CHAT Manual Sections
//!
//! - <https://talkbank.org/0info/manuals/CHAT.html#File_Format>

/// Entry point for this binary target.
fn main() {
    let mut build = cc::Build::new();

    // Compile the C library
    build
        .file("csrc/send2clan.c")
        .include("csrc")
        .warnings(true)
        .extra_warnings(true);

    #[cfg(target_os = "windows")]
    {
        build.define("_WIN32", None);
    }

    build.compile("send2clan");

    // Platform-specific linking
    #[cfg(target_os = "macos")]
    {
        println!("cargo:rustc-link-lib=framework=Carbon");
        println!("cargo:rustc-link-lib=framework=ApplicationServices");
        println!("cargo:rustc-link-lib=framework=CoreServices");
    }

    #[cfg(target_os = "windows")]
    {
        println!("cargo:rustc-link-lib=shell32");
        println!("cargo:rustc-link-lib=user32");
        println!("cargo:rustc-link-lib=advapi32");
    }

    // Re-run build script if C source changes
    println!("cargo:rerun-if-changed=csrc/send2clan.c");
    println!("cargo:rerun-if-changed=csrc/send2clan.h");
}
