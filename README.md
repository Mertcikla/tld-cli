
# ! The Functionality of the CLI moved !
[https://github.com/Mertcikla/tld](https://github.com/Mertcikla/tld)


## tld - CLI for diagrams as code
[![License](https://img.shields.io/github/license/mertcikla/tld-cli)](./LICENSE) [![Build Status](https://img.shields.io/github/actions/workflow/status/mertcikla/tld-cli/test.yml?branch=main)](https://github.com/mertcikla/tld-cli/actions) 



tld is a command-line interface for managing software architecture diagrams as code. It is a companion app designed for use with [tlDiagram.com](http://tldiagram.com/). It lets you define your architecture in YAML, validate the consistency of your definitions, and sync them to [tlDiagram.com](http://tldiagram.com/).

The workspace uses the unified `element/view/connector` model. The CLI writes `elements.yaml` and `connectors.yaml`, while `tld plan`, `tld apply`, `tld export`, and `tld pull` bridge that model onto the backend's current export/apply contract.


## Features

- **Architecture as Code**: Define elements, views, and connectors in human-readable YAML.
- **Integrity Validation**: Check workspace integrity (e.g., checking for broken references).
- **Plan & Apply Workflow**: Preview changes with a markdown-based plan and apply them atomically.
- **Idempotent Upserts**: Safely re-run applies; the CLI tracks system IDs to update existing resources instead of duplicating them.
- **Conflict Detection**: Detects if resources have been modified on the server (e.g., via the web UI) since your last apply.
- **Export/Import**: Backup your entire organization to YAML files or restore diagrams to a new organization.
- **Connector Handles**: Specify precise connection points (handles) for connectors.

## Installation

### Latest Binary

```bash
curl -LsSf https://tldiagram.com/install.sh | sh
```

### From Source

To build and install tld from the source directory, ensure you have Rust installed and run:

```bash
cargo install --path .
```

This will install the `tld` binary into your `$CARGO_HOME/bin` directory.

## Getting Started

1. Initialize a new workspace:
   ```bash
   tld init
   ```

2. Authorize tld with your tlDiagram.com account.
   ```bash
   tld login
   ```

3. Create your first element:
   ```bash
   tld add "System Overview" --kind workspace
   ```

4. Add elements to that view:
   ```bash
   tld add "Web API" --ref web-api --parent system-overview --kind service --technology "Go / Gin"
   ```

5. Validate your workspace:
   ```bash
   tld validate
   ```

6. Preview and apply changes:
   ```bash
   tld plan
   tld apply
   ```

## Workspace Structure

A tld workspace consists of the following directory structure:

- `~/.config/tldiagram/tld.yaml` (or your OS equivalent): Configuration file for server connection and organization details.
- `.tld.lock`: (Generated) Lock file for workspace versioning and change tracking.
- `elements.yaml`: YAML file defining elements, their canonical diagrams, and their placements.
- `connectors.yaml`: Relationship definitions between elements inside the inferred parent diagram.
- Local workspaces should only contain `elements.yaml`, `connectors.yaml`, and `.tld.lock`.

## Commands

### Core Workflow

- `tld init`: Initialize a new workspace.
- `tld login`: Log in to the configured server.
- `tld validate`: Check the workspace for structural and reference errors.
- `tld views`: Show the derived workspace view structure, including root/owned views, depth, child counts, and connector counts.
- `tld plan`: Generate a preview of the changes and detect conflicts/drift.
- `tld apply`: Synchronize the local workspace state with the server. Supports interactive conflict resolution.
- `tld export [org-id]`: Export all diagrams from an organization to the local workspace.

### Resource Creation

- `tld add <name>`: Define a new element. Every created element owns a canonical diagram.
  - `--ref <slug>`: Override the generated slug/ref for the element.
  - `--tag <name>`: Apply a tag to the element (can be specified multiple times; idempotent).
  - Example: `tld add "Web API" --ref web-api --kind service --tag backend --tag api`
- `tld connect <source> <target>`: Define a connector between two elements. The owning diagram is inferred from their shared parent placement.
- `tld remove element <ref>`: Remove an element from the workspace.
- `tld remove connector --view <ref> --from <source_ref> --to <target_ref>`: Remove matching connector(s).
- `tld update element <ref> <field> <value>`: Update an element field. Use `field=ref` to rename an element reference and cascade the change through placements and connector endpoints.
- `tld update connector <ref> <field> <value>`: Update a connector field. Use `field=ref` to rename a connector key.

### Tags

- `tld tag create <name>`: Create or update a tag in the organization on the server.
  - `--color <hex>`: Tag color as a hex string (default: `#888888`).
  - `--description <text>`: Optional description for the tag.
  - Example: `tld tag create backend --color "#4A90E2" --description "Backend services"`

## Conflict Resolution

When you run `tld apply`, the CLI performs a server-side dry run to detect if any resources have been modified on the server since they were last synced to your local workspace. 

If conflicts are detected, you will be prompted with options:
1. **Abort**: Stop the apply and review changes.
2. **Force Apply**: Overwrite remote changes with your local state.
3. **Review**: Review conflicts one-by-one (coming soon).

## Configuration

The `~/.config/tldiagram/tld.yaml` (or your OS equivalent) file supports the following fields:

- `server_url`: The URL of your tlDiagram instance.
- `api_key`: Your personal API key (can also be set via `TLD_API_KEY`).
- `org_id`: The UUID of the organization you are managing.

## Environment Variables

- `TLD_API_KEY`: API key for authentication (overrides `~/.config/tldiagram/tld.yaml` (or your OS equivalent)).
