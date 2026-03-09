.PHONY: test clippy golden parity-check book clean

test:
	cargo nextest run -p talkbank-clan
	cargo test --doc -p talkbank-clan

clippy:
	cargo clippy -p talkbank-clan --all-targets -- -D warnings

golden:
	cargo nextest run -p talkbank-clan -- golden

parity-check:
	./scripts/parity-check.sh

parity-check-summary:
	./scripts/parity-check.sh --summary-only

book:
	mdbook build book/

clean:
	cargo clean -p talkbank-clan
	rm -rf parity-results/
