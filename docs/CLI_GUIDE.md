# TLD CLI Guide

`tld` is a CLI for tlDiagram.com. It allows you to define, plan, and apply your system architecture using configuration files and commands.

# Usage guide
To efficiently map your codebase with this app, adopt a top-down workflow centered on element-owned views. Start by creating a root element that defines your system's high-level boundaries, such as your API gateway, main services, and external dependencies. Identify the primary entry points (for example `main.go` or `App.tsx`) and add them as the first set of child elements. Use the tld CLI to define these programmatically in YAML, which allows you to version-control your architecture alongside your source code and keeps your architectural source of truth as dynamic as the code itself.

Once the high-level actors are connected via connectors that represent data flow or dependencies, give the complex elements their own canonical internal views. Instead of overcrowding a single canvas, enter the view owned by a specific service or module and model its internal components there. Because the data model separates element identity from placement, you can reuse the same actor, such as a shared database or auth service, across multiple parent views.

As your codebase evolves, iterate by refining these nested views and adding new connectors for emerging complexity. This recursive approach, identifying actors, connecting them, and then opening the most complex elements into their own views, prevents visual clutter and mirrors the natural mental model of software abstraction. By leveraging the `tld plan` and `tld apply` workflow, you can automate parts of this process and keep your documentation in sync with a rapidly growing project.

## Global Options

- `-w, --workspace`: Specify the workspace directory. Defaults to the current directory (`.`).

## Workspace Configuration

`tld init` creates a workspace-local [.tld.yaml](.tld.yaml) file in the workspace root. This file now holds the project settings that used to live in a separate ignore file.

Example:

```yaml
# tld workspace configuration
project_name: "E-commerce Platform"
exclude:
  - vendor/
  - node_modules/
  - .git/
  - "**/*_test.go"
  - "**/*.pb.go"
repositories:
  frontend:
    url: github.com/example/frontend
    localDir: frontend
    root: bKLqGV48
    config:
      mode: auto
    exclude:
      - generated/**
  backend:
    url: github.com/example/backend
    localDir: backend
    root: Zx91QpLm
    config:
      mode: manual
    exclude:
      - internal/experimental/**
  payments:
    url: github.com/example/payments
    localDir: services/payments
    root: 7nKp2qV1
```

- `exclude` is a gitignore-style glob list. It applies globally when set at the top level, and each repository can add its own `exclude` list.
- `repositories` lists the repositories that belong to this workspace. Each entry keeps the remote URL and local checkout path together.
- `root` is the hashid of the repository's root element in the diagram.
- `config.mode` is reserved for future repository-specific tld-cli behavior. Supported values are `auto`, `upsert`, and `manual`.
- Multi-repo workspaces can contain diagrams from 0 or more repositories, including repos owned by different teams.

## Commands Reference

### Initialization

#### `tld init [dir]`
Initialize a new `tld` workspace. This command ensures the global configuration setup exists in `~/.config/tldiagram/tld.yaml` and prepares the specified directory (or the current directory if omitted) as a workspace.

It also creates the workspace-local `.tld.yaml` file with default global exclusions and repository registry scaffolding.

#### `tld analyze <path> [flags]`
Extract code symbols from one repo or from a workspace root and write them into `elements.yaml` and `connectors.yaml`.
**Flags:**
- `--deep`: Scan the entire git repository for cross-file references.
- `--dry-run`: Show what would be written without modifying the workspace.
- `--changed-since <ref>`: Only analyze files changed relative to a git ref.
- `--download <langs>`: Download missing tree-sitter parsers before scanning, for example `--download rust,python`.
- `--lsp`: Ask available language servers for definition lookups to improve cross-file call resolution.
- `--view <mode>`: Choose `structural`, `business`, or `data-flow` output.
- `--noise-threshold <score>`: Hide low-salience nodes in semantic views.
- `--include-low-signal`: Disable pruning in business view and keep low-salience nodes.

`analyze` currently plans a scan around the requested path and can expand that scope to the enclosing git root with `--deep`. Business and data-flow views use the semantic projection pipeline, so they intentionally hide low-signal symbols unless you lower the threshold or opt into `--include-low-signal`.

#### `tld check [flags]`
Validate the workspace YAML and verify linked code symbols in the active repository.
**Flags:**
- `--strict`: Exit non-zero when outdated diagrams are detected.

`check` is CI-friendly and only validates symbols that belong to the repo you are currently operating on.

### Resource Creation

#### `tld add <name> [flags]`
Create or update an element in `elements.yaml`.
Each created element owns a single canonical diagram.
**Flags:**
- `--kind`: Element kind (default: `service`).
- `--description`: Description of the element.
- `--technology`: Primary technology used by the element.
- `--url`: External URL for further reference.
- `--parent`: Parent element reference or `root` (default: `root`).
- `--diagram-label`: Optional label for that canonical diagram.
- `--ref`: Override the generated reference ID.

#### `tld connect [flags]`
Add a connector between two elements.
The owning diagram is inferred from the two elements' shared parent placement.
**Flags:**
- `--from`: Source element reference **(required)**.
- `--to`: Target element reference **(required)**.
- `--label`: Connector label.
- `--description`: Connector description.
- `--relationship`: Semantic relationship type.
- `--direction`: Direction of the arrow. Options: `forward`, `backward`, `both`, `none` (default: `forward`).
- `--url`: External URL for documentation related to this connection.

The CLI only exposes the unified element and connector workflow. Server export and pull still bridge legacy backend payloads internally, but local workspaces should contain only `elements.yaml`, `connectors.yaml`, and `.tld.lock`.

### Connecting and Linking Resources

#### `tld update element <ref> <field> <value>`
Update one element field in `elements.yaml`. Use `field=ref` to rename an element reference and cascade the change through placements, connector endpoints, and metadata.

#### `tld update connector <ref> <field> <value>`
Update one connector field in `connectors.yaml`. Updating `view`, `source`, `target`, or `label` regenerates the connector key. Use `field=ref` to rename a connector key directly.

### Workspace Workflow

#### `tld validate`
Validate the workspace YAML files to ensure semantic correctness, referential integrity, and required fields. It also checks for duplicate names to prevent confusion and slug collisions.

When an element has both `file_path` and `symbol`, `validate` confirms that the symbol exists in the active repository. Elements from other repositories in the same workspace are left alone.

#### `tld status [flags]`
Show the sync status between your local YAML files and the last known sync point (lock file).
**Flags:**
- `--check-server`: Perform a live dry-run on the server to detect drift from manual changes in the frontend UI.

#### `tld pull [flags]`
Fetch the latest state from tlDiagram.com and update local YAML files. This is the recommended way to sync changes made in the web UI back to your codebase. `tld` uses stable resource IDs (stored in `_meta`) to correctly handle renames and coordinate updates.
**Flags:**
- `--force`: Overwrite local changes without prompting.
- `--dry-run`: Show what would be pulled without writing to files.

#### `tld plan [flags]`
Analyze the workspace and show what changes would be applied to the diag server. By default, it shows a high-level summary and the diagram hierarchy. It uses detailed drift analysis to show exactly what properties have changed.
**Flags:**
- `-v, --verbose`: Show detailed resource reporting for element placements and connectors.
- `-o, --output`: Write the plan to a specified file instead of printing to standard output.

#### `tld apply [flags]`
Apply the generated plan or the current workspace state to the diag server.
**Flags:**
- `--force`: Skip the interactive approval prompt and apply immediately.

If conflicts are detected, the dialog includes a Pull & Merge option that fetches server state, merges locally, and re-applies using the merged workspace.

---

## Diagram as Code Best Practices

1. **Version Control**: Keep your `.yaml` files and `.tld.lock` in Git. This allows you to track architectural changes alongside code changes.
2. **Stable Refs**: While `tld` automatically generates slugs from names, you can override them with `--ref`. Once a resource is created, its ref is stored in the YAML keys. `tld` tracks the underlying system ID in the `_meta` section, allowing you to rename resources without losing history.
3. **Bi-directional Sync**: Use `tld pull` regularly if your team makes changes in the tlDiagram web UI. This ensures your local YAML files remain the source of truth for the latest layout and coordinates.
4. **Validation**: Run `tld check` in your CI/CD pipeline to ensure that any architectural changes proposed in Pull Requests are semantically correct and that linked symbols still exist in the active repo.

5. **Multi-repo discovery**: Keep `repositories` up to date as teams add new repos to the product. `tld analyze` uses that registry to understand which repositories belong to the workspace.

This example walks through building a product-level workspace for an e-commerce platform with multiple repositories using the `tld` CLI.

### 1. Initialize the Workspace
First, create a new workspace directory and initialize the `tld` workspace.
```bash
mkdir ecom-arch
cd ecom-arch
tld init .
```

After initialization, edit [.tld.yaml](.tld.yaml) so it lists the repositories that belong to this product.

```yaml
project_name: "E-commerce Platform"
exclude:
  - vendor/
  - node_modules/
  - .git/
  - "**/*_test.go"
  - "**/*.pb.go"
repositories:
  frontend:
    url: github.com/example/frontend
    localDir: frontend
    exclude:
      - generated/**
  backend:
    url: github.com/example/backend
    localDir: backend
    exclude:
      - internal/experimental/**
  payments:
    url: github.com/example/payments
    localDir: services/payments
```

### 2. Add the product-level architecture
Create the top-level element that owns the overall system view.
```bash
tld add "E-commerce Platform" --kind workspace --diagram-label "System Context" --description "High-level overview of the product architecture."
```

Then add one child element per repo-owned boundary.

```bash
tld add "Frontend App" --parent e-commerce-platform --kind software_system --file-path frontend/src/main.tsx --symbol App --description "React frontend owned by the frontend repo." --position-x 100 --position-y 260
tld add "API Service" --parent e-commerce-platform --kind software_system --file-path backend/cmd/server/main.go --symbol main --description "Go backend owned by the backend repo." --position-x 420 --position-y 260
tld add "Payments Worker" --parent e-commerce-platform --kind software_system --file-path services/payments/main.go --symbol main --description "Payments service owned by the payments team." --position-x 740 --position-y 260
```

### 3. Add internal repo symbols
Use `tld analyze` to populate code-backed elements for a repo.
```bash
tld analyze ./frontend --dry-run
tld analyze ./backend --dry-run
tld analyze ./services/payments --dry-run
```

When you are ready, run it without `--dry-run` to write `elements.yaml` and `connectors.yaml`.

### 4. Connect the main dependencies
Define how the major systems interact.
```bash
tld connect --from "frontend-app" --to "api-service" --label "Calls API" --relationship "uses"
tld connect --from "api-service" --to "payments-worker" --label "Dispatches payment jobs" --relationship "publishes"
```

### 5. Validate the active repo
Run validation from inside a repository to verify only that repo's linked symbols.
```bash
cd frontend
tld validate

cd ../backend
tld check --strict
```

The `check` command is intended for CI/CD and will report broken symbols, validation errors, and outdated elements.

### 6. Review the plan and apply
Use the normal plan/apply flow once the workspace is in sync.
```bash
tld plan -v
tld apply --force
```

### 7. Keep it current
As new repositories are added to the product, update [.tld.yaml](.tld.yaml) with the new `repositories` entry and rerun `tld analyze` for that repo.

### Example workspace layout

```text
ecom-arch/
  .tld.yaml
  elements.yaml
  connectors.yaml
  frontend/
    .git/
    src/
  backend/
    .git/
    cmd/
  services/
    payments/
      .git/
      main.go
```

With this layout, the workspace can hold diagrams from multiple repositories, and the CLI will only verify symbols for the repo it is currently running in.

### What changed from the old flow

- `ignore.yaml` is gone as a separate file; those settings now live in `.tld.yaml` as `exclude` lists.
- `tld analyze` can discover multiple configured repositories from the workspace root.
- `tld check` verifies symbols only for the active repository.
- `tld validate` remains the structural check for the whole workspace.

`plan` and `apply` currently bridge the element/view/connector workspace onto the legacy backend request shape. That bridge is temporary and will be removed once the backend contract is migrated.
