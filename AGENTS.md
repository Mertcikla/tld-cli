# AGENTS.md

This file provides guidance to Codex (Codex.ai/code) when working with code in this repository.

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

### Data flow

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

## analyze command architecture

### Element hierarchy
`root → repository → folder(s) → file → symbol → method/constructor`

### Parser approach
- All parsers in `src/analyzer/parsers/` use `tree_sitter::Query` (S-expression patterns), not manual node walking.
- **`QueryMatches` is NOT `std::iter::Iterator`** — it implements `StreamingIterator` (tree-sitter 0.26.x).
  Use `while let Some(m) = matches.next()` after importing `use tree_sitter::StreamingIterator;`.
- All parser `parse()` signatures take `(node, source, path, language: &Language, result)`.

### workspace_builder placement rules
- TypeScript/JavaScript/Java/Python: methods/constructors go under parent class element.
- Go/Rust: all symbols go under the file element.
- C++: constructors/destructors go under parent class; regular methods go under the file.
- File elements are only created for files that contain at least one symbol.

### Slug collision handling
- When a symbol's `slugify(name)` collides with an existing element, use `{file_stem}-{symbol_slug}`.
- Display name becomes `ClassName.methodName` (unless it's a destructor).
- `workspace_builder::build()` takes `BuildContext` and returns `BuildOutput`.

### Test reference outputs
- `tests/test-codebase/*/v1.tld/` — reference YAML from the first version of analyze.
- Test with: `tld -w /tmp/X-test analyze tests/test-codebase/X`
- The v1 reference has some known inconsistencies (e.g. Java methods placed under file instead of class).

### Files to skip in analysis
`should_skip_file()` in `workspace_builder.rs` excludes: `lock`, `toml`, `json`, `md`, `txt`, `yaml`, `yml`, `sum`, `mod`, `gitignore`, `xml`, `gradle`, `properties`.

## CI & Release

**Proto files** live in https://github.com/Mertcikla/tld-proto.git.
The `diag-proto` Rust crate (`proto/rust/`) is a **path dependency** — `tld` links directly to it.

- **Locally**: clone `tld-proto` as a sibling `../proto/` directory; `cargo build` just works.
- **CI**: workflows must clone tld-proto into the expected relative path before `cargo build`.
  ```yaml
  - uses: actions/checkout@v4
    with:
      repository: Mertcikla/tld-proto
      path: proto   # checked out at <workspace>/proto, sibling of tld
  ```
- **Updating protos**: edit `.proto` files in the proto repo, run `cd ../proto && buf generate`, then commit the updated `rust/src/diag.v1.rs` and `diag.v1.tonic.rs` to the proto repo.
- No `build.rs`, no `protoc`, no `TLD_PROTO_PATH` env var needed.

**Release flow** (`make release`):
- Bumps patch version, runs `git-cliff --tag vX.Y.Z` to update `CHANGELOG.md`, commits, tags, and pushes.
- Pushing the tag triggers `.github/workflows/release.yml` which cross-compiles for 6 targets and creates a GitHub Release.
- `make changelog` regenerates the full `CHANGELOG.md` without tagging.
- Requires `git-cliff` locally: `cargo install git-cliff`

**Changelog config:** `cliff.toml` — groups `feat`/`fix`/`perf`/`refactor`/`docs`; skips `ci`/`chore`/`test`/`style`.
