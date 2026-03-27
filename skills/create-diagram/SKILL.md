---
name: create-diagram
description: Create C4-style architecture diagrams from a codebase using the tld CLI. Use this skill whenever the user asks to diagram, map, or document their codebase or system architecture — even if they don't mention "tld" or "diagram" explicitly. Trigger on phrases like "map my services", "document my architecture", "create a system diagram", "diagram this repo", "show how my code is structured", or any request to visually represent how a system's components fit together.
allowed-tools: Bash(tld *)
---

> **Reference:** When you need exact command syntax, flags, or a full walkthrough example, read `references/tld-docs.md`.

## The goal: a diagram that earns its existence

A diagram is only useful if it tells you something you couldn't immediately see from the file structure. The failure mode to avoid is a **shallow diagram** — a list of boxes with no edges, or edges with no labels, or components that exist in isolation without showing who talks to whom, who owns what data, or where failures propagate. Boxes are cheap. The edges are the insight.

**Before moving to the next step, ask: would a new engineer reading this diagram understand how data or control actually flows? If not, add more edges.**

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

- [ ] Step 1: Explore — produce a complete subsystem inventory before touching the CLI
- [ ] Step 2: Scale check — validate your inventory covers the full codebase
- [ ] Step 3: Create the root "System Context" diagram
- [ ] Step 4: Add objects and wire them together with labeled edges
- [ ] Step 4b: Density check — if any diagram exceeds ~10 objects, cluster before continuing
- [ ] Step 5: Audit shared objects — trace every diagram they appear on
- [ ] Step 6: For each major subsystem: create a sub-diagram, link it, populate it, connect it
- [ ] Step 7: Run `tld validate`
- [ ] Step 8: Ask user to run `tld plan` and give feedback; iterate if needed

---

## Step 1: Explore the codebase

Ask the user which thoroughness level they want, then explore accordingly.

**Quick** (~30 sec) — File structure, entry points, directory layout
**Medium** (~2-3 min) — Cross-file patterns, architectural mapping, key modules
**Very Thorough** — Exhaustive. Every top-level directory visited. Every major package examined. Dependency chains traced. Don't stop until you've seen the whole surface area.

### Exploration strategy

Start broad, then narrow:
1. Get the lay of the land — directory tree, top-level file structure
2. Find entry points — `main.go`, `app.tsx`, `index.ts`, `server.py`, etc.
3. Identify major layers — API, services, repositories, workers, frontends
4. Follow imports/dependencies to map how components connect
5. Check config files for external dependencies (DBs, queues, auth providers)

**The output of this step is not a list of files — it's a map of connections.** For each component you identify, note: what does it call? what calls it? what data does it read or write? You need this to produce meaningful edges in Steps 4 and 6.

### Produce a subsystem inventory before moving on

Before creating a single diagram node, write out an explicit inventory of every major subsystem you found:

```
Subsystem inventory:
- [name]: [one-line description] | calls: [...] | called by: [...] | shared deps: [...]
- [name]: ...
```

Do not proceed to Step 2 until every top-level directory or package is accounted for in this list. If you moved on and later realize you missed a subsystem, go back — a diagram that covers 60% of the codebase is worse than no diagram, because it creates false confidence.

---

## Step 2: Scale check — validate your inventory before creating anything

Look at your inventory and calibrate expectations against the size of the codebase:

| Codebase size | Expected total objects | Expected sub-diagrams |
|---|---|---|
| Small (<50 files) | 5–20 | 3–5 |
| Medium (50–500 files) | 20–50 | 5–15 |
| Large (500+ files) | 50–100 | 15–25 |
| Very large (monorepo / framework scale) | 100+ | 25-50+ |

If your inventory item count is well below the left column for your codebase size, you haven't explored enough. Go back to Step 1 and keep digging — look specifically at directories you skimmed, packages you assumed were unimportant, and build/config files that reveal hidden dependencies.

A common failure mode: exploring one interesting area deeply while glossing over the rest. Check that every top-level directory has at least one inventory entry before moving on.

---

## Step 3: Create the root "System Context" diagram

```bash
tld create diagram "System Context" --ref system-context --level-label "System"
```

This is the top-level view — major components and how they relate to the outside world, not internal implementation details.

---

## Step 4: Add objects and wire them together

Map every subsystem from your inventory onto the diagram. Every inventory item should become at least one object — if something was worth listing, it's worth showing.

```bash
tld create object system-context "REST API" service --technology "Go" --ref api
tld create object system-context "Web App" service --technology "React" --ref web
tld create object system-context "Database" database --technology "PostgreSQL" --ref db
tld create object system-context "Auth Provider" external_system --technology "OAuth2" --ref auth
```

Common types: `service`, `database`, `person`, `external_system`, `queue`, `cache`

**Then connect everything you discovered in Step 1.** Don't add an object without asking: who calls this? what does it call? Every object that isn't connected to anything is a red flag — either it's missing edges, or it shouldn't be on this diagram at all.

```bash
tld connect objects system-context --from web --to api --label "HTTP requests"
tld connect objects system-context --from api --to db --label "reads/writes"
tld connect objects system-context --from api --to auth --label "validates token"
```

Use `--label` to say *what* the interaction is, not just that it exists. "calls" is weak. "validates JWT", "publishes events", "reads user profile" are useful.

### Step 4b: Density check — cluster if needed

Count the objects on the diagram you just built. If you have more than ~10, stop here and apply the clustering strategy from the **Diagram density** section above before moving on. Do not proceed to Step 5 with an overfull diagram — the sub-diagrams you create in Step 6 will inherit the same problem and compound it.

A quick heuristic: if you can't read the diagram aloud in under a minute without losing track of what connects to what, it's too dense.

---

## Step 5: Audit shared objects before drilling down

Before creating any sub-diagrams, identify every object that will appear on more than one diagram — typically infrastructure shared across services: databases, caches, message queues, auth providers, external APIs.

For each shared object, answer:
- Which diagrams will it appear on?
- What does it *do differently* in each context? (reads vs. writes, different tables, different queues)
- Are all of those interactions already represented as edges?

**This is the most common source of shallow diagrams.** A database placed on five diagrams with no edges on any of them tells you nothing. A database on five diagrams, each with labeled edges showing exactly which service queries which data, tells the whole story. Do not place a shared object on a diagram without immediately wiring it up.

---

## Step 6: Drill into every major subsystem — this is a loop, not a one-shot

From your subsystem inventory, identify every component that warrants a closer look — the ones where a reader would ask "but how does that actually work inside?" For a non-trivial codebase this should be multiple subsystems, not one. Create a sub-diagram for each.

**For each subsystem, repeat this sequence:**

### 6a. Create the sub-diagram and link it

```bash
tld create diagram "API Internals" --ref api-internals --parent system-context --level-label "Container"
tld add link --object api --from system-context --to api-internals
```

The link means: clicking the anchor object in the UI drills down into this diagram.

### 6b. Populate with internal components

Add the internal building blocks. Go back to the code for this — don't guess the internals. **Keep the 10-object rule in mind here too** — if a subsystem has many internal components, cluster the largest cohesive group into a child sub-diagram rather than listing everything flat. A sub-diagram with 25 components is just as unreadable as a root diagram with 25.

```bash
tld create object api-internals "Auth Controller" service --technology "Go" --ref auth-controller
tld create object api-internals "User Service" service --technology "Go" --ref user-service
tld create object api-internals "User Repo" service --technology "Go / pgx" --ref user-repo
tld create object api-internals "Database" database --technology "PostgreSQL" --ref db
```

**Object reuse:** If `db` already exists from a parent diagram, `tld create object` adds a new *placement* on this diagram instead of creating a duplicate. The same object on two diagrams is correct — but a reused object inherits its identity, not its connections. Add the edges for this specific context explicitly.

### 6c. Connect internal objects

Show how data or control flows. This is where most of the value lives.

```bash
tld connect objects api-internals --from auth-controller --to user-service --label "calls"
tld connect objects api-internals --from user-service --to user-repo --label "queries"
tld connect objects api-internals --from user-repo --to db --label "SQL"
```

Before moving on to the next subsystem, do a pass over every object on this diagram:
- Does it have at least one incoming edge? If not — what triggers it?
- Does it have at least one outgoing edge? If not — what does it affect?
- Are the labels specific enough that a reader knows what the interaction does?

If an object has no edges at all, it's almost certainly missing connections. Go back to the code and find them.

**Then repeat 6a–6c for the next subsystem in your list.**

---

## Step 7: Validate

```bash
tld validate
```

Fix any broken refs or missing fields before handing off to the user.

---

## Step 8: Hand off to user

Ask the user to run:

```bash
tld plan
```

Walk them through the output. If they want changes — more objects, deeper drill-downs, better edge labels — go back to the relevant step and iterate.
