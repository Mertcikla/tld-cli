module github.com/mertcikla/tld-cli

go 1.26.1

require (
	buf.build/gen/go/tldiagramcom/diagram/connectrpc/go/diag/v1/diagv1connect v1.19.1-20260328022351-cdc7b18ed408.2
	buf.build/gen/go/tldiagramcom/diagram/protocolbuffers/go v1.36.11-20260328022351-cdc7b18ed408.1
	connectrpc.com/connect v1.19.1
	github.com/speps/go-hashids/v2 v2.0.1
	github.com/spf13/cobra v1.9.1
	github.com/tree-sitter/go-tree-sitter v0.25.0
	github.com/tree-sitter/tree-sitter-cpp v0.23.4
	github.com/tree-sitter/tree-sitter-go v0.25.0
	github.com/tree-sitter/tree-sitter-java v0.23.5
	github.com/tree-sitter/tree-sitter-python v0.25.0
	go.lsp.dev/jsonrpc2 v0.10.0
	go.lsp.dev/protocol v0.12.0
	go.lsp.dev/uri v0.3.0
	go.uber.org/zap v1.21.0
	google.golang.org/protobuf v1.36.11
	gopkg.in/yaml.v3 v3.0.1
)

require (
	github.com/mattn/go-pointer v0.0.1 // indirect
	github.com/segmentio/asm v1.1.3 // indirect
	github.com/segmentio/encoding v0.3.4 // indirect
	github.com/tree-sitter/tree-sitter-javascript v0.25.0 // indirect
	github.com/tree-sitter/tree-sitter-typescript v0.23.2 // indirect
	go.lsp.dev/pkg v0.0.0-20210717090340-384b27a52fb2 // indirect
	go.uber.org/atomic v1.9.0 // indirect
	go.uber.org/multierr v1.8.0 // indirect
)

require (
	github.com/bmatcuk/doublestar/v4 v4.6.1
	github.com/inconshreveable/mousetrap v1.1.0 // indirect
	github.com/kr/pretty v0.3.1 // indirect
	github.com/mitchellh/colorstring v0.0.0-20190213212951-d06e56a500db // indirect
	github.com/rivo/uniseg v0.4.7 // indirect
	github.com/schollz/progressbar/v3 v3.19.0
	github.com/spf13/pflag v1.0.6 // indirect
	github.com/tetratelabs/wazero v1.11.0
	golang.org/x/sys v0.43.0 // indirect
	golang.org/x/term v0.42.0 // indirect
	gopkg.in/check.v1 v1.0.0-20201130134442-10cb98267c6c // indirect
)

replace buf.build/gen/go/tldiagramcom/diagram/protocolbuffers/go => ../backend/grpc/gen

replace buf.build/gen/go/tldiagramcom/diagram/connectrpc/go/diag/v1/diagv1connect => ../backend/grpc/gen/diag/v1/diagv1connect
