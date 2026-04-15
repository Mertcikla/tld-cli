.PHONY: build dev test fmt lint clean release install changelog

# Default protoc path for macOS Homebrew, can be overridden
PROTOC ?= protoc

build:
	PROTOC=$(PROTOC) cargo build

dev:
	PROTOC=$(PROTOC) cargo run -- $(filter-out $@,$(MAKECMDGOALS))

test:
	PROTOC=$(PROTOC) cargo test

fmt:
	cargo fmt

lint:
	PROTOC=$(PROTOC) cargo clippy -- -D warnings

clean:
	cargo clean

changelog:
	@if ! command -v git-cliff &> /dev/null; then \
		echo "Installing git-cliff..."; \
		cargo install git-cliff; \
	fi
	git-cliff --output CHANGELOG.md

release:
	@if ! command -v git-cliff &> /dev/null; then \
		echo "Installing git-cliff..."; \
		cargo install git-cliff; \
	fi
	@echo "Fetching latest tags..."
	@git fetch --tags --quiet
	@LATEST_TAG=$$(git tag --sort=-v:refname | head -n 1); \
	if [ -z "$$LATEST_TAG" ]; then LATEST_TAG="v0.0.0"; fi; \
	VERSION=$${LATEST_TAG#v}; \
	MAJOR=$$(echo $$VERSION | cut -d. -f1); \
	MINOR=$$(echo $$VERSION | cut -d. -f2); \
	PATCH=$$(echo $$VERSION | cut -d. -f3); \
	NEW_PATCH=$$((PATCH + 1)); \
	NEW_TAG="v$$MAJOR.$$MINOR.$$NEW_PATCH"; \
	echo "Current tag: $$LATEST_TAG"; \
	echo "New tag:     $$NEW_TAG"; \
	printf "Confirm release tag and push? [y/N] "; \
	read confirm; \
	if [ "$$confirm" = "y" ] || [ "$$confirm" = "Y" ]; then \
		echo "Generating changelog..."; \
		git-cliff --tag $$NEW_TAG --output CHANGELOG.md; \
		git add CHANGELOG.md; \
		git commit -m "chore(release): update changelog for $$NEW_TAG"; \
		echo "Creating tag $$NEW_TAG..."; \
		git tag $$NEW_TAG; \
		echo "Pushing to origin..."; \
		git push origin HEAD $$NEW_TAG; \
	else \
		echo "Tagging cancelled."; \
	fi

install:
	PROTOC=$(PROTOC) cargo install --path .

# Allow passing arguments to cargo run
%:
	@: