# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

**tld** is a CLI for the tlDiagram.com architecture diagramming system. Users define diagrams, objects, edges, and drill-down links as YAML files, preview changes with `tld plan`, and apply them atomically to a diag backend server via gRPC.

## Development commands

```bash
make build        # Compile binary: go build -o tld .
make test         # Full test suite (all packages)
make test-unit    # Unit tests: workspace/, planner/, reporter/
make test-cmd     # CLI command tests: cmd/
make test-stage4  # Only TestApplyCmd (integration with mock gRPC server)
```

## Release process

The project uses **Semantic Release** to automate versioning and tagging based on Conventional Commits.
- **Workflow:** On push to `main`, the `Tag` workflow runs `semantic-release`.
- **Versioning:** It analyzes commits (feat/fix/etc.), and creates a new git tag (e.g., `v1.2.3`).
- **Artifacts:** Tag pushes trigger the `Release` workflow, which runs **GoReleaser** to build binaries and create the GitHub release.

Follow [Conventional Commits](https://www.conventionalcommits.org/) for automated releases.

Run a single test:
```bash
go test ./cmd/... -run TestPlanCmd -count=1
go test ./workspace/... -run TestLoader -count=1
```

## Architecture

### Data flow

```
YAML files in workspace/
  → workspace.Load()     - parse all YAML into a Workspace struct
  → ws.Validate()        - check refs, cycles, required fields
  → planner.Build()      - convert to gRPC ApplyPlanRequest + topo-sorted DiagramOrder
  → planner.RenderPlanMarkdown() - human-readable preview
  → client.ApplyPlan()   - gRPC call to diag backend
  → reporter.RenderExecutionMarkdown() - execution summary
```

### Packages

- **`workspace/`** - load/validate/write/delete workspace YAML. Types mirror gRPC protos. `writer.go` contains `Slugify()` for name → ref conversion. `deleter.go` handles cascade deletion (removing a diagram/object removes dependent edges, links, and placements).
- **`planner/`** - `Build()` maps workspace to `ApplyPlanRequest`; `topoSortDiagrams()` (Kahn's algorithm) ensures parents are created before children.
- **`reporter/`** - renders execution result markdown (planned vs. created counts, drift items).
- **`client/`** - gRPC client factory with bearer-token interceptor.
- **`cmd/`** - Cobra commands; each command validates then plans/applies. `groups.go` registers `create`, `connect`, `add`, and `remove` subcommand groups.

### Command tree

```
tld
├── init [dir]
├── login
├── validate
├── plan [-o file]
├── apply [--auto-approve]
├── create
│   ├── diagram <name> [--ref --description --level-label --parent]
│   └── object <diagram_ref> <name> <type> [--ref --description --technology --url --position-x --position-y]
├── connect
│   └── objects <diagram_ref> --from --to [--label --relationship-type --direction --edge-type]
├── add
│   └── link --from --to [--object]
└── remove
    ├── diagram <ref>
    ├── object <ref>
    ├── edge --diagram --from --to
    └── link --from --to [--object]
```

### Workspace file layout

```
~/.config/tldiagram/tld.yaml              # Config: server URL, API key, org slug
diagrams.yaml          # All diagrams in one map
objects.yaml           # All objects in one map
edges.yaml             # All edges in one list
links.yaml             # All drill-down links in one list
```

### Key patterns

- All commands accept `-w <dir>` (default `.`) for the workspace root.
- `apply` prompts for confirmation unless `--auto-approve` is passed.
- API key can be set via `TLD_API_KEY` env var instead of `~/.config/tldiagram/tld.yaml`.
- `remove diagram` and `remove object` cascade: they remove the entry and filter out all dependent edges, links, and placements from the consolidated files.

- Tests use `t.TempDir()` + helper functions from `cmd/testhelper_test.go`; `apply_test.go` spins up a mock Connect RPC server in-process.
