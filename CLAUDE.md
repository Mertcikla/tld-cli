# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

**tld** is a CLI for the tlDiagram.com architecture diagramming system. The workspace uses the unified `element/view/connector` model. New work should only touch `elements.yaml` and `connectors.yaml`; `tld plan`, `tld apply`, `tld export`, and `tld pull` bridge that workspace onto the backend's current request and export shapes.

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
YAML files in workspace/ (usually ./tld/)
  → workspace.Load()     - parse all YAML into a Workspace struct
  → ws.Validate()        - check refs, cycles, required fields
  → planner.Build()      - convert to gRPC ApplyPlanRequest + topo-sorted view order
  → planner.RenderPlanMarkdown() - human-readable preview
  → client.ApplyPlan()   - gRPC call to diag backend
  → reporter.RenderExecutionMarkdown() - execution summary
```

### Packages

- **`workspace/`** - load/validate/write/delete workspace YAML. `merger.go` handles surgical three-way merges using `yaml.Node`. `writer.go` handles cascading renames.
- **`planner/`** - `Build()` maps workspace to `ApplyPlanRequest`. During migration it can bridge `elements.yaml` and `connectors.yaml` onto the legacy backend contract.
- **`reporter/`** - renders execution result markdown.
- **`client/`** - gRPC client factory with bearer-token interceptor.
- **`cmd/`** - Cobra commands. `root.go` auto-detects `./tld/` directory.

### Command tree

```
tld
├── init [dir]         - initializes .tld/ with .tld.yaml, elements.yaml, and connectors.yaml
├── login
├── validate
├── views              - summarize derived view structure and per-view counts
├── plan [-o file]
├── apply [--auto-approve]
├── pull               - surgical three-way merge from server state
├── diff               - git-style diff between local and server state
├── status             - show sync status and merge conflicts
├── rename
│   ├── element <old> <new>
│   └── connector <old> <new>
├── add <name>         - adds or updates an element
├── connect            - adds a connector between two elements
└── remove
  ├── element <ref>
  └── connector --view --from --to
```

### Workspace file layout

```
~/.config/tldiagram/tld.yaml  # Global config: API key, org slug
.tld/
  ├── .tld.yaml              # Workspace config: project metadata, repositories, excludes
  ├── elements.yaml           # Elements + placements + canonical-view ownership
  ├── connectors.yaml         # Connectors inside element-owned views
  └── .tld.lock               # Sync state, hash, and metadata at last sync
```

Local workspaces should only contain `elements.yaml`, `connectors.yaml`, and `.tld.lock`.
Server-facing bridge logic still materializes legacy backend payloads internally during export and pull.

### Key patterns

- All commands accept `-w <dir>` (defaults to `.tld` if it exists, else `tld` if it exists, else `.`).
- `pull` uses surgical merging to preserve local comments and formatting.
- `rename` cascades changes to all files locally.
- Version conflicts are detected during `pull` (merge conflicts) and `apply` (server-side version check).
- `diff` uses `git diff --no-index` to compare local state with a temporary export of the server state.

- Tests use `t.TempDir()` + helper functions from `cmd/testhelper_test.go`.
