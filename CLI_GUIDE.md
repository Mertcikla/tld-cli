# TLD CLI Guide

`tld` is a CLI for tlDiagram.com. It allows you to define, plan, and apply your system architecture using configuration files and commands.

# Usage guide
To efficiently map your codebase with this app, adopt a top-down workflow centered on element-owned views. Start by creating a root element that defines your system's high-level boundaries, such as your API gateway, main services, and external dependencies. Identify the primary entry points (for example `main.go` or `App.tsx`) and add them as the first set of child elements. Use the tld CLI to define these programmatically in YAML, which allows you to version-control your architecture alongside your source code and keeps your architectural source of truth as dynamic as the code itself.

Once the high-level actors are connected via connectors that represent data flow or dependencies, give the complex elements their own canonical internal views. Instead of overcrowding a single canvas, enter the view owned by a specific service or module and model its internal components there. Because the data model separates element identity from placement, you can reuse the same actor, such as a shared database or auth service, across multiple parent views.

As your codebase evolves, iterate by refining these nested views and adding new connectors for emerging complexity. This recursive approach, identifying actors, connecting them, and then opening the most complex elements into their own views, prevents visual clutter and mirrors the natural mental model of software abstraction. By leveraging the `tld plan` and `tld apply` workflow, you can automate parts of this process and keep your documentation in sync with a rapidly growing project.

## Global Options

- `-w, --workspace`: Specify the workspace directory. Defaults to the current directory (`.`).

## Commands Reference

### Initialization

#### `tld init [dir]`
Initialize a new `tld` workspace. This command ensures the global configuration setup exists in `~/.config/tldiagram/tld.yaml` and prepares the specified directory (or the current directory if omitted) as a workspace.

### Resource Creation

#### `tld create element <name> [flags]`
Create or update an element in `elements.yaml`.
Each created element owns a single canonical diagram.
**Flags:**
- `--kind`: Element kind (default: `service`).
- `--description`: Description of the element.
- `--technology`: Primary technology used by the element.
- `--url`: External URL for further reference.
- `--position-x`: Horizontal canvas position inside the parent view.
- `--position-y`: Vertical canvas position inside the parent view.
- `--parent`: Parent element reference or `root` (default: `root`).
- `--diagram-label`: Optional label for that canonical diagram.
- `--ref`: Override the generated reference ID.

#### `tld connect elements [flags]`
Add a connector between two elements.
The owning diagram is inferred from the two elements' shared parent placement.
**Flags:**
- `--from`: Source element reference **(required)**.
- `--to`: Target element reference **(required)**.
- `--label`: Connector label.
- `--description`: Connector description.
- `--relationship`: Semantic relationship type.
- `--direction`: Direction of the arrow. Options: `forward`, `backward`, `both`, `none` (default: `forward`).
- `--style`: Visual style of the connector. Options: `bezier`, `straight`, `step`, `smoothstep` (default: `bezier`).
- `--url`: External URL for documentation related to this connection.

The legacy `create diagram` and `create object` commands have been removed from the CLI surface. Legacy diagram/object files remain only for the migration bridge.

### Connecting and Linking Resources

#### `tld connect objects <diagram_ref> [flags]`
Add an edge (connection) between two objects on a specified diagram.
**Flags:**
- `--from`: Source object reference **(required)**.
- `--to`: Target object reference **(required)**.
- `--label`: Edge label (e.g., "Makes API calls to").
- `--description`: Detailed edge description.
- `--relationship-type`: Semantic relationship type (e.g., "uses", "depends_on").
- `--direction`: Direction of the arrow. Options: `forward`, `backward`, `both`, `none` (default: `forward`).
- `--edge-type`: Visual style of the edge. Options: `bezier`, `straight`, `step`, `smoothstep` (default: `bezier`).
- `--url`: External URL for documentation related to this connection.

#### `tld create link [flags]`
Create a connector between two elements.
The owning diagram is inferred from the two elements' shared parent placement.
**Flags:**
- `--from`: Source element reference **(required)**.
- `--to`: Target element reference **(required)**.
- `--label`: Connector label.
- `--description`: Connector description.
- `--relationship`: Semantic relationship type.
- `--direction`: Direction of the arrow. Options: `forward`, `backward`, `both`, `none` (default: `forward`).
- `--style`: Visual style of the connector. Options: `bezier`, `straight`, `step`, `smoothstep` (default: `bezier`).
- `--url`: External URL for documentation related to this connection.

#### `tld update object <ref> [flags]`
Update an object's properties in the workspace YAML. Run `tld apply` to sync changes to the server.
**Flags:**
- `--name`: New name for the object.
- `--type`: New architectural type.
- `--description`: New description.
- `--technology`: New primary technology.
- `--url`: New external URL.

#### `tld update diagram <ref> [flags]`
Update a diagram's properties in the workspace YAML. Run `tld apply` to sync changes to the server.
**Flags:**
- `--name`: New name for the diagram.
- `--description`: New description.
- `--level-label`: New abstraction level label.

#### `tld update edge [flags]`
Update an edge's properties in the workspace YAML. Run `tld apply` to sync changes to the server.
**Flags:**
- `--diagram`: Diagram reference (required).
- `--from`: Source object reference (required).
- `--to`: Target object reference (required).
- `--label`: Current label (required if multiple edges exist between the same objects).
- `--new-label`: New label for the edge.
- `--description`: New description.
- `--direction`: New direction (`forward`, `backward`, `both`, `none`).
- `--edge-type`: New visual style (`bezier`, `straight`, `step`, `smoothstep`).

### Workspace Workflow

#### `tld validate`
Validate the workspace YAML files to ensure semantic correctness, referential integrity, and required fields. It also checks for duplicate names to prevent confusion and slug collisions.

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
- `-v, --verbose`: Show detailed resource reporting, including all objects per diagram, edges, and links.
- `-o, --output`: Write the plan to a specified file instead of printing to standard output.

#### `tld apply [flags]`
Apply the generated plan or the current workspace state to the diag server.
**Flags:**
- `--auto-approve`: Skip the interactive approval prompt and apply immediately.

---

## Diagram as Code Best Practices

1. **Version Control**: Keep your `.yaml` files and `.tld.lock` in Git. This allows you to track architectural changes alongside code changes.
2. **Stable Refs**: While `tld` automatically generates slugs from names, you can override them with `--ref`. Once a resource is created, its ref is stored in the YAML keys. `tld` tracks the underlying system ID in the `_meta` section, allowing you to rename resources without losing history.
3. **Bi-directional Sync**: Use `tld pull` regularly if your team makes changes in the tlDiagram web UI. This ensures your local YAML files remain the source of truth for the latest layout and coordinates.
4. **Validation**: Run `tld validate` in your CI/CD pipeline to ensure that any architectural changes proposed in Pull Requests are semantically correct.

This example walks through building a system context architecture for an e-commerce platform from scratch using the `tld` CLI.

### 1. Initialize the Workspace
First, create a new directory and initialize the `tld` workspace.
```bash
mkdir ecom-arch && cd ecom-arch
tld init .
```

### 2. Create the System Context Element
Create the high-level system context element and give it a canonical view.
```bash
tld create element "E-commerce Platform" --kind workspace --diagram-label "System Context" --description "High-level overview of the e-commerce system."
```
*(This generates an element with the reference ID: `e-commerce-platform`)*

### 3. Add Elements (Actors and Systems)
Add the primary users and systems to the root view.
```bash
# Add a Customer actor
tld create element "Customer" --parent e-commerce-platform --kind person --description "A customer of the e-commerce platform." --position-x 100 --position-y 300

# Add the main E-commerce Web Application
tld create element "Web App" --parent e-commerce-platform --kind software_system --description "The core e-commerce web application." --position-x 400 --position-y 300

# Add an external Payment Gateway
tld create element "Payment Gateway" --parent e-commerce-platform --kind software_system --description "External payment processor (e.g., Stripe)." --position-x 700 --position-y 300
```

### 4. Connect the Elements
Define how these systems and actors interact.
```bash
# Customer interacts with the Web App
tld connect elements --from "customer" --to "web-app" --label "Browses and buys products" --relationship "uses"

# Web App integrates with the Payment Gateway
tld connect elements --from "web-app" --to "payment-gateway" --label "Processes payments using" --relationship "integrates"
```

### 5. Validate and Apply
Ensure everything is configured correctly, check the plan, and deploy the architecture.
```bash
# Check for any configuration errors
tld validate

# Review the changes that will be made
tld plan
# Or for a detailed view:
tld plan -v

# Apply the changes to the diag server
tld apply --auto-approve
```

`plan` and `apply` currently bridge the element/view/connector workspace onto the legacy backend request shape. That bridge is temporary and will be removed once the backend contract is migrated.
