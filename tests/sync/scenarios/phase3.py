"""Phase 3 scenarios: concurrent CLI and Frontend usage.

Simulates how the CLI tools handles concurrent changes made by the frontend 
(using the ConnectRPC HTTP API) such as creates, updates, and deletes.
"""

from __future__ import annotations

import concurrent.futures
import json
import urllib.request
import urllib.error
from pathlib import Path

import helpers
from helpers import db_elements, ORG_ID
from scenarios.phase1 import _init, _add, _apply, _pull, _assert

API_KEY = "tld_5368c3a5bd16cc13f1ee10a9f7fa3f0ec012275097d8c7549ebf92a1cfd2f1a3"
BACKEND_URL = "http://localhost:8080"

def api_call(method: str, payload: dict) -> dict:
    url = f"{BACKEND_URL}/api/diag.v1.WorkspaceService/{method}"
    data = json.dumps(payload).encode('utf-8')
    req = urllib.request.Request(url, data=data, headers={
        'Content-Type': 'application/json',
        'Authorization': f'Bearer {API_KEY}'
    })
    try:
        with urllib.request.urlopen(req) as resp:
            return json.loads(resp.read().decode('utf-8'))
    except urllib.error.HTTPError as e:
        body = e.read().decode('utf-8')
        raise RuntimeError(f"API Error {e.code}: {body}")

def concurrent_frontend_create_element(ws: Path, prefix: str, art: Path) -> None:
    """Frontend creates an element while the CLI builds and applies its own."""
    _init(ws)
    
    # Frontend creates an element
    resp = api_call("CreateElement", {
        "name": f"{prefix} Frontend",
        "kind": "service"
    })
    fe_el = resp.get("element", {})
    _assert(fe_el.get("name") == f"{prefix} Frontend", "Frontend creation failed")

    # CLI creates another element
    _add(ws, f"{prefix}-cli", f"CLI {prefix}", kind="database")
    _apply(ws)

    # CLI pulls to merge
    _pull(ws)

    els = db_elements(prefix)
    _assert(len(els) == 2, f"Expected 2 elements, got {len(els)}")
    
    elems_yaml = (ws / ".tld" / "elements.yaml").read_text()
    _assert(f"{prefix} Frontend" in elems_yaml, "Frontend element missing from pulled YAML")

def concurrent_frontend_update_conflict(ws: Path, prefix: str, art: Path) -> None:
    """CLI and Frontend update the same element concurrently."""
    _init(ws)
    _add(ws, f"{prefix}-conflict", f"Conflict {prefix}", kind="service", description="v0")
    _apply(ws)

    els = db_elements(prefix)
    el_id = els[0]["id"]
    
    _pull(ws)

    # Frontend updates the element description
    api_call("UpdateElement", {
        "elementId": el_id,
        "name": f"{prefix} Conflict",
        "description": "frontend-wins"
    })

    # CLI updates the SAME element description locally
    helpers.run_tld(["-w", str(ws / ".tld"), "update", "element", f"{prefix}-conflict", "description", "cli-wins"], cwd=ws)

    # CLI pulls and applies
    _pull(ws)
    _apply(ws)

    els = db_elements(prefix)
    _assert(els[0]["description"] in ("frontend-wins", "cli-wins"), f"Lost updates: {els[0]['description']}")

def concurrent_frontend_delete_element(ws: Path, prefix: str, art: Path) -> None:
    """Frontend deletes an element that the CLI is about to update."""
    _init(ws)
    _add(ws, f"{prefix}-del", f"Delete {prefix}", kind="service")
    _apply(ws)
    
    els = db_elements(prefix)
    el_id = els[0]["id"]
    
    _pull(ws)
    
    # Frontend deletes it
    api_call("DeleteElement", {
        "orgId": ORG_ID,
        "elementId": el_id
    })
    
    # CLI tries to update it locally
    helpers.run_tld(["-w", str(ws / ".tld"), "update", "element", f"{prefix}-del", "description", "updated locally"], cwd=ws)
    
    # Apply may fail or recreate the element. We just want it not to panic.
    res = helpers.run_tld(["-w", str(ws / ".tld"), "apply", "--force"], cwd=ws, expect_rc=None)
    _assert("panic" not in res.stderr.lower(), f"CLI panicked on applying update to deleted element:\n{res.stderr}")
    
    _pull(ws)
    
    els = db_elements(prefix)
    _assert(len(els) in (0, 1), f"Unexpected number of elements: {len(els)}")

def concurrent_cli_apply_frontend_update(ws: Path, prefix: str, art: Path) -> None:
    """CLI Apply and Frontend Update racing on the same element."""
    _init(ws)
    _add(ws, f"{prefix}-race", f"Race {prefix}", kind="service")
    _apply(ws)
    
    els = db_elements(prefix)
    el_id = els[0]["id"]
    
    # Both CLI and Frontend try to update at the exact same time
    def run_cli():
        helpers.run_tld(["-w", str(ws / ".tld"), "update", "element", f"{prefix}-race", "description", "cli"], cwd=ws)
        return helpers.run_tld(["-w", str(ws / ".tld"), "apply", "--force"], cwd=ws, expect_rc=None)
        
    def run_fe():
        return api_call("UpdateElement", {
            "elementId": el_id,
            "name": f"{prefix} Race",
            "description": "frontend"
        })
        
    with concurrent.futures.ThreadPoolExecutor(max_workers=2) as ex:
        cli_fut = ex.submit(run_cli)
        fe_fut = ex.submit(run_fe)
        
        cli_res = cli_fut.result()
        fe_res = fe_fut.result()
        
    els = db_elements(prefix)
    _assert(len(els) == 1, f"Concurrency caused duplicate element or deletion. Got {len(els)}")
    _assert(els[0]["description"] in ("cli", "frontend", None), f"Unknown description {els[0]['description']}")


SCENARIOS = [
    ("concurrent_frontend_create_element", concurrent_frontend_create_element),
    ("concurrent_frontend_update_conflict", concurrent_frontend_update_conflict),
    ("concurrent_frontend_delete_element", concurrent_frontend_delete_element),
    ("concurrent_cli_apply_frontend_update", concurrent_cli_apply_frontend_update),
]
