.PHONY: help symbols-gen generated-check test-gen mine-candidates test build clean check verify parser-guard chat-anchors-check book book-serve coverage smoke check-specs ci-local install-hooks

help:
	@echo "TalkBank Core Library Tasks"
	@echo ""
	@echo "  make test-gen       Generate tests from specs"
	@echo "  make symbols-gen    Generate shared symbol sets"
	@echo "  make generated-check Regenerate and verify generated artifacts are in sync"
	@echo "  make mine-candidates Mine valid CHAT candidates from ../data into spec/tmp/"
	@echo "  make test           Run all tests"
	@echo "  make build          Build all components"
	@echo "  make check          Fast compile check"
	@echo "  make verify         Canonical pre-merge verification gates"
	@echo "  make coverage       Report grammar node type coverage for reference corpus"
	@echo "  make parser-guard   Enforce parser ErrorSink/Option signature guardrail"
	@echo "  make chat-anchors-check Verify CHAT manual anchors referenced in source/docs"
	@echo "  make smoke CRATE=x  Fast check + test a single crate"
	@echo "  make check-specs    Verify every error code has a spec file"
	@echo "  make clean          Clean all build artifacts"
	@echo "  make book           Build the documentation book"
	@echo "  make book-serve     Serve the documentation book locally"
	@echo ""

# Guardrail: disallow introducing new ErrorSink + Option parser signatures.
parser-guard:
	@scripts/check-errorsink-option-signatures.sh

# Validate that CHAT manual anchors referenced in source/docs resolve in CHAT.html.
# Optional local mirror path: CHAT_HTML_PATH=/abs/path/to/CHAT.html make chat-anchors-check
chat-anchors-check:
	@scripts/check-chat-manual-anchors.sh

# Generate shared symbol sets used by grammar.
symbols-gen:
	@echo "==> Generating shared symbol sets..."
	node spec/symbols/validate_symbol_registry.js
	node scripts/generate-symbol-sets.js
	node spec/symbols/generate_rust_symbol_sets.js
	rustfmt crates/talkbank-model/src/generated/symbol_sets.rs spec/tools/src/generated/symbol_sets.rs

# Generate tests from specs
test-gen:
	@$(MAKE) symbols-gen
	@echo "==> Generating tree-sitter tests from spec..."
	cd spec/tools && cargo run --bin gen_tree_sitter_tests -- \
		--spec-dir ../constructs \
		--error-dir ../errors \
		--template-dir templates \
		--output-dir ../../grammar/test/corpus
	@echo "==> Generating Rust tests from spec..."
	cd spec/tools && cargo run --bin gen_rust_tests -- \
		--construct-dir ../constructs \
		--error-dir ../errors \
		--output-dir ../../crates/talkbank-parser-tests/tests/generated \
		--test-error-path talkbank_parser_tests::test_error::TestError
	@echo "==> Generating error documentation..."
	cd spec/tools && cargo run --bin gen_error_docs -- \
		--error-dir ../errors \
		--output-dir ../../docs/errors

# Mine candidate files from real data (staging-only, for curation input)
# Usage: make mine-candidates LANGUAGES=eng,spa TOP=50 MAX_LINES=200 MAX_FILES=20000 DATA_DIR=../data
mine-candidates:
	@mkdir -p spec/tmp/mined
	cd spec/tools && cargo run --bin extract_corpus_candidates -- \
		--data-dir $${DATA_DIR:-../data} \
		--languages $${LANGUAGES:-eng} \
		--node-types ../../grammar/src/node-types.json \
		--max-lines $${MAX_LINES:-200} \
		--max-files $${MAX_FILES:-20000} \
		--top $${TOP:-50} \
		--require-rust-parse=true \
		--require-rust-validation=true \
		--validate-alignment=true \
		--json \
		--output ../tmp/mined/candidates.$${LANGUAGES:-eng}.json

# Regenerate and verify all generated artifacts are committed.
generated-check:
	@$(MAKE) symbols-gen
	@$(MAKE) test-gen
	@echo "==> Checking for uncommitted generated changes..."
	git diff --exit-code -- \
		spec/symbols/symbol_registry.json \
		spec/symbols/validate_symbol_registry.js \
		spec/symbols/generate_rust_symbol_sets.js \
		crates/talkbank-model/src/generated/symbol_sets.rs \
		spec/tools/src/generated/symbol_sets.rs \
		crates/talkbank-parser-tests/tests/generated \
		docs/errors

# Run all tests
test:
	@echo "==> Testing Rust workspace..."
	cargo nextest run --workspace
	@echo "==> Testing doctests..."
	cargo test --doc
	@echo "==> Testing spec tools..."
	cd spec/tools && cargo test

# Build all components
build:
	@$(MAKE) symbols-gen
	@echo "==> Building Rust workspace..."
	cargo build --workspace --release
	@echo "==> Building spec tools..."
	cd spec/tools && cargo build --release

# Fast compile check
check:
	@echo "==> Checking Rust workspace..."
	cargo check --workspace --all-targets
	@echo "==> Checking spec tools..."
	cd spec/tools && cargo check --all-targets

# Canonical pre-merge verification gates
verify:
	@echo "==> [G0] Parser signature guardrail"
	@$(MAKE) parser-guard
	@echo "==> [G1] Rust workspace compile check"
	cargo check --workspace --all-targets
	@echo "==> [G2] Spec tools compile check"
	cd spec/tools && cargo check --all-targets
	@echo "==> [G3] CHAT manual anchor links"
	@$(MAKE) chat-anchors-check
	@echo "==> [G4] Generated parser corpus equivalence suite"
	cargo nextest run -p talkbank-parser-tests --test generated
	@echo "==> [G5] Word-level parser equivalence suite"
	cargo nextest run -p talkbank-parser-tests --test parser_equivalence_words
	@echo "==> [G6] Bare-timestamp regression gate"
	cargo nextest run --test bare_timestamp_regression
	@echo "==> [G7] Reference corpus semantic equivalence (tree-sitter vs direct)"
	cargo nextest run -p talkbank-parser-tests --test parser_equivalence_files
	@echo "==> [G8] %wor tier parsing and alignment"
	cargo nextest run -p talkbank-parser-tests --test wor_terminator_alignment
	@echo "==> [G9] Golden tier roundtrip (%mor, %gra, %pho, %wor)"
	cargo nextest run -p talkbank-parser-tests --test parser_suite
	@echo "==> [G10] Reference corpus node coverage"
	@$(MAKE) coverage

# Reference corpus grammar node type coverage
coverage:
	cd spec/tools && cargo run --bin corpus_node_coverage -- \
		--corpus-dir ../../corpus/reference \
		--node-types ../../grammar/src/node-types.json

# Fast iteration: compile-check workspace + test a single crate
# Usage: make smoke CRATE=talkbank-model
smoke:
	@echo "==> Compile check (workspace)..."
	cargo check --workspace --all-targets
	@echo "==> Testing $(CRATE)..."
	cargo nextest run -p $(CRATE) --no-fail-fast

# Verify every error code in the Rust enum has a corresponding spec file
check-specs:
	@scripts/check-error-specs.sh

# Fast local CI: checks that mirror the CI pipeline (no tests).
ci-local:
	@echo "==> fmt check (main workspace)"
	cargo fmt --all -- --check
	@echo "==> fmt check (spec/tools)"
	cd spec/tools && cargo fmt --all -- --check
	@echo "==> clippy"
	cargo clippy --all-targets -- -D warnings
	@echo "==> compile check (main workspace)"
	cargo check --workspace --all-targets
	@echo "==> compile check (spec/tools)"
	cd spec/tools && cargo check --all-targets
	@echo "==> parser guardrail"
	@scripts/check-errorsink-option-signatures.sh
	@echo "==> generated artifacts check"
	@$(MAKE) generated-check
	@echo "✓ ci-local passed"

# Install git hooks (pre-push).
install-hooks:
	ln -sf ../../scripts/pre-push.sh .git/hooks/pre-push
	@echo "✓ pre-push hook installed"

# Clean build artifacts
clean:
	cargo clean
	cd spec/tools && cargo clean

# Build the documentation book
book:
	mdbook build book/

# Serve the documentation book locally
book-serve:
	mdbook serve book/
