# tld CLI Reference (Diagram Creation)

`tld` is a CLI for tlDiagram. Define your architecture in YAML, validate it, and sync it to a tlDiagram server.


## The model: elements, views, and connectors

In tld, everything is an **element**. Every element you add automatically gets its own **view** — a navigable canvas that can hold child elements. The hierarchy is defined purely by `--parent` flags: place an element inside another element's view, and the parent becomes navigable. There are no standalone "diagrams" to create separately.

**Connectors** link two elements within the same view. The view is inferred from the elements' shared parent, so you never specify it manually.

**Roles** (service, database, external system, person, etc.) are expressed as tags on an element — not as distinct types.

### Workspace file layout

```
~/.config/tldiagram/tld.yaml    # Global config: API key, org
.tld/
  ├── .tld.yaml                 # Workspace config: project metadata
  ├── elements.yaml             # Elements + placements
  ├── connectors.yaml           # Connectors between elements
  └── .tld.lock                 # Sync state at last apply
```


## Commands

### `tld add <name> [flags]`
Create or update an element in `elements.yaml`. **Idempotent:** if an element with the same ref already exists, running `add` again merges the update or adds a new placement.

Every element automatically gets a view (`has_view: true`), so any element can be drilled into.

**Flags:**
- `--ref <slug>` - Override the auto-generated ref (default: slugified name, e.g. `"My API"` → `my-api`)
- `--parent <ref>` - Place this element in the named element's view (default: `root`)
- `--kind <kind>` - Element kind (e.g. `service`, `database`, `function`)
- `--description "..."` - What this element does
- `--technology "..."` - Primary technology (e.g. `Go`, `React`, `PostgreSQL`)
- `--url "..."` - Link to docs, repo, or dashboard
- `--tag <tag>` - Semantic tag to apply (can be used multiple times)

```bash
# Root-level elements (shown in the top-level canvas)
tld add "Frontend" --technology "React" --tag "role:frontend"
tld add "API Gateway" --technology "Go" --tag "role:gateway"
tld add "Database" --technology "PostgreSQL" --tag "role:database"

# Child elements (shown inside the parent's view when you drill down)
tld add "Auth Middleware" --parent api-gateway --technology "Go"
tld add "User Handler" --parent api-gateway --technology "Go"
```

---

### `tld connect <source> <target> [flags]`
Add a connector between two elements. The view is **automatically inferred** from the elements' shared parent if omitted.

**Arguments:**
- `<source>` **(required)** - Source element ref
- `<target>` **(required)** - Target element ref

**Flags:**
- `--view <ref>` - Parent view ref for the connector (inferred if missing)
- `--label "..."` - Short label shown on the connector (e.g. `"calls"`, `"reads"`)
- `--relationship "..."` - Semantic type (e.g. `uses`, `depends_on`, `integrates`)
- `--direction` - `forward` | `backward` | `both` | `none` (default: `forward`)

```bash
# Connectors between root-level elements
tld connect frontend api-gateway --label "HTTP"
tld connect api-gateway database --label "SQL"

# Connectors within a drill-down view (view inferred from shared parent)
tld connect auth-middleware user-handler --label "forwards request"
tld connect user-handler database --label "SQL"
```

---

### `tld remove element <ref>`
Remove an element (and all its placements) from `elements.yaml`.

```bash
tld remove element old-service
```

---

### `tld remove connector <source> <target> [flags]`
Remove a specific connector from `connectors.yaml`.

**Flags:**
- `--view <ref>` - View ref where the connector exists (inferred if missing)

---

### `tld update <ref> <field> <value>`
Update a specific field of an element.

```bash
tld update my-api technology "Rust"
```

---

### `tld validate`
Validate workspace YAML for referential integrity, required fields, and cycles.

```bash
tld validate
```

---

## Slugify rules

Names are automatically converted to refs: lowercase, non-alphanumeric characters replaced with hyphens, leading/trailing hyphens trimmed.

Examples: `"My API"` → `my-api`, `"Auth Controller"` → `auth-controller`

---

## Full walkthrough example

```bash
# 1. Init workspace
mkdir my-arch && cd my-arch
tld init .

# 2. Add root-level elements (appear in the top-level "root" canvas)
tld add "Customer" --ref customer
tld add "Web App" --ref web-app --technology "React"
tld add "Payment Gateway" --ref payment-gateway --technology "Stripe"

# 3. Connect them (view inferred automatically as "root")
tld connect customer web-app --label "browses and buys"
tld connect web-app payment-gateway --label "processes payments"

# 4. Drill down into Web App (add children — view inferred as "web-app")
tld add "React Frontend" --parent web-app --technology "React"
tld add "GraphQL API" --parent web-app --technology "Go"
tld add "Postgres DB" --parent web-app --technology "PostgreSQL"

# 5. Connect inside the Web App view
tld connect react-frontend graphql-api --label "queries"
tld connect graphql-api postgres-db --label "SQL"

# 6. Validate and apply
tld validate
tld plan
tld apply
```
