# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

**tld** is a CLI for the tlDiagram.com architecture diagramming system. The workspace uses the unified `element/view/connector` model. New work should only touch `elements.yaml` and `connectors.yaml`; `tld plan`, `tld apply`, `tld export`, and `tld pull` bridge that workspace onto the backend's current request and export shapes.

## Development commands

The project is built in **Rust**.

```bash
make build        # Compile binary: cargo build
make dev          # Run with args: cargo run -- <args>
make test         # Run all tests: cargo test
make fmt          # Format code: cargo fmt
make lint         # Lint code: cargo clippy
make install      # Install to path: cargo install --path .
```

Run a single test:
```bash
cargo test --package tld --lib workspace::tests::test_load -- --nocapture
```

## Architecture

### Data flow (Rust)

```
YAML files in workspace/ (usually ./tld/)
  → workspace::load()     - parse all YAML into a Workspace struct
  → ws.validate()         - check refs, cycles, required fields
  → planner::build()      - convert to gRPC ApplyPlanRequest + topo-sorted view order
  → planner::render_plan_markdown() - human-readable preview
  → client::apply_plan()  - gRPC call to diag backend
  → output::print_*       - render formatted text or JSON results
```

### Modules (src/)

- **`workspace/`** - load/validate/write/delete workspace YAML. Handles surgical three-way merges using `yaml-rust`.
- **`planner/`** - `build()` maps workspace to `ApplyPlanRequest`.
- **`analyzer/`** - `treesitter` based code analysis for symbol extraction.
- **`client/`** - gRPC client using `tonic`.
- **`cli/`** - Command definitions using `clap`. Mirrors the expected CLI structure.
- **`output/`** - Formatting and terminal UI helpers (spinners, tables, colors).

### Command tree

```
tld
├── init               - initializes .tld/ with .tld.yaml, elements.yaml, and connectors.yaml
├── login
├── validate
├── views              - summarize derived view structure and per-view counts
├── plan [-o file]
├── apply [--force]
├── pull               - surgical three-way merge from server state
├── diff               - git-style diff between local and server state
├── status             - show sync status and merge conflicts
├── add <name>         - adds or updates an element
├── connect            - adds a connector between two elements
├── remove             - removes workspace resources
├── analyze <path>     - extracts symbols from source files
└── check              - check workspace for architectural issues
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

### Key patterns

- All commands accept `-w <dir>` (global flag).
- `pull` uses surgical merging to preserve local comments and formatting.
- `analyze` uses tree-sitter to build the architecture plan from source code.
- Version conflicts are detected during `pull` (merge conflicts) and `apply` (server-side version check).
- `diff` compares local state with remote state.

- Tests use `tempfile` for workspace isolation.

## CI & Release

**Proto files** live in https://github.com/Mertcikla/tld-proto.git (not vendored).
`build.rs` reads `TLD_PROTO_PATH` env var for the proto repo root; falls back to the hardcoded local sibling path.
In CI, the workflows clone that repo and set the env var automatically.

**Release flow** (`make release`):
- Bumps patch version, runs `git-cliff --tag vX.Y.Z` to update `CHANGELOG.md`, commits, tags, and pushes.
- Pushing the tag triggers `.github/workflows/release.yml` which cross-compiles for 6 targets and creates a GitHub Release.
- `make changelog` regenerates the full `CHANGELOG.md` without tagging.
- Requires `git-cliff` locally: `cargo install git-cliff`

**Changelog config:** `cliff.toml` — groups `feat`/`fix`/`perf`/`refactor`/`docs`; skips `ci`/`chore`/`test`/`style`.
