# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `chatter clan` subcommand integrating all 16 CLAN analysis commands (freq, mlu, mlt, wdlen, maxwd, freqpos, timedur, kwal, gemlist, combo, cooccur, dist, chip, phonfreq, modrep, vocd) and 8 transform commands (flo, lowcase, chstring, dates, delim, fixbullets, retrace, repeat) as direct subcommands of `chatter`.

### Fixed

- `chatter to-json` no longer fails with "missing schema" when run from outside the source tree. The JSON schema is now embedded at compile time via `talkbank_transform::SCHEMA_JSON`.
- `chatter schema` now uses the same embedded schema constant from `talkbank-transform` instead of a separate `include_str!`.

## [0.1.0] - 2026-02-21

### Added

- Initial release.
