"""Phase 2 scenarios: targeted fixes and robustness.

Contains targeted tests for identified Phase 1 failures (Bug B and C),
plus additional tests for concurrency, lockfile edge cases, and 
corruption resilience.
"""

from __future__ import annotations

import concurrent.futures
import time
from pathlib import Path

import helpers
from helpers import (
    db_elements,
    db_dupes_elements,
    run_tld,
)
from scenarios.phase1 import _init, _add, _apply, _pull, _assert

def pull_empty_remote_with_local_additions(ws: Path, prefix: str, art: Path) -> None:
    """Validates Bug C end-to-end: pulling from empty remote shouldn't clobber local additions."""
    _init(ws)
    _add(ws, f"{prefix}-c-mine", f"MyThing {prefix}", kind="service")
    # At this point, local workspace has the element but server is empty.
    # Pull should retain local additions.
    _pull(ws)
    
    elems = (ws / ".tld" / "elements.yaml").read_text()
    _assert(f"{prefix}-c-mine" in elems, f"Local element clobbered by pull from empty server:\n{elems}")

def ref_persistence_after_pull(ws: Path, prefix: str, art: Path) -> None:
    """Validates Bug B end-to-end: explicitly set ref should be preserved as YAML key on pull."""
    src = ws / "src"
    dst = ws / "dst"
    src.mkdir()
    dst.mkdir()
    
    _init(src)
    _add(src, f"{prefix}-bugb-seed", "Seed Element", kind="service")
    _apply(src)
    
    _init(dst)
    _pull(dst)
    
    elems = (dst / ".tld" / "elements.yaml").read_text()
    # The key should be exactly the ref, not 'seed-element'
    _assert(f"{prefix}-bugb-seed:\n" in ("\n" + elems) or elems.startswith(f"{prefix}-bugb-seed:\n"),
            f"Explicit ref not preserved as YAML key after pull:\n{elems}")
            
    # Also verify updating via ref works
    u = run_tld(["-w", str(dst / ".tld"), "update", "element", f"{prefix}-bugb-seed", "description", "updated"], cwd=dst, expect_rc=None)
    _assert(u.rc == 0, f"Updating element by explicit ref failed (element not found?):\n{u.stderr}")

def concurrent_lockfile_edge_cases(ws: Path, prefix: str, art: Path) -> None:
    """Test behavior when pull/apply encounters lockfile contention or corruption."""
    _init(ws)
    _add(ws, f"{prefix}-lock-test", f"Lock Test {prefix}", kind="service")
    
    # Write a bogus lockfile
    lock = ws / ".tld" / ".tld.lock"
    lock.write_text('{"corrupt": true, "not_a_real_lock": [}')
    
    # Apply should either recover by overwriting the bad lock, or fail gracefully
    res = run_tld(["-w", str(ws / ".tld"), "apply", "--force"], cwd=ws, expect_rc=None)
    # We just want to ensure it doesn't panic/crash without a clean error
    _assert("panic" not in res.stderr.lower(), f"CLI panicked on corrupt lockfile:\n{res.stderr}")
    
    # Verify if it succeeded, the DB has the element without duplicates
    if res.rc == 0:
        els = db_elements(prefix)
        _assert(len(els) == 1, f"Expected 1 element after applying over bad lock, got {len(els)}")

def pull_with_corrupt_local_yaml(ws: Path, prefix: str, art: Path) -> None:
    """Test that pulling into a directory with syntactically invalid YAML fails gracefully."""
    src = ws / "src"
    dst = ws / "dst"
    src.mkdir()
    dst.mkdir()
    
    _init(src)
    _add(src, f"{prefix}-yaml-seed", "YAML Seed", kind="service")
    _apply(src)
    
    _init(dst)
    # Corrupt elements.yaml before pull
    (dst / ".tld" / "elements.yaml").write_text("invalid:\n  yaml:\n  - syntax error\nwhat:")
    
    res = run_tld(["-w", str(dst / ".tld"), "pull", "--force"], cwd=dst, expect_rc=None)
    _assert(res.rc != 0, "Pull should fail when local YAML is corrupt")
    _assert("panic" not in res.stderr.lower(), f"CLI panicked on corrupt YAML:\n{res.stderr}")


SCENARIOS = [
    ("pull_empty_remote_with_local_additions", pull_empty_remote_with_local_additions),
    ("ref_persistence_after_pull", ref_persistence_after_pull),
    ("concurrent_lockfile_edge_cases", concurrent_lockfile_edge_cases),
    ("pull_with_corrupt_local_yaml", pull_with_corrupt_local_yaml),
]
