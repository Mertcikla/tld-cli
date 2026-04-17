.PHONY: build dev test fmt lint clean release install changelog

build:
	cargo build

dev:
	cargo run -- $(filter-out $@,$(MAKECMDGOALS))

test:
	cargo test
	
repo-test:
	uv run scripts/test_bench.py

fmt:
	cargo fmt

lint:
	cargo clippy -- -D warnings

clean:
	cargo clean

changelog:
	git-cliff --output CHANGELOG.md

release:
	@echo "Fetching latest tags..."
	@git fetch --tags --quiet
	@CURRENT_VERSION=$$(sed -nE 's/^version = "([^"]+)"/\1/p' Cargo.toml | head -n 1); \
	NEW_VERSION=$$(git-cliff --bumped-version); \
	NEW_TAG="v$$NEW_VERSION"; \
	if git rev-parse -q --verify "refs/tags/$$NEW_TAG" >/dev/null; then \
		echo "Tag $$NEW_TAG already exists."; \
		exit 1; \
	fi; \
	echo "Current crate version: $$CURRENT_VERSION"; \
	echo "New crate version:     $$NEW_VERSION"; \
	echo "New tag:               $$NEW_TAG"; \
	printf "Confirm release tag and push? [y/N] "; \
	read confirm; \
	if [ "$$confirm" = "y" ] || [ "$$confirm" = "Y" ]; then \
		echo "Updating crate version..."; \
		./scripts/set-version.sh $$NEW_VERSION; \
		echo "Generating changelog..."; \
		git-cliff --tag $$NEW_TAG --output CHANGELOG.md; \
		git add Cargo.toml Cargo.lock CHANGELOG.md; \
		git commit -m "chore(release): $$NEW_TAG"; \
		echo "Creating tag $$NEW_TAG..."; \
		git tag $$NEW_TAG; \
		echo "Pushing to origin..."; \
		git push origin HEAD $$NEW_TAG; \
	else \
		echo "Tagging cancelled."; \
	fi

install:
	cargo install --path . --force

# Allow passing arguments to cargo run
%:
	@:
