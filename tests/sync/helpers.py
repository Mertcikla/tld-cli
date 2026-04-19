"""Helpers for the tld sync integration suite.

Every SELECT and DELETE is scoped by BOTH org_id AND a `synctest-`
ref/name prefix. Either filter alone is insufficient; together they make
an accidental broad wipe of a dev DB impossible.
"""

from __future__ import annotations

import os
import shutil
import subprocess
import time
import uuid
from contextlib import contextmanager
from dataclasses import dataclass
from pathlib import Path
from typing import Any

import psycopg2
import psycopg2.extras

TLD_BIN = os.environ.get("TLD_BIN", "tld")
DB_URL = os.environ.get("TLD_TEST_DB_URL", "postgres://postgres:postgres@localhost:5432/diag?sslmode=disable")
ORG_ID = os.environ.get("TLD_TEST_ORG_ID", "019d9faa-fbe9-78fe-845a-04987e68d7de")
RUN_ID = os.environ.get("TLD_TEST_RUN_ID") or f"{int(time.time())}-{os.getpid()}"
PREFIX_ROOT = "synctest"


class TldError(RuntimeError):
    pass


@dataclass
class ProcResult:
    rc: int
    stdout: str
    stderr: str
    args: list[str]
    cwd: Path

    def __str__(self) -> str:
        return f"tld {' '.join(self.args)} (cwd={self.cwd}) rc={self.rc}\n--- stdout ---\n{self.stdout}\n--- stderr ---\n{self.stderr}"


def require_env() -> None:
    if not ORG_ID:
        raise TldError("TLD_TEST_ORG_ID must be set to a dev org uuid (every DB op is scoped by both org_id and a synctest- prefix).")


def run_tld(args: list[str], *, cwd: Path, expect_rc: int | None = 0, timeout: int = 60, stdin: str | None = None, extra_env: dict[str, str] | None = None) -> ProcResult:
    env = os.environ.copy()
    if extra_env:
        env.update(extra_env)
    proc = subprocess.run([TLD_BIN, *args], cwd=cwd, capture_output=True, text=True, timeout=timeout, input=stdin, env=env)
    result = ProcResult(proc.returncode, proc.stdout, proc.stderr, args, cwd)
    if expect_rc is not None and proc.returncode != expect_rc:
        raise TldError(f"expected rc={expect_rc}, got rc={proc.returncode}\n{result}")
    return result


def unique_prefix(scenario: str) -> str:
    # synctest-<runid>-<scenario>-<uuid8>
    # Must match ^[a-z0-9-]+$ so slugify is idempotent.
    scen = scenario.lower().replace("_", "-")
    return f"{PREFIX_ROOT}-{RUN_ID}-{scen}-{uuid.uuid4().hex[:8]}"


# --- DB primitives --------------------------------------------------------


@contextmanager
def _conn():
    c = psycopg2.connect(DB_URL)
    try:
        yield c
    finally:
        c.close()


def db_query(sql: str, params: tuple[Any, ...] = ()) -> list[dict]:
    with _conn() as c:
        with c.cursor(cursor_factory=psycopg2.extras.RealDictCursor) as cur:
            cur.execute(sql, params)
            if cur.description is None:
                return []
            return [dict(r) for r in cur.fetchall()]


def db_exec(sql: str, params: tuple[Any, ...] = ()) -> int:
    with _conn() as c:
        with c.cursor() as cur:
            cur.execute(sql, params)
            c.commit()
            return cur.rowcount


def db_elements(prefix: str) -> list[dict]:
    return db_query(
        """
        SELECT id, ref, ref_suffix, name, kind, description, technology, url,
               has_view, tags
        FROM elements
        WHERE org_id = %s AND ref LIKE %s
        ORDER BY ref, ref_suffix
        """,
        (ORG_ID, prefix + "%"),
    )


def db_connectors(prefix: str) -> list[dict]:
    return db_query(
        """
        SELECT c.id, c.label, c.direction, c.description,
               src.ref AS source_ref, tgt.ref AS target_ref,
               v.name AS view_name, v.is_root AS view_is_root,
               owner.ref AS view_owner_ref
        FROM connectors c
        JOIN elements src ON src.id = c.source_element_id
        JOIN elements tgt ON tgt.id = c.target_element_id
        JOIN views v      ON v.id   = c.view_id
        LEFT JOIN elements owner ON owner.id = v.owner_element_id
        WHERE c.org_id = %s
          AND (src.ref LIKE %s OR tgt.ref LIKE %s)
        ORDER BY src.ref, tgt.ref, c.label
        """,
        (ORG_ID, prefix + "%", prefix + "%"),
    )


def db_dupes_elements(prefix: str) -> list[dict]:
    return db_query(
        """
        SELECT ref, count(*) AS n
        FROM elements
        WHERE org_id = %s AND ref LIKE %s
        GROUP BY ref
        HAVING count(*) > 1
        """,
        (ORG_ID, prefix + "%"),
    )


def db_dupes_connectors(prefix: str) -> list[dict]:
    return db_query(
        """
        SELECT src.ref AS source_ref, tgt.ref AS target_ref,
               c.view_id, c.label, count(*) AS n
        FROM connectors c
        JOIN elements src ON src.id = c.source_element_id
        JOIN elements tgt ON tgt.id = c.target_element_id
        WHERE c.org_id = %s
          AND (src.ref LIKE %s OR tgt.ref LIKE %s)
        GROUP BY src.ref, tgt.ref, c.view_id, c.label
        HAVING count(*) > 1
        """,
        (ORG_ID, prefix + "%", prefix + "%"),
    )


def db_audit_since(ts_iso: str) -> list[dict]:
    return db_query(
        """
        SELECT action, entity_type, entity_id, created_at
        FROM audit_logs
        WHERE org_id = %s AND created_at > %s
        ORDER BY created_at
        """,
        (ORG_ID, ts_iso),
    )


def db_now() -> str:
    return db_query("SELECT now() AS t")[0]["t"].isoformat()


def cleanup(prefix: str) -> None:
    """Scoped delete. Always (org_id AND ref/tag prefix).

    Order matters: connectors FK → elements, view_elements FK → elements &
    views, views FK → owner_element. We let ON DELETE CASCADE do the rest
    when we delete elements, but we also explicitly wipe tags.
    """
    # connectors involving any prefixed element (cascade from element delete
    # covers this too, but explicit is safer if FK is loosened later).
    db_exec(
        """
        DELETE FROM connectors
        WHERE org_id = %s
          AND (source_element_id IN (SELECT id FROM elements WHERE org_id=%s AND ref LIKE %s)
               OR target_element_id IN (SELECT id FROM elements WHERE org_id=%s AND ref LIKE %s))
        """,
        (ORG_ID, ORG_ID, prefix + "%", ORG_ID, prefix + "%"),
    )
    db_exec("DELETE FROM elements WHERE org_id = %s AND ref LIKE %s", (ORG_ID, prefix + "%"))
    db_exec("DELETE FROM tags WHERE org_id = %s AND tag LIKE %s", (ORG_ID, prefix + "%"))


def preflight_prefix_empty(prefix_root: str = PREFIX_ROOT) -> int:
    """Return count of any pre-existing synctest-* rows (for --force-cleanup)."""
    row = db_query("SELECT count(*) AS n FROM elements WHERE org_id = %s AND ref LIKE %s", (ORG_ID, prefix_root + "-%"))
    return row[0]["n"]


def wipe_all_synctest() -> None:
    cleanup(PREFIX_ROOT + "-")


# --- workspace utilities --------------------------------------------------


def snapshot_workspace(ws: Path) -> dict[str, str]:
    out: dict[str, str] = {}
    root = ws / ".tld"
    if not root.exists():
        return out
    for f in sorted(root.rglob("*")):
        if f.is_file():
            out[str(f.relative_to(ws))] = f.read_text(encoding="utf-8")
    return out


def dump_artifacts(artifacts_dir: Path, ws: Path, result: ProcResult | None) -> None:
    artifacts_dir.mkdir(parents=True, exist_ok=True)
    snap = snapshot_workspace(ws)
    for rel, content in snap.items():
        p = artifacts_dir / "workspace" / rel
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text(content)
    if result is not None:
        (artifacts_dir / "last_tld_stdout.txt").write_text(result.stdout)
        (artifacts_dir / "last_tld_stderr.txt").write_text(result.stderr)
        (artifacts_dir / "last_tld_args.txt").write_text(" ".join(result.args) + f"\nrc={result.rc}\ncwd={result.cwd}\n")


def rmtree(p: Path) -> None:
    if p.exists():
        shutil.rmtree(p)
