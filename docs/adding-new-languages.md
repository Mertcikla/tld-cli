# Adding Language Support To `tld analyze`

This document traces `tld analyze` from the CLI command down to the YAML writers
and turns the current implementation into a reusable, language-agnostic process.

The goal is not to copy the current Go-specific parser behavior into every new
language. The goal is to understand the shared pipeline and feed it the normalized
data it already expects.

---

## Why this guide exists

`tld analyze` does two distinct jobs:

1. Parse source files into a normalized set of declarations and references.
2. Translate that normalized data into workspace elements, connectors, and views.

When adding a new language, most of the work should stay in step 1. The command,
deduping, connector planning, and YAML persistence layers are already shared.

---

## End-to-end flow (Rust implementation)

The entry point is `src/cli/analyze.rs`.

### 1. Resolve the analysis scope

The command starts by:

- Resolving the input path to an absolute path with `Path::canonicalize`.
- Loading the workspace with `workspace::load`.
- Building ignore `Rules` from workspace config.

This matters for new languages because repo-relative file paths are used as stable
identities in `elements.yaml`.

### 2. Build the scan plan

For each file the walker visits, it checks `Rules::should_ignore_path` before
dispatching to the parser. This step is language-neutral — a new language should
rely on the same ignore model and not add its own exclusion logic.

### 3. Extract normalized analyzer results

The service path is:

- `src/analyzer/mod.rs`: `TreeSitterService` implements `Service`.
- `src/analyzer/parsers/`: one file per language.

Language detection uses `tree_sitter_language_pack::detect_language_from_path`
(covers 300+ extensions automatically — no manual mapping needed).

The grammar is loaded with `tree_sitter_language_pack::get_language(lang_name)`.
With the default `download` feature enabled, the grammar is auto-downloaded on
first use and cached in the system cache directory.

`is_parser_implemented(lang_name)` acts as an explicit gate: a language can be
recognised and downloaded without tld having an AST-walk for it yet.

Each parser only needs to return:

- `Vec<Symbol>`
- `Vec<Ref>`

### 4. Error handling for missing parsers

Three distinct error variants cover the three failure modes:

| Variant | When | User-visible message |
|---|---|---|
| `TldError::UnsupportedLanguage(ext)` | Extension not in language pack | "Unsupported file type: .xyz" |
| `TldError::ParserDownloadRequired { lang, reason }` | Grammar not in local cache | Prompts to run `--download` or downloads interactively |
| `TldError::ParserNotImplemented(lang)` | Grammar loaded but no AST-walk | Directs user to this guide |

During directory walks, `UnsupportedLanguage` and `ParserNotImplemented` are silently
skipped so a mixed-language repository only processes the languages tld knows about.
`ParserDownloadRequired` is surfaced immediately because it requires user action.

---

## The parser contract

The stable extension seam is the `parse` function in each
`src/analyzer/parsers/<lang>.rs` file:

```rust
pub fn parse(node: &Node, source: &[u8], path: &str, result: &mut AnalysisResult)
```

Everything that follows depends on the quality of `Symbol` and `Ref` records
returned through `result`.

### `Symbol` fields and why they matter

`src/analyzer/types.rs` defines the normalized declaration shape.

**`name`** — simple declaration name, not package-qualified unless the language
requires it.

**`kind`** — stable cross-language categories: `function`, `method`, `struct`,
`enum`, `trait`, `interface`, `type`, `class`, etc. Avoid leaking raw AST node
names.

**`file_path`** — the real path given to the parser. The CLI later normalizes it
to a repo-relative path.

**`line` / `end_line`** — `line` anchors the declaration start (name node).
`end_line` should cover the full declaration body. Both are required for
source-to-symbol attribution during reference resolution.

**`parent`** — immediate lexical owner for nested declarations (e.g. the struct
name for a method inside an `impl` block). Enables child-under-parent placement in
views.

**`description`** — extracted doc comment text, stripped of comment delimiters.

### `Ref` fields and why they matter

**`name`** — simple referenced name, not package-qualified. For chained calls, use
the terminal callable name.

**`kind`** — `"call"` for call-like references; `"import"` for import-like
dependencies that should generate file/folder dependency edges.

**`target_path`** — for import-like refs, the module path translated into a
repo-relative directory equivalent (e.g. `"crate/analyzer/parsers"` for
`use crate::analyzer::parsers`). Enables aggregate folder-level dependency edges
even when no exact declaration target is available.

**`file_path`**, **`line`**, **`column`** — locate the reference site. Accurate
columns improve LSP-based definition lookup.

---

## Rust-specific findings

Added in: `src/analyzer/parsers/rust.rs`.

### Node kinds used

| Declaration | tree-sitter node |
|---|---|
| Free function | `function_item` |
| Method (in impl) | `function_item` inside `impl_item` |
| Struct | `struct_item` |
| Enum | `enum_item` |
| Trait | `trait_item` |
| Type alias | `type_item` |
| Use statement | `use_declaration` |
| Free function call | `call_expression` → `function` field |
| Method call | `method_call_expression` → `method` field |

### Parent detection for methods

Rust methods live inside `impl_item` nodes. The walker detects an `impl_item`,
extracts the implementing type name from its `type` field, and passes it down as
`impl_parent`. Functions encountered while `impl_parent` is `Some` are emitted with
`kind = "method"` and `parent = <type name>`.

`impl_type_name` handles the common cases:
- `impl Foo` → `"Foo"` (plain `type_identifier`)
- `impl<T> Bar<T>` → `"Bar"` (first `type_identifier` inside `generic_type`)
- `impl foo::Bar` → `"Bar"` (last `type_identifier` in `scoped_type_identifier`)

### Call-site isolation

To avoid emitting duplicate call refs when functions are nested, the walker uses a
two-phase approach:

1. When it encounters a `function_item`, it registers the symbol and then calls
   `walk_calls_only` on the function body only.
2. `walk_calls_only` collects `call_expression` and `method_call_expression` nodes
   but does not descend into `closure_expression` bodies to avoid double-counting.
3. The main `walk_node` returns early after `function_item` so the recursive descent
   doesn't double-visit the body.

### Import handling

`use_declaration` → `argument` field is walked recursively to collect all leaf
import targets from use lists and use trees. Each leaf becomes a `Ref` with:

- `kind = "import"`
- `target_path` set to the `::`-path with `::` replaced by `/` (making it a
  pseudo-directory path for aggregation)
- `name` set to the last path segment

`use_wildcard` nodes (`use foo::*`) emit the module prefix as the import target.

### Call name normalization

`rust_call_name` extracts the terminal callable name:

- `identifier` → returned as-is
- `field_expression` → `field` child (e.g. `obj.method` → `"method"`)
- `scoped_identifier` → `name` field, or last `::` segment (e.g. `Foo::bar` →
  `"bar"`)
- `generic_function` → recurse into `function` field (`foo::<T>()` → `"foo"`)

### Doc comment extraction

`find_doc_comment` checks the `prev_named_sibling` of a declaration node. If it is
a `line_comment`, `block_comment`, or `doc_comment` immediately adjacent (≤ 1 row
gap), the comment text is returned stripped of `///`, `//!`, `//`, `/**`, `/*`,
and `*/` markers.

---

## What to implement for a new language

### Minimum implementation

1. Add a `parse` function in `src/analyzer/parsers/<lang>.rs` that populates
   `AnalysisResult` with `Symbol` and `Ref` records.
2. Export the module in `src/analyzer/parsers/mod.rs`.
3. Add `lang_name` to the `is_parser_implemented` match in `src/analyzer/mod.rs`.
4. Add the dispatch arm to the `match lang_name` block in `extract_file`.
5. Add unit tests in the parser module.

`detect_language_from_path` already handles extension → language name mapping for
300+ languages, so no changes to the extension mapping are needed unless the
language uses a non-standard extension.

### Recommended parser behavior

- Emit declaration spans (`line` and `end_line`), not just start lines.
- Emit `parent` names for nested declarations (methods under classes/impls, etc.).
- Emit identifier-start columns for references.
- Emit import-like refs separately from call-like refs (`kind = "import"`).
- Normalize member-call names to the terminal callable name.
- Prefer normalized semantic kinds over raw AST node names.

### Language-specific hook to think about early

Most popular languages have some notion of external or cross-file dependency target:
imports, package references, namespace references, includes, module paths, use
statements. If the parser can recover that target, put it in `Ref.target_path` and
mark the ref as `"import"`. The connector planner uses this to build aggregate
directory-level dependency edges.

---

## Deduping strategies already built into the command

New language support should reuse these strategies instead of replacing them.

### 1. Identity-based element reuse

Element identity is based on ref slug derived from the symbol name. If a symbol
with the same slug already exists in `elements.yaml`, it is updated in place rather
than duplicated.

### 2. Stable ref generation

`workspace::slugify` converts a symbol name into a stable URL-safe key. This keeps
refs deterministic across repeated analysis passes.

### 3. Global display-name uniqueness

Because slugs are derived from names, two symbols with the same name in different
files produce the same slug. When that happens the second upsert overwrites the
first. Future work: incorporate file path into the identity key to support
same-named symbols across files.

### 4. Silent skip for unsupported files

During directory walks, files with unsupported extensions and files in languages
with no implemented parser are silently skipped. Only `ParserDownloadRequired`
errors are surfaced immediately.

---

## Current limitations worth keeping in mind

### Parent matching is name-based

Methods are attached to parents by the impl type name string. If two impl blocks in
different files share the same type name, the parent attribution is still correct
because each method is anchored by its own `file_path` and `line`.

### Same-name symbols across files

Two functions named `new` in different files both slug to `"new"` and the second
upsert wins. This is a known limitation of the current slug-based identity model.
Incorporating `file_path` into the identity key would fix this but requires a
workspace schema change.

### Files without kept symbols

The current graph model is declaration-driven. Files that produce no retained
symbols do not become standalone file elements.

### Import aggregation is language-specific

The connector planner expects repo-relative dependency targets. The Rust parser
translates `use crate::foo::bar` → `target_path = "crate/foo/bar"`. Other
languages will need their own translation of module/import paths into repo-relative
directory equivalents.

---

## The core rule

When adding language support, do not reimplement the analyze command.

Instead:

- Parse the language into the shared `Symbol` and `Ref` model.
- Preserve enough positional information for later resolution.
- Let the existing command own dedupe, graph projection, connector planning, and
  YAML persistence.

If a new language needs extra behavior, add it as a narrow normalization hook near
the parser or reference-resolution boundary, not as a separate command path.
