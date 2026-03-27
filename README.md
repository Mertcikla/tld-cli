# tld

[![Go Version](https://img.shields.io/github/go-mod/go-version/mertcikla/tld-cli)](https://go.dev/) [![License](https://img.shields.io/github/license/mertcikla/tld-cli)](./LICENSE) [![Build Status](https://img.shields.io/github/actions/workflow/status/mertcikla/tld-cli/test.yml?branch=main)](https://github.com/mertcikla/tld-cli/actions) [![Go Report Card](https://goreportcard.com/badge/github.com/mertcikla/tld-cli)](https://goreportcard.com/report/github.com/mertcikla/tld-cli)

tld is a command-line interface for managing software architecture diagrams as code. It allows you to define your architecture in YAML, validate the consistency of your definitions, and sync them to a tlDiagram server.


## Features

- **Diagrams as Code**: Define diagrams, objects, edges, and drill-down links in human-readable YAML.
- **Integrity Validation**: Check workspace integrity (e.g., checking for broken references).
- **Plan & Apply Workflow**: Preview changes with a markdown-based plan and apply them atomically.
- **Idempotent Upserts**: Safely re-run applies; the CLI tracks system IDs to update existing resources instead of duplicating them.
- **Conflict Detection**: Detects if resources have been modified on the server (e.g., via the web UI) since your last apply.
- **Export/Import**: Backup your entire organization to YAML files or restore diagrams to a new organization.
- **Edge Handles**: Specify precise connection points (handles) for edges.

## Installation

### From Source

To build and install tld from the source directory, ensure you have Go installed and run:

```bash
go install .
```

This will install the `tld` binary into your `$GOPATH/bin` directory.

## Getting Started

1. Initialize a new workspace:
   ```bash
   tld init my-architecture
   cd my-architecture
   ```

2. Edit `~/.config/tldiagram/tld.yaml` (or your OS equivalent) to configure your server URL, API key, and Organization ID.

3. Create your first diagram:
   ```bash
   tld create diagram "System Overview" --ref system-overview
   ```

4. Add an object to the diagram:
   ```bash
   tld create object system-overview "Web API" Service --technology "Go / Gin"
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
- `diagrams.yaml`: YAML file defining diagram metadata and system IDs (`_meta`).
- `objects.yaml`: YAML file defining objects and their placements on diagrams.
- `edges.yaml`: Relationship definitions between objects.
- `links.yaml`: Drill-down navigation links between diagrams.

## Commands

### Core Workflow

- `tld init [dir]`: Initialize a new workspace.
- `tld login`: Log in to the configured server.
- `tld validate`: Check the workspace for structural and reference errors.
- `tld plan`: Generate a preview of the changes and detect conflicts/drift.
- `tld apply`: Synchronize the local workspace state with the server. Supports interactive conflict resolution.
- `tld export [org-id]`: Export all diagrams from an organization to the local workspace.

### Resource Creation

- `tld create diagram <name>`: Create a new diagram definition.
- `tld create object <diagram_ref> <name> <type>`: Define a new object and place it on a diagram.
- `tld connect objects <diagram_ref> --from <source> --to <target> [--source-handle <name>] [--target-handle <name>]`: Define a relationship between two objects, optionally specifying connection handles.
- `tld add link --from <diagram> --to <diagram> [--object <ref>]`: Create a navigation link between two diagrams.

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
