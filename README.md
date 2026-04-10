# tld - CLI for diagrams as code
[![Go Version](https://img.shields.io/github/go-mod/go-version/mertcikla/tld-cli)](https://go.dev/) [![License](https://img.shields.io/github/license/mertcikla/tld-cli)](./LICENSE) [![Build Status](https://img.shields.io/github/actions/workflow/status/mertcikla/tld-cli/test.yml?branch=main)](https://github.com/mertcikla/tld-cli/actions) [![Go Report Card](https://goreportcard.com/badge/github.com/mertcikla/tld-cli)](https://goreportcard.com/report/github.com/mertcikla/tld-cli)

tld is a command-line interface for managing software architecture diagrams as code. It is a companion app designed for use with [tlDiagram.com](http://tldiagram.com/). It lets you define your architecture in YAML, validate the consistency of your definitions, and sync them to [tlDiagram.com](http://tldiagram.com/).

The workspace is currently migrating from the legacy `diagram/object/edge/link` model to the new `element/view/connector` model. The new CLI workflow writes `elements.yaml` and `connectors.yaml`, while `tld plan` and `tld apply` temporarily bridge that model onto the legacy backend contract.


## Features

- **Architecture as Code**: Define elements, views, and connectors in human-readable YAML.
- **Integrity Validation**: Check workspace integrity (e.g., checking for broken references).
- **Plan & Apply Workflow**: Preview changes with a markdown-based plan and apply them atomically.
- **Idempotent Upserts**: Safely re-run applies; the CLI tracks system IDs to update existing resources instead of duplicating them.
- **Conflict Detection**: Detects if resources have been modified on the server (e.g., via the web UI) since your last apply.
- **Export/Import**: Backup your entire organization to YAML files or restore diagrams to a new organization.
- **Edge Handles**: Specify precise connection points (handles) for edges.

## Installation

### Latest Binary

```bash
curl -LsSf https://tldiagram.com/install.sh | sh
```

### From Source

To build and install tld from the source directory, ensure you have Go installed and run:

```bash
go install .
```

This will install the `tld` binary into your `$GOPATH/bin` directory.

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
   tld create element "System Overview" --ref system-overview --kind workspace --diagram-label "System Context"
   ```

4. Add elements to that view:
   ```bash
   tld create element "Web API" --parent system-overview --kind service --technology "Go / Gin"
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
- `diagrams.yaml`, `objects.yaml`, `edges.yaml`, `links.yaml`: Legacy files retained only during the migration bridge.

## Commands

### Core Workflow

- `tld init`: Initialize a new workspace.
- `tld login`: Log in to the configured server.
- `tld validate`: Check the workspace for structural and reference errors.
- `tld plan`: Generate a preview of the changes and detect conflicts/drift.
- `tld apply`: Synchronize the local workspace state with the server. Supports interactive conflict resolution.
- `tld export [org-id]`: Export all diagrams from an organization to the local workspace.

### Resource Creation

- `tld create element <name>`: Define a new element. Every created element owns a canonical diagram.
- `tld create link --from <source_ref> --to <target_ref>`: Define a connector between two elements. The owning diagram is inferred from their shared parent placement.
- `tld connect elements --from <source> --to <target>`: Define a connector between two elements. The owning diagram is inferred from their shared parent placement.
- `tld remove element <ref>`: Remove an element from the workspace.
- `tld remove connector --view <ref> --from <source_ref> --to <target_ref>`: Remove matching connector(s).
- The legacy `create diagram` and `create object` commands have been removed. Legacy diagram/object files are still retained only for the migration bridge.

### Removing Resources

- `tld remove diagram <ref>`: Remove a diagram and cascade-delete its edges, links, and placements.
- `tld remove object <ref>`: Remove an object and cascade-delete edges and links referencing it.
- `tld remove edge --diagram <ref> --from <source_ref> --to <target_ref>`: Remove matching edge(s).
- `tld remove link --from <from_diagram_ref> --to <to_diagram_ref> [--object <ref>]`: Remove matching link(s).

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
