# TLD CLI Guide

`tld` is the CLI for tlDiagram.com diagram-as-code workflows.

Workspaces are local YAML files:
- `.tld/.tld.yaml` (workspace config)
- `.tld/elements.yaml`
- `.tld/connectors.yaml`
- `.tld/.tld.lock` (sync metadata after server operations)

## Global flags

- `-w, --workspace <DIR>`: workspace directory (defaults to `.tld` when present, otherwise `.`)
- `--format <text|json>`: output format (default: `text`)
- `--compact`: compact JSON output
- `-v, --verbose`: verbose output (also enables extra analyze details)

## Quick local workflow

```bash
tld init
tld add Backend --kind service --tag layer:domain
tld add Api --parent backend --technology Go --tag protocol:rest
tld add Database --parent backend --technology PostgreSQL --tag role:database
tld connect api database --view backend --label reads-writes --relationship uses
tld update element api description Public_HTTP_API
tld validate --skip-symbols
tld check
tld views
```

## Workspace config (`.tld/.tld.yaml`)

```yaml
project_name: E-commerce Platform
auto_tag: role,domain,endpoint,external
exclude:
  - node_modules
  - target
repositories:
  backend:
    url: github.com/example/backend
    localDir: backend
    root: backend-root-ref
    config:
      mode: auto
    exclude:
      - generated/**
```

- `exclude`: workspace-level ignore globs for analysis.
- `repositories`: optional multi-repo registry.
- `root`: optional canonical root element ref for that repo.
- `config.mode`: repository mode metadata used by tld workflows.

## Command reference

### `tld init [dir] [--wizard]`
Initialize workspace files. Default directory is `.tld`.

### `tld add <name> [flags]`
Create or update an element.

Flags:
- `--ref <slug>`: override the generated element reference
- `--kind <kind>`
- `--technology <text>`
- `--description <text>`
- `--url <url>`
- `--parent <element-ref>`
- `--tag <name>` (repeatable)

Element reference is generated from the name slug.

### `tld connect <source-ref> <target-ref> [flags]`
Create or update a connector.

Flags:
- `--view <element-ref>` (if omitted, inferred from shared parent or `root`)
- `--label <text>`
- `--relationship <text>`
- `--direction <forward|backward|both|none>` (default `forward`)

### `tld update element <ref> [field] [value]`
Update an element field. If `field` and `value` are omitted, prints available fields.

### `tld update connector <ref> [field] [value]`
Update a connector field. If `field` and `value` are omitted, prints available fields.

### `tld remove element <ref>`
Remove an element and dependent connectors.

### `tld remove connector --view <ref> --from <source> --to <target>`
Remove connector(s) by coordinates.

### `tld analyze <path> [flags]`
Analyze source files and replace workspace elements/connectors with analyzed output.

Flags:
- `--dry-run`
- `--download <csv-langs>`
- `--noise-threshold <int>` (default `-4`)
- `--include-low-signal`
- `--auto-tag <none|all|csv-dimensions>`
- `--lsp <auto|on|off>` (default `auto`)
- `--max-elements <n>` (must be `1..=10000`)

### `tld validate [--skip-symbols]`
Validate workspace references and semantics.

### `tld check [--strict]`
Run validation plus outdated-diagram checks.

### `tld views [--parent <ref>]`
List elements that own canonical views.

### `tld status [--long]`
Show local sync metadata from `.tld.lock`.

### Server-backed commands

These commands require configured server/auth (`tld login` or `TLD_SERVER_URL`/`TLD_API_KEY`/`TLD_ORG_ID`):

- `tld login [--server <url>] [--no-browser]`
- `tld plan [--recreate-ids] [-v|--verbose] [-o|--output <file>]`
- `tld apply [-f|--force] [--recreate-ids]`
- `tld pull [--force] [--dry-run]`
- `tld export [org_id] [-o|--output-file <file>]`
- `tld diff [--skip-symbols]`
- `tld tag create <name> [--color <#RRGGBB>] [--description <text>]`

`tld plan --output` writes the rendered plan report to the target file instead of stdout.
