.PHONY: build dev test fmt lint clean release install

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

release:
	@echo "Building release binary..."
	PROTOC=$(PROTOC) cargo build --release
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
		echo "Creating tag $$NEW_TAG..."; \
		git tag $$NEW_TAG; \
		echo "Pushing tag $$NEW_TAG to origin..."; \
		git push origin $$NEW_TAG; \
	else \
		echo "Tagging cancelled. Release binary is ready in target/release/tld"; \
	fi

install:
	PROTOC=$(PROTOC) cargo install --path .

# Allow passing arguments to cargo run
%:
	@: