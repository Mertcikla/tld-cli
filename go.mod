module github.com/mertcikla/tld-cli

go 1.26.1

require (
	buf.build/gen/go/tldiagramcom/diagram/connectrpc/go/diag/v1/diagv1connect v1.19.1-20260328022351-cdc7b18ed408.2
	buf.build/gen/go/tldiagramcom/diagram/protocolbuffers/go v1.36.11-20260328022351-cdc7b18ed408.1
	connectrpc.com/connect v1.19.1
	github.com/speps/go-hashids/v2 v2.0.1
	github.com/spf13/cobra v1.9.1
	google.golang.org/protobuf v1.36.11
	gopkg.in/yaml.v3 v3.0.1
)

require (
	github.com/inconshreveable/mousetrap v1.1.0 // indirect
	github.com/kr/pretty v0.3.1 // indirect
	github.com/spf13/pflag v1.0.6 // indirect
	github.com/tetratelabs/wazero v1.11.0 // indirect
	golang.org/x/sys v0.38.0 // indirect
	gopkg.in/check.v1 v1.0.0-20201130134442-10cb98267c6c // indirect
)

replace buf.build/gen/go/tldiagramcom/diagram/protocolbuffers/go => ../backend/grpc/gen

replace buf.build/gen/go/tldiagramcom/diagram/connectrpc/go/diag/v1/diagv1connect => ../backend/grpc/gen/diag/v1/diagv1connect
