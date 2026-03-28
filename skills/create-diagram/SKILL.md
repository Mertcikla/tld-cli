---
name: create-diagram
description: Create architecture diagrams from a codebase using the tld CLI. Use this skill whenever the user asks to diagram, map, or document their codebase or system architecture — even if they don't mention "tld" or "diagram" explicitly. Trigger on phrases like "map my services", "document my architecture", "create a system diagram", "diagram this repo", "show how my code is structured", or any request to visually represent how a system's components fit together.
allowed-tools: Bash(tld *), Write
---

> **Reference:** When you need exact command syntax, flags, or a full walkthrough example, read `references/tld-docs.md`.

## How tld thinks about maps

In tld, the word "diagram" is overloaded — it means any navigatable view at any level of abstraction. A "diagram" can be your entire system, a single microservice, a Python module, or a single class. Every object on any view can link to its own diagram that zooms into it.

This is not architecture diagramming in the traditional sense. It is a **navigatable atlas of the entire codebase** — like a hyperlinked wiki where you can zoom into anything. At the top you see services talking to each other. Click into a service and you see its modules. Click into a module and you see its classes. Click into a class and you see its `__init__` parameters, its methods, what each method calls, its parent classes, its subclasses, the interfaces it implements.

**The goal at full detail is that a reader can start at the top and follow links until they understand any piece of the system without ever opening a file.**

This means:
- Any significant class, module, function, or type can be an object on a parent view and link to its own diagram
- "Zoom into a class" means: one object per method, edges showing the call graph, `__init__` parameters as objects, inheritance chain linked
- "Zoom into a function" means: parameters, return type, external calls it makes, internal logic flow
- Polymorphic hierarchies are explicit — base class links to a diagram showing all subclasses; each subclass links back and to itself.

---

## The goal: a view that earns its existence

A view is only useful if it tells you something you couldn't immediately see from the file structure. The failure mode to avoid is a **shallow view** — a list of boxes with no edges, or edges with no labels, or components that exist in isolation without showing who talks to whom, who owns what data, or where failures propagate. Boxes are cheap. The edges are the insight.

**Before moving to the next step, ask: would a new engineer reading this view understand how data or control actually flows? If not, add more edges.**

---

## Working surface: diagram.sh

All tld commands go into a single bash script, `diagram.sh`, in the workspace directory. The script is your notes and your execution log — comments explain what you found, commands record what you built.

**Create the script once at the beginning of Step 3:**

```bash
#!/bin/bash
# Architecture diagram build script
set -e
```

**For every batch:**
1. Append a labeled block of commands (with a `# ===` comment header) to `diagram.sh`
2. Run only that new block
3. Do a **batch checkpoint** before continuing to the next batch

The script is the complete, replayable record of every diagram decision. Never run a tld command outside it.

---

## Batch checkpoint

Run this check after every batch of `tld create` or `tld connect` commands:

- **New objects:** Do any just-created objects also appear in existing diagrams? Place and wire them there immediately.
- **New diagrams:** Do any just-created diagrams connect to existing diagrams via drill-down? Add `tld add link` commands.
- **Cross-batch edges:** Are there interactions between objects from this batch and objects from a previous batch that aren't wired yet?

Append any missing links or edges to `diagram.sh` and run them before moving on.

---

## Diagram density: the 10-object rule

Keep each diagram focused. **Aim for ~10 objects per diagram; never exceed 20.** A crowded diagram isn't more informative — it becomes a wall of boxes where the relationships disappear. If you're placing more than 10–15 objects on a single diagram, that's a signal to cluster before you continue.

### When and how to cluster

After mapping your inventory onto a diagram, count the objects you're about to place. If the count is heading above 10–15:

1. **List all the objects** — write them out so you can see the whole set at once.
2. **Find the largest cohesive group** — look for objects that share a role, layer, or domain: all the auth-related pieces, all the data-layer components, all the event processors, etc. The largest such natural grouping is your clustering candidate.
3. **Label the cluster by its role** — give the sub-diagram a name that reflects what those objects *do* at that level of abstraction (e.g., "Auth Services", "Data Layer", "Event Pipeline"), not a generic container name.
4. **Create a sub-diagram for the cluster** — use `tld create diagram` with the parent set to the current diagram. Move the clustered objects there and wire them up inside.
5. **Replace the cluster on the parent with a single representative object** — the parent diagram now has one object for the whole cluster, linked to the sub-diagram. Pick a type and name that reflects the group's role to someone reading the parent level.
6. **Repeat** until every diagram sits at or below 15 objects.

The goal is not to hide complexity — it's to present the right level of detail at each zoom level. A reader should be able to understand the parent diagram in 30 seconds, then drill into any cluster that interests them.

---

## Task Progress

Copy this checklist and check off items as you complete them:

- [ ] Step 1: Explore the codebase
- [ ] Step 2: Produce a subsystem inventory
- [ ] Step 3: Create diagram.sh and root diagrams — batch checkpoint
- [ ] Step 4: Add objects per diagram — batch checkpoint after each diagram
- [ ] Step 4a: Connect objects within each diagram
- [ ] Step 4b: 10-object rule
- [ ] Step 5: Audit shared objects — trace every diagram they appear on
- [ ] Step 6: Drill into every major subsystem (3–5 levels) — batch checkpoint after each sub-diagram
- [ ] Step 7: Run `tld validate`
- [ ] Step 8: Ask user to run `tld plan` and give feedback; iterate if needed

---

## Step 1: Explore the codebase

Before exploring, ask the user two questions:

> 1. **How many diagrams do you expect?** (rough target is fine — e.g. "around 20", "as many as needed", "keep it under 10")
> 2. **How deeply nested do you want the drill-downs?** (e.g. "just top-level", "2–3 levels", "go as deep as possible")

Use their answers to calibrate the rest of the work. If they don't know, suggest options based on the codebase size once you've done a quick directory scan.

**Shallow** ~5–10 views, 1–2 levels deep — Services and their direct dependencies. Entry points, major layers, external systems. Nothing low level.

**Medium** ~10–30 views, 2–3 levels deep — Modules and packages decomposed. Key classes identified and placed. Major data and logic flows wired. Inheritance shown where architecturally significant.

**Detailed** ~50–200+ views, 4–6 levels deep — Every significant class has its own linked view. Each class view shows: `__init__` parameters and their types, all public methods with edges showing which methods call which, all private methods, the full inheritance chain (parent class linked, all known subclasses linked, interfaces/protocols implemented). Every significant function shows its parameters, return type, and external calls. Polymorphic hierarchies are explicit. A reader should be able to navigate from the top-level system view down to understanding a specific method's behavior without opening a file.

### Exploration strategy

Start broad, then narrow:
1. Get the lay of the land — directory tree, top-level file structure
2. Find entry points — `main.go`, `app.tsx`, `index.ts`, `server.py`, etc.
3. Identify major layers — API, services, repositories, workers, frontends
4. Follow imports/dependencies to map how components connect
5. Check config files for external dependencies (DBs, queues, auth providers)

**The output of this step is not a list of files — it's a map of connections.** For each component you identify, note: what does it call? what calls it? what data does it read or write? You need this to produce meaningful edges in Steps 4 and 6.

## Step 2: Produce a subsystem inventory

Before creating a single diagram node, write out an explicit inventory of every major subsystem you found:

```
Subsystem inventory:
- [name]: [one-line description] | calls: [...] | called by: [...] | shared deps: [...]
- [name]: ...
```

Do not proceed to Step 3 until every top-level directory or package is accounted for in this list. If you moved on and later realize you missed a subsystem, go back — a diagram that covers 60% of the codebase is worse than no diagram, because it creates false confidence.

A common failure mode: exploring one interesting area deeply while glossing over the rest. Check that every top-level directory has at least one inventory entry before moving on.

---

## Step 3: Create diagram.sh and root diagrams

Create `diagram.sh`, then append and run the root diagrams as the first batch:

```bash
# === Root diagrams ===
tld create diagram "Domain & Business Logic" --ref domain
tld create diagram "Data & Persistence" --ref data
tld create diagram "Interfaces & Integrations" --ref interfaces
tld create diagram "Platform & Infrastructure" --ref deployment
```

This is the top-level view — major components and how they relate to the outside world, not internal implementation details.

Append and run the next level as a second batch:

```bash
# === Level 2: major subsystems ===
tld create diagram "Backend" --ref backend --parent domain
tld create diagram "Frontend" --ref frontend --parent interfaces
tld create diagram "Storage" --ref storage --parent data

# --- drill-down links ---
tld add link --from domain --to backend
tld add link --from interfaces --to frontend
tld add link --from data --to storage
```

**Batch checkpoint:** Are there additional root-to-root or root-to-subsystem links suggested by your inventory? Add them now.

---

## Step 4: Add objects and wire them together

Work one diagram at a time. For each diagram, append its objects as a batch, run it, then immediately append and run its edges before moving to the next diagram.

Common types: `service`, `database`, `person`, `external_system`, `queue`, `cache`.

```bash
# === Backend objects ===
tld create object backend "REST API" service --technology "Go" --ref api
tld create object backend "Stripe API" external_system --technology "Stripe" --ref stripe
tld create object backend "Job Worker" service --technology "Go" --ref worker
```

**Batch checkpoint after objects:** Do any of these objects appear in other diagrams? Place them there and wire them before continuing to edges.

### Step 4a: Connect everything

Append and run edges for the same diagram immediately after its objects:

```bash
# === Backend edges ===
tld connect objects backend --from api --to stripe --label "billing"
tld connect objects backend --from api --to db --label "reads/writes"
tld connect objects backend --from worker --to queue --label "consumes jobs"
```

Labels should describe *what* the interaction does — "validates JWT", "publishes events" — not just "calls".

An unconnected object is a bug — either it's missing edges, or it doesn't belong on this diagram.

**Batch checkpoint after edges:** Any objects from other diagrams that interact with objects you just wired? Append cross-diagram connections now.

### Step 4b: 10-object rule

Count objects on each diagram. More than ~10? Apply the **clustering strategy** from the Diagram density section before continuing.

---

## Step 5: Audit shared objects

Identify every object appearing on more than one diagram (databases, caches, queues, external APIs). For each one, verify:
- Is it placed on every diagram where it's relevant?
- Does each placement have labeled edges showing exactly how that diagram uses it?

No edges on a shared object = missing information. Append any missing placements and edges to `diagram.sh` and run them.

---

## Step 6: Drill into subsystems

**Depth target: match the user's requested level.** At full detail, every significant class and module has its own linked view and the depth can reach 5–6 levels. Stop only when there is nothing meaningful left to decompose. If you find yourself at only 1–2 levels on a detailed request, you haven't gone deep enough — go back to the code.

Example depth for a typical backend service at full detail:
```
System (L1)
  └── Backend Service (L2)
        └── Auth Module (L3)
              └── AuthService class (L4)
                    └── AuthService: __init__ params, methods, base classes, subclasses (L5)
                          └── validate_token() : params, JWT lib call, DB call, return type (L6)
```

### What to capture in a code-level view

**Class view** (L4–L5): one object per method (public and significant private), one object per `__init__` parameter that is a dependency (not primitives), edges showing which methods call which, edges to parent class view and each known subclass view, edges to any interface/protocol it implements.

**Inheritance view**: a dedicated view for a polymorphic hierarchy — base class at top, all concrete subclasses below, edges labelled with what each override changes. Link each subclass to its own class view.

**Function/method view** (L5–L6): parameters as objects (with types), return type as an object, edges to external calls it makes (other methods, libraries, I/O). Only create this level if the function is complex enough to warrant it — more than ~5 distinct operations.

**Module/package view** (L3–L4): exported symbols as objects, internal dependencies between them wired, external imports shown as external_system objects.

For every subsystem, append one batch per sub-diagram and run it before starting the next:

### 6a. Create the sub-diagram and link it

```bash
# === API (L3) ===
tld create diagram "API" --ref api --parent backend --level-label "Container"
tld add link --object api --from backend --to api
```

### 6b. Populate with internal components

Go back to the code — don't guess. Apply the 10-object rule here too; cluster if needed.

```bash
tld create object api "Auth Middleware" function --technology "Go" --ref auth-mw
tld create object api "User Handler" function --technology "Go" --ref user-handler
tld create object api "Database" database --technology "PostgreSQL" --ref db
```

> If `db` already exists from a parent diagram, `tld create object` adds a new *placement* rather than a duplicate. Reused objects don't inherit edges — add them explicitly for this context.

### 6c. Connect internal objects

```bash
tld connect objects api --from auth-mw --to user-handler --label "forwards request"
tld connect objects api --from user-handler --to db --label "SQL"
```

Before moving to the next subsystem, check every object: at least one incoming edge, at least one outgoing edge, labels specific enough to tell a reader what the interaction does. Missing edges = go back to the code.

**Batch checkpoint:** Do any objects in this sub-diagram appear in sibling diagrams? Are there links between this sub-diagram and any other diagram you've already built? Append and run them now.

Repeat 6a–6c for each remaining subsystem, going 3–5 levels deep per topic.

---

## Step 7: Validate

Append to `diagram.sh` and run:

```bash
# === Validation ===
tld validate
```
Carefully observe its output and its instructions to improve the quality of the diagrams. Don't use scripting to automate this and just to bypass the validation. It needs careful attention and deliberation to improve the diagrams. Work on each issue one by one.

Fix any broken refs or missing fields, append the fixes to `diagram.sh`, and re-run. 

---

## Step 8: Hand off to user

Ask the user to run:

```bash
tld plan
```

Walk them through the output. If they want changes — more objects, deeper drill-downs, better edge labels — append the changes to `diagram.sh`, run the new block, and iterate.
