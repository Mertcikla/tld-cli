.PHONY: build run test test-unit test-cmd test-cover test-cover-html lint fmt release

proto:
	go get buf.build/gen/go/tldiagramcom/diagram/protocolbuffers/go@$(shell buf registry sdk version --module=buf.build/tldiagramcom/diagram --plugin=buf.build/protocolbuffers/go)
	go get buf.build/gen/go/tldiagramcom/diagram/connectrpc/go@$(shell buf registry sdk version --module=buf.build/tldiagramcom/diagram --plugin=buf.build/connectrpc/go)
	go mod tidy

build:
	go build -o ./build/tld .

run:
	go run .

test:
	go test ./... -race -shuffle=on -count=1 -timeout 60s

test-cover:
	go test ./... -race -coverprofile=coverage.out -count=1 -timeout 60s
	go tool cover -func=coverage.out

test-cover-html:
	go test ./... -race -coverprofile=coverage.out -count=1 -timeout 60s
	go tool cover -html=coverage.out

test-unit:
	go test ./... -count=1

test-cmd:
	go test ./cmd/... -count=1

lint: ## Run linters
	@echo "Running linters..."
	@golangci-lint run --timeout=5m

fmt: ## Format code
	@echo "Formatting code..."
	@go fmt ./...
	@golangci-lint run --fix

release: ## Create and push a new patch release
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
	printf "Confirm release? [y/N] "; \
	read confirm; \
	if [ "$$confirm" = "y" ] || [ "$$confirm" = "Y" ]; then \
		echo "Creating tag $$NEW_TAG..."; \
		git tag $$NEW_TAG; \
		echo "Pushing tag $$NEW_TAG to origin..."; \
		git push origin $$NEW_TAG; \
	else \
		echo "Release cancelled."; \
	fi
