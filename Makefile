.PHONY: build run test test-unit test-cmd test-cover test-cover-html lint fmt

proto:
	go get buf.build/gen/go/tldiagramcom/diagram/protocolbuffers/go@$(shell buf registry sdk version --module=buf.build/tldiagramcom/diagram --plugin=buf.build/protocolbuffers/go)
	go get buf.build/gen/go/tldiagramcom/diagram/connectrpc/go@$(shell buf registry sdk version --module=buf.build/tldiagramcom/diagram --plugin=buf.build/connectrpc/go)

build:
	go build -o ./build/tld .

run:
	go run .

test:
	go test ./... -v -race -shuffle=on -count=1 -timeout 60s

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
