# API Stability

**Status:** Current
**Last updated:** 2026-04-30 23:00 EDT

## The CLI is the only public surface

`batchalign3` is a CLI-first tool. The compatibility surface that
public consumers may depend on is:

- the `batchalign3` command-line interface (subcommands, options,
  output formats, exit codes)
- the typed CHAT files written to disk by `batchalign3` commands
- the OpenAPI schema exported by `batchalign3 openapi` for the HTTP
  server, if you run a server

That is the entire compatibility surface. The Python runtime, the
PyO3 extension module, the worker IPC payloads, and every internal
module under `batchalign.*` are implementation details. They are not
covered by any compatibility guarantee and may change without notice.

## Python is internal

There is no public Python API. The Python code in this package
exists to host worker-side ML inference (Stanza, Whisper backends,
Cantonese ASR engines, etc.) on behalf of the Rust runtime. It is not
a library you can import against.

This includes:

- everything under `batchalign.worker.*`
- everything under `batchalign.inference.*`
- the `batchalign.providers` re-export module
- the `batchalign_core` PyO3 extension module
- every previously-documented Python facade (`pipeline_api`,
  `compat`, `BatchalignPipeline`, `WhisperEngine`, `CHATFile`,
  `Document`, `ParsedChat`, `run_pipeline()`, etc.)

If you have BA2-era Python integration code, port it to subprocess
calls into `batchalign3`. The CLI's flags and outputs are the
long-term stable contract.

The intended long-term direction is to remove Python entirely as
Rust gains coverage of the remaining ML pieces. The current Python
surface is therefore a deliberately shrinking layer, not a shape to
preserve.

## Beta caveat

The project is in beta. The CLI surface, output formats, and
configuration files are not yet frozen, breaking changes may land
during the pre-1.0 stabilization period. Breaking changes will be
documented but may land without a deprecation period while the
project remains in beta. The public API surface will be formally
frozen at the 1.0 release.

## See also

- [User Guide: No Python API](../user-guide/python-api.md)
- [Python-Rust Boundary, Three-Layer Split](../../architecture/python-rust-boundary/python-rust-boundary.md#three-layer-split--internal-only)
