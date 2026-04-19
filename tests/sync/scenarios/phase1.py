"""Phase 1 scenarios: apply/pull round-trip seams.

Each scenario takes (ws: Path, prefix: str, artifacts_dir: Path) and
raises on failure. All DB queries are org+prefix scoped.

Conventions:
- `ws` is a fresh tempdir. Every scenario starts by running `tld init`
  inside it (which creates ws/.tld/). Subsequent commands use `-w <ws>/.tld`.
- Element refs are derived from the per-scenario prefix so every row in
  the DB can be matched on ref LIKE prefix||'%'.
"""

from __future__ import annotations

import concurrent.futures
import hashlib
import threading
import time
from pathlib import Path

import helpers
from helpers import (
    ORG_ID,
    cleanup,
    db_audit_since,
    db_connectors,
    db_dupes_connectors,
    db_dupes_elements,
    db_elements,
    db_exec,
    db_now,
    db_query,
    run_tld,
)


# --- common setup ---------------------------------------------------------

def _init(ws: Path) -> Path:
    run_tld(["init"], cwd=ws)
    return ws / ".tld"


def _add(ws: Path, ref: str, name: str, **kw) -> None:
    args = ["-w", str(ws / ".tld"), "add", name, "--ref", ref]
    for k, v in kw.items():
        args += [f"--{k.replace('_', '-')}", v]
    run_tld(args, cwd=ws)


def _connect(ws: Path, src: str, tgt: str, *, label: str | None = None, view: str | None = None) -> None:
    args = ["-w", str(ws / ".tld"), "connect", src, tgt]
    if label:
        args += ["--label", label]
    if view:
        args += ["--view", view]
    run_tld(args, cwd=ws)


def _apply(ws: Path, *extra: str) -> helpers.ProcResult:
    return run_tld(["-w", str(ws / ".tld"), "apply", "--force", *extra], cwd=ws)


def _pull(ws: Path, *extra: str) -> helpers.ProcResult:
    return run_tld(["-w", str(ws / ".tld"), "pull", "--force", *extra], cwd=ws)


def _assert(cond: bool, msg: str) -> None:
    if not cond:
        raise AssertionError(msg)


def _sha(p: Path) -> str:
    return hashlib.sha256(p.read_bytes()).hexdigest()


# --- scenarios ------------------------------------------------------------

def apply_idempotence(ws: Path, prefix: str, art: Path) -> None:
    _init(ws)
    _add(ws, f"{prefix}-a", f"A {prefix}", kind="service")
    _add(ws, f"{prefix}-b", f"B {prefix}", kind="database")
    _connect(ws, f"{prefix}-a", f"{prefix}-b", label="reads")

    run_tld(["-w", str(ws / ".tld"), "validate"], cwd=ws)
    run_tld(["-w", str(ws / ".tld"), "plan"], cwd=ws)

    t0 = db_now()
    _apply(ws)
    els1 = db_elements(prefix)
    cons1 = db_connectors(prefix)
    _assert(len(els1) == 2, f"expected 2 elements, got {len(els1)}: {els1}")
    _assert(len(cons1) == 1, f"expected 1 connector, got {len(cons1)}: {cons1}")
    _assert(cons1[0]["source_ref"] == f"{prefix}-a", f"bad source: {cons1}")
    _assert(cons1[0]["target_ref"] == f"{prefix}-b", f"bad target: {cons1}")
    _assert(cons1[0]["label"] == "reads", f"bad label: {cons1}")

    # Second apply: IDs must not change; duplicate checks must be empty.
    _apply(ws)
    els2 = db_elements(prefix)
    cons2 = db_connectors(prefix)
    _assert({e["id"] for e in els1} == {e["id"] for e in els2},
            f"element IDs changed on re-apply: {els1} vs {els2}")
    _assert({c["id"] for c in cons1} == {c["id"] for c in cons2},
            f"connector IDs changed on re-apply: {cons1} vs {cons2}")
    _assert(db_dupes_elements(prefix) == [], f"duplicate elements after re-apply")
    _assert(db_dupes_connectors(prefix) == [], f"duplicate connectors after re-apply")


def apply_without_lockfile(ws: Path, prefix: str, art: Path) -> None:
    tld_dir = _init(ws)
    _add(ws, f"{prefix}-x", f"X {prefix}", kind="service")
    _apply(ws)
    # Nuke lock and re-apply.
    lock = tld_dir / ".tld.lock"
    if lock.exists():
        lock.unlink()
    _apply(ws)
    els = db_elements(prefix)
    _assert(len(els) == 1, f"expected 1 element, got {len(els)}")
    _assert(db_dupes_elements(prefix) == [], "duplicate after lockless apply")


def pull_into_empty_workspace(ws: Path, prefix: str, art: Path) -> None:
    # Seed: apply from ws1; pull into ws2 (simulated by two subdirs).
    ws1 = ws / "src"
    ws2 = ws / "dst"
    ws1.mkdir()
    ws2.mkdir()
    _init(ws1)
    _add(ws1, f"{prefix}-root", f"Root {prefix}", kind="service")
    _add(ws1, f"{prefix}-db", f"DB {prefix}", kind="database")
    _connect(ws1, f"{prefix}-root", f"{prefix}-db", label="writes")
    _apply(ws1)

    _init(ws2)
    _pull(ws2)
    elems = (ws2 / ".tld" / "elements.yaml").read_text()
    cons = (ws2 / ".tld" / "connectors.yaml").read_text()
    # Strict check: yaml keys must exactly equal the server's ref, not
    # slugify(name). (Catches pull regenerating refs from name.)
    _assert(f"\n{prefix}-root:\n" in ("\n" + elems) or elems.startswith(f"{prefix}-root:\n"),
            f"ref '{prefix}-root' is not the YAML key for its element:\n{elems}")
    _assert(f"\n{prefix}-db:\n" in ("\n" + elems),
            f"ref '{prefix}-db' is not the YAML key for its element:\n{elems}")
    _assert(f"{prefix}-root" in cons and f"{prefix}-db" in cons,
            f"connector missing from pulled connectors.yaml:\n{cons}")

    # diff should be clean.
    d = run_tld(["-w", str(ws2 / ".tld"), "diff"], cwd=ws2, expect_rc=None)
    _assert(d.rc == 0, f"diff non-zero after pull: {d}")


def pull_twice_byte_identical(ws: Path, prefix: str, art: Path) -> None:
    # Seed server.
    ws1 = ws / "src"
    ws2 = ws / "dst"
    ws1.mkdir(); ws2.mkdir()
    _init(ws1)
    _add(ws1, f"{prefix}-s1", f"S1 {prefix}", kind="service")
    _connect_self_skip = None  # no connector needed
    _apply(ws1)

    _init(ws2)
    _pull(ws2)
    e1 = _sha(ws2 / ".tld" / "elements.yaml")
    c1 = _sha(ws2 / ".tld" / "connectors.yaml")
    _pull(ws2)
    e2 = _sha(ws2 / ".tld" / "elements.yaml")
    c2 = _sha(ws2 / ".tld" / "connectors.yaml")
    _assert(e1 == e2, f"elements.yaml hash changed on second pull: {e1} vs {e2}")
    _assert(c1 == c2, f"connectors.yaml hash changed on second pull: {c1} vs {c2}")


def pull_preserves_local_additions(ws: Path, prefix: str, art: Path) -> None:
    _init(ws)
    _add(ws, f"{prefix}-local-only", f"LocalOnly {prefix}", kind="service")
    # Server has NOTHING for this prefix. Pull should not erase local addition.
    _pull(ws)
    elems = (ws / ".tld" / "elements.yaml").read_text()
    _assert(f"{prefix}-local-only" in elems,
            f"local-only element erased by pull:\n{elems}")


def pull_applies_server_additions(ws: Path, prefix: str, art: Path) -> None:
    # Seed via a separate workspace (proper apply path; injecting rows via
    # raw SQL would bypass the views/view_elements wiring and isn't a fair
    # test of "server added something").
    src = ws / "src"
    dst = ws / "dst"
    src.mkdir(); dst.mkdir()
    _init(src)
    _add(src, f"{prefix}-seed", f"Seed {prefix}", kind="service")
    _apply(src)

    _init(dst)
    _pull(dst)
    elems = (dst / ".tld" / "elements.yaml").read_text()
    _assert(f"{prefix}-seed" in elems,
            f"server-added element not pulled:\n{elems}")


def conflict_merge_friendly(ws: Path, prefix: str, art: Path) -> None:
    # Same element on both sides, different fields edited.
    src = ws / "src"; dst = ws / "dst"
    src.mkdir(); dst.mkdir()
    _init(src)
    _add(src, f"{prefix}-m", f"M {prefix}", kind="service",
         description="original", technology="Go")
    _apply(src)

    _init(dst)
    _pull(dst)

    # Server-side edit: change technology via src (applies to same ref).
    run_tld(["-w", str(src / ".tld"), "update", "element", f"{prefix}-m",
             "technology", "Rust"], cwd=src)
    _apply(src)

    # Local edit on dst: change description.
    run_tld(["-w", str(dst / ".tld"), "update", "element", f"{prefix}-m",
             "description", "local edit"], cwd=dst)

    _pull(dst)
    # Both fields should survive on the merged local workspace or subsequent apply.
    y = (dst / ".tld" / "elements.yaml").read_text()
    # At minimum, DB after pull+apply should reflect both edits.
    _apply(dst)
    els = db_elements(prefix)
    _assert(len(els) == 1, f"expected 1 element, got {els}")
    got = els[0]
    # We want *neither field silently lost*. Accept either side's winner for
    # description/technology as long as nothing is None that was set before.
    _assert(got["technology"] in ("Rust", "Go"),
            f"technology lost: {got}")
    _assert(got["description"] in ("local edit", "original"),
            f"description lost: {got}")
    _assert(db_dupes_elements(prefix) == [], "duplicate element after merge")


def conflict_same_field(ws: Path, prefix: str, art: Path) -> None:
    src = ws / "src"; dst = ws / "dst"
    src.mkdir(); dst.mkdir()
    _init(src)
    _add(src, f"{prefix}-c", f"C {prefix}", kind="service", description="v0")
    _apply(src)

    _init(dst)
    _pull(dst)

    # Edit the *same field* on both sides.
    run_tld(["-w", str(src / ".tld"), "update", "element", f"{prefix}-c",
             "description", "server-wins"], cwd=src)
    _apply(src)
    run_tld(["-w", str(dst / ".tld"), "update", "element", f"{prefix}-c",
             "description", "local-wins"], cwd=dst)

    # Pull should either merge (last-write-wins) or flag a conflict. We
    # don't care which, only that no data is silently doubled and that the
    # resulting DB state matches whatever the pull wrote locally.
    _pull(dst)
    _apply(dst)
    els = db_elements(prefix)
    _assert(len(els) == 1, f"expected 1 element, got {els}")
    _assert(els[0]["description"] in ("server-wins", "local-wins"),
            f"unexpected description: {els[0]}")
    _assert(db_dupes_elements(prefix) == [], "duplicate element after same-field conflict")


def connector_dedup_probe(ws: Path, prefix: str, art: Path) -> None:
    # Create identical connector on both sides between pull+apply; end state
    # must have exactly one row.
    src = ws / "src"; dst = ws / "dst"
    src.mkdir(); dst.mkdir()
    _init(src)
    _add(src, f"{prefix}-a", f"A {prefix}", kind="service")
    _add(src, f"{prefix}-b", f"B {prefix}", kind="database")
    _apply(src)

    _init(dst)
    _pull(dst)

    # Both add the same connector independently.
    _connect(src, f"{prefix}-a", f"{prefix}-b", label="writes")
    _apply(src)
    _connect(dst, f"{prefix}-a", f"{prefix}-b", label="writes")
    _pull(dst)
    _apply(dst)

    cons = db_connectors(prefix)
    dupes = db_dupes_connectors(prefix)
    _assert(dupes == [], f"DUPLICATE CONNECTORS: {dupes}")
    # Accept 1 (correctly deduped). >1 is a failure.
    _assert(len(cons) == 1,
            f"expected 1 connector after dedup, got {len(cons)}: {cons}")


def version_conflict_parallel_apply(ws: Path, prefix: str, art: Path) -> None:
    src = ws / "src"; a = ws / "a"; b = ws / "b"
    src.mkdir(); a.mkdir(); b.mkdir()
    _init(src)
    _add(src, f"{prefix}-seed", f"Seed {prefix}", kind="service")
    _apply(src)

    # Two copies pull the same baseline.
    _init(a); _pull(a)
    _init(b); _pull(b)

    # Each makes a different local change.
    run_tld(["-w", str(a / ".tld"), "update", "element", f"{prefix}-seed",
             "technology", "FromA"], cwd=a)
    run_tld(["-w", str(b / ".tld"), "update", "element", f"{prefix}-seed",
             "technology", "FromB"], cwd=b)

    # Parallel apply.
    def go(w: Path) -> helpers.ProcResult:
        return run_tld(["-w", str(w / ".tld"), "apply", "--force"],
                       cwd=w, expect_rc=None, timeout=60)

    with concurrent.futures.ThreadPoolExecutor(max_workers=2) as ex:
        fa = ex.submit(go, a)
        fb = ex.submit(go, b)
        ra, rb = fa.result(), fb.result()

    # Whatever the exact rules: no duplicates, exactly one element row.
    els = db_elements(prefix)
    _assert(len(els) == 1, f"expected 1 element after parallel apply, got {els}")
    _assert(db_dupes_elements(prefix) == [],
            "duplicate elements after parallel apply")
    tech = els[0]["technology"]
    _assert(tech in ("FromA", "FromB"),
            f"unexpected technology (not one of the parallel writes): {tech}")


def pull_during_concurrent_write(ws: Path, prefix: str, art: Path) -> None:
    # Seed, then during a pull, do another apply from a second workspace.
    # Assert final pull leaves DB-equivalent local state and no duplicates.
    src = ws / "src"; dst = ws / "dst"; rogue = ws / "rogue"
    for d in (src, dst, rogue):
        d.mkdir()
    _init(src)
    _add(src, f"{prefix}-s", f"S {prefix}", kind="service")
    _apply(src)

    _init(rogue)
    _pull(rogue)

    _init(dst)
    # Kick off a pull in dst; mid-flight, add+apply from rogue.
    def rogue_write():
        time.sleep(0.05)
        _add(rogue, f"{prefix}-mid", f"Mid {prefix}", kind="database")
        _apply(rogue)

    t = threading.Thread(target=rogue_write)
    t.start()
    _pull(dst)
    t.join()
    # Second pull picks up the mid-flight write.
    _pull(dst)

    elems = (dst / ".tld" / "elements.yaml").read_text()
    _assert(f"{prefix}-mid" in elems,
            f"concurrent-write element missing after second pull:\n{elems}")
    _assert(db_dupes_elements(prefix) == [],
            "duplicates after concurrent write")


def unicode_roundtrip(ws: Path, prefix: str, art: Path) -> None:
    src = ws / "src"; dst = ws / "dst"
    src.mkdir(); dst.mkdir()
    _init(src)
    fancy_name = f"Auth 🔐 Service {prefix}"
    _add(src, f"{prefix}-u", fancy_name, kind="service",
         description="日本語 with emoji 🚀")
    _add(src, f"{prefix}-v", f"V {prefix}", kind="database")
    _connect(src, f"{prefix}-u", f"{prefix}-v", label="流れ →")
    _apply(src)

    els = db_elements(prefix)
    _assert(len(els) == 2, f"expected 2 elements, got {len(els)}")
    by_ref = {e["ref"]: e for e in els}
    _assert(by_ref[f"{prefix}-u"]["name"] == fancy_name,
            f"name corrupted: {by_ref[f'{prefix}-u']}")

    _init(dst); _pull(dst)
    y = (dst / ".tld" / "elements.yaml").read_text()
    _assert("🔐" in y, f"emoji missing from pulled elements.yaml:\n{y}")
    _assert("日本語" in y, f"CJK missing from pulled elements.yaml:\n{y}")
    c = (dst / ".tld" / "connectors.yaml").read_text()
    _assert("流れ" in c, f"CJK label missing from pulled connectors.yaml:\n{c}")


# --- registry -------------------------------------------------------------

SCENARIOS = [
    ("apply_idempotence", apply_idempotence),
    ("apply_without_lockfile", apply_without_lockfile),
    ("pull_into_empty_workspace", pull_into_empty_workspace),
    ("pull_twice_byte_identical", pull_twice_byte_identical),
    ("pull_preserves_local_additions", pull_preserves_local_additions),
    ("pull_applies_server_additions", pull_applies_server_additions),
    ("conflict_merge_friendly", conflict_merge_friendly),
    ("conflict_same_field", conflict_same_field),
    ("connector_dedup_probe", connector_dedup_probe),
    ("version_conflict_parallel_apply", version_conflict_parallel_apply),
    ("pull_during_concurrent_write", pull_during_concurrent_write),
    ("unicode_roundtrip", unicode_roundtrip),
]
