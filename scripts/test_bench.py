#!/usr/bin/env python3
"""Benchmark runner for `tld analyze` across a list of git repositories.

The script reads a text file containing one git URL per line, clones each
repository into a local work directory, runs `tld analyze` in each supported
view mode, appends the captured output to a results file, and removes generated
workspace artifacts before continuing to the next repository.
"""

from __future__ import annotations

import argparse
import hashlib
import re
import shutil
import subprocess
import sys
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Iterable
from urllib.parse import urlparse

VIEWS = ("structural", "business", "data-flow")
DEFAULT_RESULTS_FILE = "tld-bench-results.txt"
DEFAULT_WORK_ROOT = "tld-bench-work"


@dataclass(frozen=True)
class RunResult:
    url: str
    repo_dir: Path
    view: str
    returncode: int
    stdout: str
    stderr: str


def generate_run_id() -> str:
    return datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")


def read_urls(input_file: Path) -> list[str]:
    urls: list[str] = []
    for raw_line in input_file.read_text(encoding="utf-8").splitlines():
        line = raw_line.strip()
        if not line or line.startswith("#"):
            continue
        urls.append(line)
    return urls


def sanitize_repo_name(url: str) -> str:
    parsed = urlparse(url)
    candidate = parsed.path.rstrip("/") or parsed.netloc or url
    candidate = candidate.split("/")[-1]
    candidate = re.sub(r"(?i)\.git$", "", candidate)
    candidate = re.sub(r"[^A-Za-z0-9._-]+", "-", candidate).strip(".-_")
    if not candidate:
        candidate = "repo"
    digest = hashlib.sha1(url.encode("utf-8")).hexdigest()[:8]
    return f"{candidate}-{digest}"


def ensure_empty_directory(path: Path) -> None:
    if path.exists():
        shutil.rmtree(path)
    path.mkdir(parents=True, exist_ok=True)


def clone_repo(url: str, target_dir: Path, depth: int) -> subprocess.CompletedProcess[str]:
    return subprocess.run(["git", "clone", "--depth", str(depth), url, str(target_dir)], text=True, capture_output=True, check=False)


def run_tld_analyze(tld_bin: str, repo_dir: Path, view: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run([tld_bin, "analyze", ".", "--view", view], cwd=repo_dir, text=True, capture_output=True, check=False)


def remove_analysis_artifacts(repo_dir: Path) -> None:
    for relative_path in ("elements.yaml", "connectors.yaml"):
        artifact = repo_dir / relative_path
        if artifact.exists():
            artifact.unlink()


def append_result(results_file: Path, run_id: str, result: RunResult) -> None:
    with results_file.open("a", encoding="utf-8") as handle:
        handle.write("=" * 88 + "\n")
        handle.write(f"repo: {result.repo_dir} | {result.view}\n")
        handle.write(result.stdout.rstrip() + "\n" if result.stdout else "\n")
        handle.write(result.stderr.rstrip() + "\n" if result.stderr else "\n")


def process_repository(url: str, work_root: Path, results_file: Path, run_id: str, tld_bin: str, depth: int, keep_repos: bool) -> None:
    repo_name = sanitize_repo_name(url)
    repo_dir = work_root / repo_name
    ensure_empty_directory(repo_dir)

    clone = clone_repo(url, repo_dir, depth)
    if clone.returncode != 0:
        append_result(results_file, run_id, RunResult(url, repo_dir, "clone", clone.returncode, clone.stdout, clone.stderr))
        if not keep_repos:
            shutil.rmtree(repo_dir, ignore_errors=True)
        return

    try:
        for view in VIEWS:
            analyze = run_tld_analyze(tld_bin, repo_dir, view)
            append_result(results_file, run_id, RunResult(url, repo_dir, view, analyze.returncode, analyze.stdout, analyze.stderr))
            remove_analysis_artifacts(repo_dir)
    finally:
        if not keep_repos:
            shutil.rmtree(repo_dir, ignore_errors=True)


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Clone repos from a URL list and benchmark tld analyze across all supported views.")
    parser.add_argument("--input_file", type=Path, default="./repos.txt", help="Text file containing one git URL per line.")
    parser.add_argument("--results-file", type=Path, default=Path(DEFAULT_RESULTS_FILE), help=f"File to append run results to (default: {DEFAULT_RESULTS_FILE}).")
    parser.add_argument("--work-root", type=Path, default=Path(DEFAULT_WORK_ROOT), help=f"Directory used for temporary repo clones (default: {DEFAULT_WORK_ROOT}).")
    parser.add_argument("--run-id", default=None, help="Run identifier to include in the output log. Defaults to a UTC timestamp.")
    parser.add_argument("--tld-bin", default="tld", help='Path to the tld binary to run (default: "tld").')
    parser.add_argument("--depth", type=int, default=1, help="Git clone depth for each repository (default: 1).")
    parser.add_argument("--keep-repos", action="store_true", help="Keep cloned repositories on disk after processing.")
    return parser


def main(argv: Iterable[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    run_id = args.run_id or generate_run_id()

    if not args.input_file.exists():
        print(f"Input file does not exist: {args.input_file}", file=sys.stderr)
        return 2

    args.work_root.mkdir(parents=True, exist_ok=True)
    if args.results_file.parent:
        args.results_file.parent.mkdir(parents=True, exist_ok=True)

    urls = read_urls(args.input_file)
    if not urls:
        print("No repository URLs found in the input file.", file=sys.stderr)
        return 1

    for index, url in enumerate(urls, start=1):
        print(f"[{index}/{len(urls)}] processing {url}")
        process_repository(url=url, work_root=args.work_root, results_file=args.results_file, run_id=run_id, tld_bin=args.tld_bin, depth=args.depth, keep_repos=args.keep_repos)

    print(f"Results appended to {args.results_file} (run-id: {run_id})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
