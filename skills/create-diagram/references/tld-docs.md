# tld CLI Reference (Diagram Creation)

`tld` is a CLI for tlDiagram. Define your architecture in YAML, validate it, and sync it to a tlDiagram server.

---

## Global Options

- `-w, --workspace` — Workspace directory. Defaults to `.`.

---

## Workspace Structure

```
~/.config/tldiagram/tld.yaml       # Global Config (shared across workspaces)
.tld.lock       # Auto-generated after first apply; tracks versions and drift
diagrams.yaml   # Diagram definitions
objects.yaml    # Object definitions and placements
edges.yaml      # Edges (relationships) between objects
links.yaml      # Drill-down navigation links between diagrams
```

### `~/.config/tldiagram/tld.yaml` fields

```yaml
server_url: https://tldiagram.com
api_key: <your-key>   # or set TLD_API_KEY env var
org_id: <uuid>
```

---

## Commands

### `tld init [dir]`
Initialize a new workspace. Ensures global configuration exists at `~/.config/tldiagram/tld.yaml`.

```bash
tld init .
tld init my-arch
```

---

### `tld login`
Authenticate with the tlDiagram server via device authorization flow.

**Flags:**
- `--server` — Server URL (default: `$TLD_SERVER_URL` or `https://tldiagram.com`)
- `--no-browser` — Print the auth URL instead of opening a browser

---

### `tld create diagram <name> [flags]`
Create a new diagram entry in `diagrams.yaml`.

**Flags:**
- `--ref <slug>` — Override the auto-generated ref (default: slugified name, e.g. `"My API"` → `my-api`)
- `--description "..."` — Shown in the diagram panel
- `--level-label "..."` — Abstraction level label (e.g. `System`, `Container`, `Component`)
- `--parent <diagram_ref>` — Makes this a child diagram; used for hierarchy and topological apply ordering

```bash
tld create diagram "System Context" --ref system-context --level-label "System"
tld create diagram "API Internals" --ref api-internals --parent system-context --level-label "Container"
```

---

### `tld create object <diagram_ref> <name> <type> [flags]`
Create a new object and place it on a diagram. **Idempotent**: if an object with the same ref already exists, it adds a new placement on the target diagram instead of creating a duplicate.

**Common types:** `service`, `database`, `person`, `external_system`, `queue`, `cache`

**Flags:**
- `--ref <slug>` — Override auto-generated ref
- `--description "..."` — What this object does
- `--technology "..."` — Primary technology (e.g. `Go`, `React`, `PostgreSQL`)
- `--url "..."` — Link to docs, repo, or dashboard
- `--position-x <float>` — Horizontal canvas position
- `--position-y <float>` — Vertical canvas position

> **Note:** `logo_url` is supported in `objects.yaml` directly but has no CLI flag. Set it manually in YAML if needed.

```bash
tld create object system-context "REST API" service --technology "Go" --ref api
tld create object system-context "Database" database --technology "PostgreSQL" --ref db
tld create object system-context "Web App" service --technology "React" --ref web
tld create object system-context "Auth Provider" external_system --technology "OAuth2" --ref auth
```

---

### `tld connect objects <diagram_ref> [flags]`
Add a directed edge between two objects on a diagram.

**Flags:**
- `--from <ref>` **(required)** — Source object ref
- `--to <ref>` **(required)** — Target object ref
- `--label "..."` — Short label shown on the edge (e.g. `"calls"`, `"reads"`)
- `--description "..."` — Longer explanation shown in the edge panel
- `--relationship-type "..."` — Semantic type (e.g. `uses`, `depends_on`, `integrates`)
- `--direction` — `forward` | `backward` | `both` | `none` (default: `forward`)
- `--edge-type` — `bezier` | `straight` | `step` | `smoothstep` (default: `bezier`)
- `--url "..."` — Link to relevant docs or API spec

```bash
tld connect objects api-internals --from auth-controller --to user-service --label "calls"
tld connect objects api-internals --from user-service --to user-repo --label "queries"
tld connect objects api-internals --from user-repo --to db --label "SQL"
```

---

### `tld add link [flags]`
Create a drill-down navigation link. Clicking the anchor object in the UI navigates to the target diagram.

**Flags:**
- `--from <diagram_ref>` **(required)** — Source diagram
- `--to <diagram_ref>` **(required)** — Target diagram to drill down into
- `--object <ref>` — Object on the source diagram that acts as the drill-down anchor

```bash
tld add link --object api --from system-context --to api-internals
```

---

### `tld validate`
Validate workspace YAML for referential integrity, required fields, and parent diagram cycles.

```bash
tld validate
```

---

### `tld plan [-o file]`
Show a markdown preview of what would be applied. Performs a server-side dry run to detect conflicts and drift.

```bash
tld plan
tld plan -o plan.md
```

---

### `tld apply [--auto-approve]`
Atomically sync the local workspace to the server. Prompts for confirmation unless `--auto-approve` is passed.

If conflicts are detected (resources modified on the server since last sync), you'll be prompted to abort or force-apply.

**Flags:**
- `--auto-approve` — Skip the confirmation prompt
- `--debug` — Log detailed network request/response info

```bash
tld apply
tld apply --auto-approve
```

---

## Slugify Rules

Names are automatically converted to refs: lowercase, non-alphanumeric characters replaced with hyphens, leading/trailing hyphens trimmed.

Examples: `"My API"` → `my-api`, `"Auth Controller"` → `auth-controller`

---

## Full Walkthrough Example

```bash
# 1. Init workspace
mkdir my-arch && cd my-arch
tld init .

# 2. Create top-level diagram
tld create diagram "System Context" --ref system-context --level-label "System"

# 3. Add actors and systems
tld create object system-context "Customer" person
tld create object system-context "Web App" service --technology "React" --ref web-app
tld create object system-context "Payment Gateway" external_system --ref payment-gateway

# 4. Connect them
tld connect objects system-context --from customer --to web-app --label "Browses and buys"
tld connect objects system-context --from web-app --to payment-gateway --label "Processes payments"

# 5. Drill down into Web App
tld create diagram "Web App Containers" --ref web-app-containers --parent system-context --level-label "Container"
tld add link --from system-context --object web-app --to web-app-containers

# 6. Validate and apply
tld validate
tld plan
tld apply
```
