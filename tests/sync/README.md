# tld sync integration suite

Round-trip tests for `tld apply` / `tld pull` / `tld diff` / `tld status`
against the live local dev stack. Complements the Rust tests in
`tld/tests/` which are local-only (YAML mutation, slugify, analyze).

## Prerequisites

1. Local dev stack up:
   ```bash
   cd /path/to/diag-compose   # or wherever docker-compose.yml lives
   docker-compose up -d       # postgres on :5432, backend on :8080
   ```
2. `tld` installed (`cargo install --path .`) and logged into the local
   server (`tld login`).
3. Python 3.10+ with `psycopg2-binary`:
   ```bash
   pip install psycopg2-binary
   ```

## Env vars

| Var | Purpose | Default |
|---|---|---|
| `TLD_BIN` | path to `tld` binary | `tld` (PATH) |
| `TLD_TEST_DB_URL` | Postgres URL for ground-truth queries | `postgres://postgres:postgres@localhost:5432/diag?sslmode=disable` |
| `TLD_TEST_ORG_ID` | org uuid used for every query/DELETE | **required** |
| `TLD_TEST_RUN_ID` | unique suffix for `synctest-*` refs | auto (`{epoch}-{pid}`) |

## Running

```bash
# from tld/
export TLD_TEST_ORG_ID=<your dev org uuid>
make test-sync                                # all Phase 1 scenarios
python3 tests/sync/run_sync_suite.py --list   # list scenario names
python3 tests/sync/run_sync_suite.py --scenario apply_idempotence
python3 tests/sync/run_sync_suite.py --keep-artifacts
python3 tests/sync/run_sync_suite.py --force-cleanup   # wipe stale synctest-* rows first
```

Exit 0 iff every selected scenario passes. On failure, a scenario's
before/after workspace snapshot and last tld stdout/stderr are written
to `tests/sync/artifacts/<run_id>/<scenario>/`.

## Safety

Every SELECT and DELETE is scoped by **both** `org_id` AND
`ref LIKE 'synctest-%'`. Neither filter alone is sufficient; together
they make an accidental broad wipe impossible.

The runner refuses to start if any `synctest-*` rows already exist in
the DB (pre-existing pollution from a crashed run). Clear them with
`--force-cleanup`.

## Scenarios (Phase 1)

1. `apply_idempotence` — init → add → connect → apply; re-apply keeps same row IDs, no duplicates.
2. `apply_without_lockfile` — delete `.tld.lock`, apply still succeeds without duplicating rows.
3. `pull_into_empty_workspace` — fresh workspace pulls seeded server state; `diff` clean after.
4. `pull_twice_byte_identical` — re-pulling with no server changes yields byte-identical YAML.
5. `pull_preserves_local_additions` — pull against empty server doesn't erase local-only elements.
6. `pull_applies_server_additions` — pull picks up server-added element.
7. `conflict_merge_friendly` — different fields edited on same element both sides; no data loss.
8. `conflict_same_field` — same field edited on both sides; resolves to one of the writes, no duplicates.
9. `connector_dedup_probe` — identical connector added locally and remotely; DB has exactly one row.
10. `version_conflict_parallel_apply` — two parallel applies; exactly one element row remains.
11. `pull_during_concurrent_write` — server write mid-pull is picked up on next pull without duplicates.
12. `unicode_roundtrip` — emoji + CJK names/labels survive apply → pull byte-level.

## Phase 2
Add additional test scenarios for identified failures in phase 1. Design and add unit tests for any bugs that are hard to reproduce or reason about in an integration test.
Identify gaps in test coverage and add scenarios/unit tests as needed. Make a backend improvement proposal that emerged from the testing process (e.g. an API change, a new endpoint, a DB constraint, etc.) and detail it in a separate plan file.

## Phase 3
Breadth and robustness: JSON output matrix, YAML corruption probes,
stale-lock detection, `tag create` round-trip, `export` agreement with
`pull`, `--verbose` behavioral equivalence.
