#!/usr/bin/env python3
"""tld CLI sync integration suite entry point.

Runs scenarios against a live local backend + Postgres. Every scenario
gets a fresh tempdir workspace and a unique `synctest-*` ref prefix; all
DB operations are scoped by BOTH org_id and that prefix.

Usage:
    python3 run_sync_suite.py                      # all Phase 1 scenarios
    python3 run_sync_suite.py --scenario <name>    # one scenario
    python3 run_sync_suite.py --list               # list scenario names
    python3 run_sync_suite.py --keep-artifacts     # don't delete tempdirs
    python3 run_sync_suite.py --force-cleanup      # wipe pre-existing synctest-* rows first

Env:
    TLD_BIN               (default: tld)
    TLD_TEST_DB_URL       (default: postgres://postgres:postgres@localhost:5432/diag?sslmode=disable)
    TLD_TEST_ORG_ID       REQUIRED — dev org uuid
    TLD_TEST_BACKEND_URL  (default: http://localhost:8080)
"""

from __future__ import annotations

import argparse
import sys
import tempfile
import time
import traceback
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

import helpers  # noqa: E402
from scenarios import phase1, phase2, phase3  # noqa: E402

GREEN = "\033[32m"
RED = "\033[31m"
YELLOW = "\033[33m"
DIM = "\033[2m"
RESET = "\033[0m"


def collect() -> list[tuple[str, callable]]:
    return list(phase1.SCENARIOS) + list(phase2.SCENARIOS) + list(phase3.SCENARIOS)


def run_one(name: str, fn, artifacts_root: Path, keep: bool) -> tuple[bool, str]:
    prefix = helpers.unique_prefix(name)
    art = artifacts_root / name
    with tempfile.TemporaryDirectory(prefix=f"tldsync-{name}-") as tmp:
        ws = Path(tmp)
        try:
            fn(ws, prefix, art)
            return True, ""
        except Exception as e:
            tb = traceback.format_exc()
            try:
                helpers.dump_artifacts(art, ws, None)
                (art / "traceback.txt").write_text(tb)
            except Exception:
                pass
            return False, f"{e.__class__.__name__}: {e}\n{tb}"
        finally:
            try:
                helpers.cleanup(prefix)
            except Exception as e:
                print(f"{YELLOW}[warn]{RESET} cleanup failed for {prefix}: {e}")
            if keep:
                # copy workspace out of the soon-to-be-removed tmpdir
                try:
                    helpers.dump_artifacts(art, ws, None)
                except Exception:
                    pass


def main() -> int:
    p = argparse.ArgumentParser()
    p.add_argument("--scenario", help="Run a single scenario by name")
    p.add_argument("--list", action="store_true", help="List available scenarios and exit")
    p.add_argument("--keep-artifacts", action="store_true")
    p.add_argument("--force-cleanup", action="store_true", help="Wipe any pre-existing synctest-* rows before running")
    args = p.parse_args()

    scenarios = collect()
    if args.list:
        for name, _ in scenarios:
            print(name)
        return 0

    if args.scenario:
        scenarios = [(n, f) for n, f in scenarios if n == args.scenario]
        if not scenarios:
            print(f"no such scenario: {args.scenario}", file=sys.stderr)
            return 2

    try:
        helpers.require_env()
    except helpers.TldError as e:
        print(f"{RED}{e}{RESET}", file=sys.stderr)
        return 2

    pre = helpers.preflight_prefix_empty()
    if pre > 0:
        if args.force_cleanup:
            print(f"{YELLOW}[force-cleanup]{RESET} wiping {pre} pre-existing synctest-* rows")
            helpers.wipe_all_synctest()
        else:
            print(
                f"{RED}refusing to run: {pre} pre-existing synctest-* rows in DB. "
                f"Re-run with --force-cleanup.{RESET}",
                file=sys.stderr,
            )
            return 2

    artifacts_root = HERE / "artifacts" / helpers.RUN_ID
    print(f"{DIM}run_id={helpers.RUN_ID}  org={helpers.ORG_ID}  artifacts={artifacts_root}{RESET}")

    results: list[tuple[str, bool, float, str]] = []
    for name, fn in scenarios:
        t0 = time.time()
        print(f"  {DIM}→{RESET} {name} ... ", end="", flush=True)
        ok, msg = run_one(name, fn, artifacts_root, args.keep_artifacts)
        dt = time.time() - t0
        results.append((name, ok, dt, msg))
        mark = f"{GREEN}PASS{RESET}" if ok else f"{RED}FAIL{RESET}"
        print(f"{mark} ({dt:.2f}s)")
        if not ok:
            for line in msg.rstrip().splitlines():
                print(f"      {line}")

    npass = sum(1 for _, ok, _, _ in results if ok)
    nfail = len(results) - npass
    color = GREEN if nfail == 0 else RED
    print(f"\n{color}{npass}/{len(results)} passed{RESET}  ({nfail} failed)")
    return 0 if nfail == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
