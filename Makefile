.PHONY: help hooks-check test test-affected batchalign-check batchalign-test-rust batchalign-test-integration batchalign-build-pyo3 batchalign-build-wheel batchalign-python-prepare batchalign-test-python batchalign-typecheck-python batchalign-ci-python batchalign-runtime-check batchalign-dashboard-api-check batchalign-dashboard-build batchalign-dashboard-e2e batchalign-dashboard-e2e-real batchalign-ci-rust build clean check check-affected lint-affected verify book-check book book-serve smoke ci-local ci-full install-hooks _batchalign-test-python _batchalign-typecheck-python audit-status audit-streak audit-scan audit-flag-staleness audit-prose-references

help:
	@echo "talkbank-tools task index (batchalign3 workspace)"
	@echo ""
	@echo "Core workflow:"
	@echo "  make check                 Fast compile check for the workspace"
	@echo "  make test                  Rust workspace tests + doctests"
	@echo "  make verify                Canonical pre-merge gate (compile + batchalign + book)"
	@echo "  make ci-local              Fast local CI approximation"
	@echo "  make ci-full               Stricter local CI approximation"
	@echo "  make smoke CRATE=x         Fast compile check + one crate test"
	@echo ""
	@echo "Batchalign workflow:"
	@echo "  make batchalign-check              Imported Batchalign compile checks"
	@echo "  make batchalign-test-rust          Imported Batchalign Rust library suites"
	@echo "  make batchalign-test-integration   Imported Batchalign focused integration gates"
	@echo "  make batchalign-build-pyo3         Imported standalone PyO3 crate"
	@echo "  make batchalign-build-wheel        Imported Batchalign wheel"
	@echo "  make batchalign-test-python        Imported Batchalign Python pytest gate"
	@echo "  make batchalign-typecheck-python   Imported Batchalign Python typecheck gate"
	@echo "  make batchalign-ci-python          Imported Batchalign Python wheel/test/type gate"
	@echo "  make batchalign-dashboard-build       Imported dashboard frontend build"
	@echo "  make batchalign-dashboard-api-check  Imported dashboard API drift gate"
	@echo "  make batchalign-dashboard-e2e        Dashboard e2e tests (mock server)"
	@echo "  make batchalign-dashboard-e2e-real   Dashboard e2e tests (real server)"
	@echo "  make batchalign-runtime-check        Imported runtime constants check"
	@echo "  make batchalign-ci-rust              Imported Batchalign Rust/PyO3 CI gate"
	@echo ""
	@echo "Docs and developer helpers:"
	@echo "  make install-hooks          Install the git pre-push hook"
	@echo "  make book                   Build the unified TalkBank Toolchain book"
	@echo "  make book-serve             Serve the unified book locally"
	@echo "  cargo run -q -p xtask -- help"
	@echo "                              List xtask audit/helper commands"
	@echo "  make clean                  Clean build artifacts"
	@echo ""

# Warn if the pre-push hook isn't installed. Not a hard failure —
# users may intentionally push without hooks in rare cases (e.g.,
# re-pushing an already-verified commit after a remote hiccup).
hooks-check:
	@if [ ! -e .git/hooks/pre-push ]; then \
	  echo "warning: .git/hooks/pre-push is not installed — run 'make install-hooks'" >&2; \
	fi

# Run all tests
test:
	@echo "==> Testing Rust workspace..."
	cargo nextest run --workspace
	@echo "==> Testing doctests..."
	cargo test --doc

test-affected:
	cargo run -q -p xtask -- affected-rust test

BATCHALIGN_PYTEST_ARGS ?= batchalign --disable-pytest-warnings -k "not test_whisper_fa_pipeline"

batchalign-check:
	@echo "==> Checking imported Batchalign Rust crates..."
	cargo check -p batchalign-types -p batchalign -p batchalign-transform -p batchalign-pyo3 --all-targets

batchalign-test-rust:
	@echo "==> Testing imported batchalign-types..."
	cargo test -p batchalign-types --lib -q
	@echo "==> Testing batchalign-transform (ML over chatter's generic transform)..."
	cargo test -p batchalign-transform --lib -q
	@echo "==> Testing imported batchalign..."
	cargo test -p batchalign --lib -q

batchalign-test-integration:
	@echo "==> Running imported Batchalign CI hygiene..."
	cargo run -q -p xtask -- lint-ci-hygiene
	@echo "==> Testing imported batchalign focused integration gates..."
	cargo test -p batchalign --test json_compat --test workflow_helpers -q

batchalign-build-pyo3:
	@echo "==> Building imported standalone PyO3 crate..."
	cargo build --manifest-path crates/batchalign-pyo3/Cargo.toml -q

batchalign-build-wheel:
	@echo "==> Building imported Batchalign wheel..."
	@# Default: always rebuild the native binary so the wheel never
	@# bundles a stale one. The 2026-04-29 deploy postmortem (cancel-
	@# cascade) was caused by a previous guard that silently reused
	@# whatever was at batchalign/_bin/batchalign3 — even when the
	@# sources had changed.
	@#
	@# Two known-safe skip paths:
	@#   1. Windows cross-compile pre-stages batchalign3.exe (existing).
	@#   2. CI's build-wheel job downloads the cli-binary artifact (the
	@#      OUTPUT of build-cli, compiled FROM THIS COMMIT'S SOURCES) and
	@#      sets BATCHALIGN_PRESTAGED_BIN=1 to declare provenance. In
	@#      that one path the prestaged binary is guaranteed fresh and
	@#      rebuilding duplicates ~9 min of fat-LTO compile.
	@#
	@# Local invocations (no env var, no .exe) fall through to the safe
	@# always-rebuild path.
	@if [ -f batchalign/_bin/batchalign3.exe ]; then \
	  echo "==> Using pre-staged Windows binary (.exe)"; \
	elif [ "$$BATCHALIGN_PRESTAGED_BIN" = "1" ] && [ -x batchalign/_bin/batchalign3 ]; then \
	  echo "==> Using pre-staged Linux binary from CI build-cli artifact"; \
	else \
	  echo "==> Building native batchalign3 binary..."; \
	  cargo build --release -p batchalign; \
	  mkdir -p batchalign/_bin; \
	  cp target/release/batchalign3 batchalign/_bin/batchalign3; \
	fi
	rm -rf dist
	mkdir -p dist
	uv build --wheel --out-dir dist/

batchalign-python-prepare: batchalign-build-wheel
	@echo "==> Syncing imported Batchalign dev dependencies..."
	uv sync --group dev --no-install-project
	@echo "==> Installing imported Batchalign wheel into the dev environment..."
	uv pip install --reinstall --no-deps dist/*.whl

_batchalign-test-python:
	@echo "==> Running imported Batchalign Python tests..."
	uv run --no-sync pytest $(BATCHALIGN_PYTEST_ARGS)

batchalign-test-python: batchalign-python-prepare
	@$(MAKE) _batchalign-test-python

_batchalign-typecheck-python:
	@echo "==> Verifying imported Batchalign retirement gates..."
	test ! -e batchalign/cli/cli.py
	test ! -e batchalign/serve/app.py
	test ! -e batchalign/serve/job_store.py
	@echo "==> Running imported Batchalign CI hygiene..."
	cargo run -q -p xtask -- lint-ci-hygiene
	@$(MAKE) batchalign-runtime-check
	@echo "==> Running imported Batchalign Python typecheck..."
	uv run --no-sync mypy

batchalign-typecheck-python: batchalign-python-prepare
	@$(MAKE) _batchalign-typecheck-python

batchalign-ci-python: batchalign-python-prepare
	@$(MAKE) _batchalign-test-python
	@$(MAKE) _batchalign-typecheck-python

batchalign-runtime-check:
	@echo "==> Verifying imported runtime constants..."
	python3 scripts/check_runtime_drift.py

batchalign-dashboard-api-check:
	@echo "==> Verifying imported dashboard API artifacts..."
	bash scripts/check_dashboard_api_drift.sh

batchalign-dashboard-build:
	@echo "==> Building imported dashboard frontend..."
	cd frontend && npm ci && npm run build

batchalign-dashboard-e2e:
	@echo "==> Running dashboard e2e tests (mock server)..."
	bash scripts/run_react_dashboard_smoke.sh

batchalign-dashboard-e2e-real:
	@echo "==> Running dashboard e2e tests (real server)..."
	cd frontend && npm ci
	cd frontend/e2e && npm ci && npm run install:browsers
	BATCHALIGN_REAL_SERVER_E2E=1 bash scripts/run_react_dashboard_smoke.sh

batchalign-ci-rust:
	@$(MAKE) batchalign-check
	@$(MAKE) batchalign-test-rust
	@$(MAKE) batchalign-test-integration
	@$(MAKE) batchalign-build-pyo3

# Build all components
build:
	@echo "==> Building Rust workspace..."
	cargo build --workspace --release

# Fast compile check
check:
	@echo "==> Checking Rust workspace..."
	cargo check --workspace --all-targets

check-affected:
	cargo run -q -p xtask -- affected-rust check

lint-affected:
	cargo run -q -p xtask -- affected-rust clippy

# Canonical pre-merge verification gates
# The CHAT-format gates (parser signature guardrail, spec tools, CHAT manual
# anchors, parser-equivalence / golden / corpus / generated-check / fuzz) moved
# to chatter, which is now the single home for the CHAT core (model, parser,
# transform, spec, grammar). talkbank-tools verifies the batchalign layer it
# still owns; CHAT-format verification lives in the chatter repo.
verify:
	@$(MAKE) hooks-check
	@echo "==> [G1] Rust workspace compile check"
	cargo check --workspace --all-targets
	@echo "==> [G2] Batchalign Rust check (types, transform, batchalign, pyo3)"
	@$(MAKE) batchalign-check
	@echo "==> [G3] Batchalign Rust lib tests"
	@$(MAKE) batchalign-test-rust
	@echo "==> [G4] mdBook build + linkcheck"
	@$(MAKE) book-check

# Build the unified TalkBank mdBook and link-check it with lychee.
#
# mermaid must be a preprocessor (it rewrites ```mermaid blocks), and no
# released mdbook-mermaid parses mdBook 0.5's renamed preprocessor wire
# format, so the diagram-rendering pair is pinned to the 0.4.x era
# (mdBook 0.4.52 + mdbook-mermaid 0.16.2). Link-checking is decoupled
# onto lychee, which runs on the built HTML and is independent of
# mdBook's wire format. The previous mdbook-linkcheck2 renderer only
# accepts mdBook 0.5's `items` RenderContext, which 0.4.x does not emit,
# so it could not run alongside mermaid on any single mdBook version.
# lychee still catches SUMMARY-unreachable targets like the 2026-05-22
# batchalign/introduction.md regression. `--offline` skips web links;
# `--root-dir` resolves the 404 page's leading '/'.
book-check:
	@command -v mdbook >/dev/null || { \
		echo "ERROR: mdbook not found on PATH."; \
		echo "Install: cargo install mdbook@0.4.52 mdbook-mermaid@0.16.2 lychee"; \
		exit 1; \
	}
	@command -v lychee >/dev/null || { \
		echo "ERROR: lychee not found on PATH."; \
		echo "Install: cargo install lychee"; \
		exit 1; \
	}
	mdbook build book
	lychee --offline --root-dir "$(CURDIR)/book/build" "$(CURDIR)/book/build"

# Fast iteration: compile-check workspace + test a single crate
# Usage: make smoke CRATE=talkbank-model
smoke:
	@echo "==> Compile check (workspace)..."
	cargo check --workspace --all-targets
	@echo "==> Testing $(CRATE)..."
	cargo nextest run -p $(CRATE) --no-fail-fast

# Fast local CI: fmt + dependency-aware compile checks + structural lints.
ci-local:
	@echo "==> fmt check (main workspace)"
	cargo fmt --all -- --check
	@echo "==> affected compile check"
	cargo run -q -p xtask -- affected-rust check
	@echo "==> wide struct audit"
	cargo run -q -p xtask -- lint-wide-structs
	@echo "==> docs sync"
	cargo run -q -p xtask -- lint-docs-sync
	@echo "✓ ci-local passed"

# Full local CI: mirrors the stricter CI-style gate.
ci-full:
	@echo "==> fmt check (main workspace)"
	cargo fmt --all -- --check
	@echo "==> clippy"
	cargo clippy --all-targets -- -D warnings
	@echo "==> compile check (main workspace)"
	cargo check --workspace --all-targets
	@echo "==> runtime_constants.toml drift check"
	@cargo run -p xtask --quiet -- gen-runtime-toml --check
	@echo "==> imported Batchalign Rust/PyO3 gate"
	@$(MAKE) batchalign-ci-rust
	@echo "✓ ci-full passed"

# Install git hooks (pre-push).
install-hooks:
	ln -sf ../../scripts/pre-push.sh .git/hooks/pre-push
	@echo "✓ pre-push hook installed"

# Clean build artifacts
clean:
	cargo clean

# Build the documentation book
book:
	mdbook build book/

# Serve the documentation book locally
book-serve:
	mdbook serve book/

# ---------------------------------------------------------------------------
# Doc audit (talkbank-tools only)
# ---------------------------------------------------------------------------
#
# The catalog DB is auditing tooling, not user content; it lives in
# the meta-repo's release-doc-audit/ working dir. Default path assumes
# the workspace layout `<workspace>/talkbank-tools` alongside
# `<workspace>/docs/release-doc-audit/audit.db`. Operators with a
# different layout override TB_AUDIT_DB.
#
# Daily-cadence: `make audit-status` is the session-start command —
# prints Bucket A progress, streak, and the next 5 unvetted sections.
# See `<workspace>/docs/release-doc-audit/audit-method.md`.
TB_AUDIT_DB ?= ../docs/release-doc-audit/audit.db
TB_AUDIT_TT_ROOT ?= $(CURDIR)

audit-status:
	@TB_AUDIT_DB="$(TB_AUDIT_DB)" cargo run -q -p xtask -- audit-docs status

audit-streak:
	@TB_AUDIT_DB="$(TB_AUDIT_DB)" cargo run -q -p xtask -- audit-docs streak

audit-scan:
	TB_AUDIT_DB="$(TB_AUDIT_DB)" TB_AUDIT_TT_ROOT="$(TB_AUDIT_TT_ROOT)" \
		cargo run -q -p xtask -- audit-docs scan

audit-flag-staleness:
	TB_AUDIT_DB="$(TB_AUDIT_DB)" TB_AUDIT_TT_ROOT="$(TB_AUDIT_TT_ROOT)" \
		cargo run -q -p xtask -- audit-docs flag-staleness

# Layer 1 CI gate. Catalog-independent — walks every .md file under the
# repo root and exits non-zero if any high-severity prose-reference
# pattern (deleted crate, moved book path) is found outside the
# allow-list. Designed for ci.yml use where audit.db is not present.
audit-prose-references:
	cargo run -q -p xtask -- audit-prose-references
