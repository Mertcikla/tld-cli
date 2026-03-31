# TLD CLI Guide

`tld` is a CLI for tlDiagram.com. It allows you to define, plan, and apply your system architecture diagrams using configuration files and commands.

# Usage guide
To efficiently map your codebase with this app, adopt a top-down architectural mapping workflow centered on the "Infinite Zoom" feature. Start by creating a root diagram that defines your system's high-level boundaries-such as your API gateway, main services, and external dependencies. Identify the primary entry points (e.g., main.go or App.tsx) and add them as the first set of objects. Use the tld CLI to define these programmatically in YAML, which allows you to version-control your architecture alongside your source code and ensures that your "source of truth" for the system design remains as dynamic as the code itself.

Once the high-level actors are connected via edges that represent data flow or dependencies, use the object-linking capability to drill down. Instead of overcrowding a single canvas, link a complex object (like a specific microservice or a core module) to its own sub-diagram. This creates a nested hierarchy where an object on the root canvas acts as a portal to a more granular view of its internal components. Because the data model separates the object definition from its placement, you can reuse the same actor (like a shared database or auth service) across multiple sub-diagrams, with the UI providing a "Find" action to quickly locate where else that component exists in your architecture.

As your codebase evolves, iterate by refining these sub-diagrams and adding new links for emerging complexity. This recursive approach-identifying actors, connecting them, and then "expanding" the most complex nodes into their own diagrams-prevents visual clutter and mirrors the natural mental model of software abstraction. By leveraging the tld plan/apply workflow, you can automate parts of this process, such as parsing directory structures to generate initial objects, making it the most scalable way to keep your documentation in sync with a rapidly growing project.

## Global Options

- `-w, --workspace`: Specify the workspace directory. Defaults to the current directory (`.`).

## Commands Reference

### Initialization

#### `tld init [dir]`
Initialize a new `tld` workspace. This command ensures the global configuration setup exists in `~/.config/tldiagram/tld.yaml` and prepares the specified directory (or the current directory if omitted) as a workspace.

### Resource Creation

#### `tld create diagram <name> [flags]`
Create a new diagram YAML file.
**Flags:**
- `--description`: Description of the diagram.
- `--level-label`: Abstraction level label (e.g., System, Container, Component).
- `--parent`: Parent diagram reference (if this diagram is a drill-down).
- `--ref`: Override the automatically generated reference ID (default is the slugified name).

#### `tld create object <diagram_ref> <name> <type> [flags]`
Create a new object YAML file and place it on the specified diagram.
**Note:** This command is idempotent. If the object (identified by its reference ID) already exists, `tld` will simply add a new placement for that object on the specified diagram, allowing you to easily reuse the same object across multiple diagrams.
**Flags:**
- `--description`: Description of the object.
- `--technology`: Primary technology used by the object (e.g., Go, React, PostgreSQL).
- `--url`: External URL for further reference.
- `--position-x`: Horizontal canvas position (float).
- `--position-y`: Vertical canvas position (float).
- `--ref`: Override the generated reference ID.

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
Create a drill-down link between two diagrams via a specific object.
**Flags:**
- `--object`: Object reference that will trigger the drill-down **(optional)**.
- `--from`: Source diagram reference **(required)**.
- `--to`: Target diagram reference to drill down into **(required)**.

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
Validate the workspace YAML files to ensure semantic correctness, referential integrity, and required fields.

#### `tld plan [flags]`
Analyze the workspace and show what changes would be applied to the diag server. By default, it shows a high-level summary and the diagram hierarchy.
**Flags:**
- `-v, --verbose`: Show detailed resource reporting, including all objects per diagram, edges, and links.
- `-o, --output`: Write the plan to a specified file instead of printing to standard output.

#### `tld apply [flags]`
Apply the generated plan or the current workspace state to the diag server.
**Flags:**
- `--auto-approve`: Skip the interactive approval prompt and apply immediately.

---

## Example: Building an E-commerce Platform Architecture

This example walks through building a system context architecture for an e-commerce platform from scratch using the `tld` CLI.

### 1. Initialize the Workspace
First, create a new directory and initialize the `tld` workspace.
```bash
mkdir ecom-arch && cd ecom-arch
tld init .
```

### 2. Create the System Context Diagram
Create the high-level system context diagram.
```bash
tld create diagram "E-commerce Platform" --level-label "System Context" --description "High-level overview of the e-commerce system."
```
*(This generates a diagram with the reference ID: `e-commerce-platform`)*

### 3. Add Objects (Actors and Systems)
Add the primary users and systems to the diagram.
```bash
# Add a Customer actor
tld create object e-commerce-platform "Customer" "person" --description "A customer of the e-commerce platform." --position-x 100 --position-y 300

# Add the main E-commerce Web Application
tld create object e-commerce-platform "Web App" "software_system" --description "The core e-commerce web application." --position-x 400 --position-y 300

# Add an external Payment Gateway
tld create object e-commerce-platform "Payment Gateway" "software_system" --description "External payment processor (e.g., Stripe)." --position-x 700 --position-y 300
```

### 4. Connect the Objects
Define how these systems and actors interact.
```bash
# Customer interacts with the Web App
tld connect objects e-commerce-platform --from "customer" --to "web-app" --label "Browses and buys products" --relationship-type "uses"

# Web App integrates with the Payment Gateway
tld connect objects e-commerce-platform --from "web-app" --to "payment-gateway" --label "Processes payments using" --relationship-type "integrates"
```

### 5. Create a Drill-Down Diagram
Create a more detailed container-level diagram for the Web App.
```bash
tld create diagram "Web App Containers" --level-label "Container" --parent "e-commerce-platform"
```
*(This generates a diagram with the reference ID: `web-app-containers`)*

### 6. Create a Drill-Down Link
Link the "Web App" object on the system context diagram to the detailed container diagram.
```bash
tld create link --from "e-commerce-platform" --object "web-app" --to "web-app-containers"
```


### 7. Validate and Apply
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
