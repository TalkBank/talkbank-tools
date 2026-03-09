# Examples

## User Examples

| Example | Description | Usage |
|---------|-------------|-------|
| `simple_roundtrip.rs` | Parse CHAT from stdin, serialize back to stdout | `echo '@UTF8\n...' \| cargo run --example simple_roundtrip` |
| `compare_parsers.rs` | Parse a file with both tree-sitter and direct parsers, compare output | `cargo run --example compare_parsers -- path/to/file.cha` |

## Developer / Debug Tools

These examples are useful for debugging parser and model issues during development:

| Example | Description |
|---------|-------------|
| `debug_alignment.rs` | Debug tier alignment for a specific file |
| `debug_ca_term.rs` | Debug conversation analysis terminator parsing |
| `debug_happy_path.rs` | Minimal parse-and-print for quick checks |
| `debug_headers.rs` | Debug header parsing |
| `debug_header_media.rs` | Debug @Media header parsing |
| `debug_pho_tree.rs` | Debug %pho tier tree structure |
| `debug_pid.rs` | Debug PID parsing |
| `debug_retrace.rs` | Debug retrace/repetition parsing |
| `debug_tree.rs` | Print full parse tree for a file |
| `debug_engn4175.rs` | Debug specific corpus file issue |
| `debug_exclude.rs` | Debug exclusion pattern parsing |
| `test_action_annot.rs` | Test action annotation parsing |
| `test_group.rs` | Test group/bracket parsing |
| `test_pause_parsing.rs` | Test pause notation parsing |
| `test_rust_parser.rs` | Test direct parser on specific files |
| `test_stability.rs` | Test parse stability across multiple passes |

## Profiling Tools

| Example | Description |
|---------|-------------|
| `analyze_boxing_options.rs` | Analyze memory layout of boxed vs inline model types |
| `check_enum_sizes.rs` | Report `size_of` for key enum types |
