#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 ZairenkaiPrjctX contributors
"""Track and compare the ART upstreams pinned in upstream/sources.toml.

Subcommands:
  validate                    offline: check sources.toml and the per-release records
  refs                        compare pinned commits with the live upstream refs (drift report)
  fetch   --workdir DIR       fetch pinned refs into a partial (blob:none) bare repository
  analyze --workdir DIR       merge bases, fork-only commits, patch-id equivalence (JSON)
  check   --workdir DIR       every fork-only commit has a record; adopted series applies

Nothing is ever pushed, and pinned revisions are never changed by this tool.
Standard library only (Python >= 3.11 for tomllib).
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
import tempfile
import tomllib
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SHA_RE = re.compile(r"^[0-9a-f]{40}$")
NAME_RE = re.compile(r"^[a-z][a-z0-9_-]{0,31}$")
DECISIONS = {"in_baseline", "adopt", "duplicate", "deferred", "rejected", "blocked"}
CATEGORIES = {
    "security_hardening", "runtime_correctness", "memory_safety", "compiler_codegen", "jit_aot",
    "interpreter", "gc_memory", "dex_verification", "startup", "art_service", "build_system",
    "architecture_abi", "performance", "observability", "rom_specific", "obsolete",
}


class ToolError(Exception):
    pass


@dataclass(frozen=True)
class Source:
    name: str
    role: str
    url: str
    release: str
    ref: str
    commit: str
    expected_merge_base: str | None = None


def git(repo: Path | None, *args: str, check: bool = True, input_: str | None = None) -> str:
    cmd = ["git"] + (["-C", str(repo)] if repo else []) + list(args)
    p = subprocess.run(cmd, capture_output=True, text=True, input=input_)
    if check and p.returncode != 0:
        raise ToolError(f"{' '.join(cmd)}: {p.stderr.strip()}")
    return p.stdout


# ---------------------------------------------------------------------------- loading/validation

def load_sources(path: Path) -> tuple[dict, list[Source]]:
    try:
        data = tomllib.loads(path.read_text())
    except (OSError, tomllib.TOMLDecodeError) as e:
        raise ToolError(f"{path}: {e}") from None
    errors = []
    if data.get("schema_version") != 1:
        errors.append("sources: schema_version must be 1")
    sources = []
    names = set()
    for i, s in enumerate(data.get("source", [])):
        try:
            src = Source(s["name"], s["role"], s["url"], s["release"], s["ref"], s["commit"],
                         s.get("expected_merge_base"))
        except KeyError as e:
            errors.append(f"source[{i}]: missing {e}")
            continue
        if not NAME_RE.match(src.name) or src.name in names:
            errors.append(f"source[{i}]: invalid or duplicate name {src.name!r}")
        names.add(src.name)
        if src.role not in ("canonical", "reference"):
            errors.append(f"source {src.name}: role must be canonical or reference")
        if not SHA_RE.match(src.commit):
            errors.append(f"source {src.name}: commit must be a full SHA-1")
        if src.expected_merge_base is not None and not SHA_RE.match(src.expected_merge_base):
            errors.append(f"source {src.name}: expected_merge_base must be a full SHA-1")
        if not src.url.startswith(("https://", "file://")):
            errors.append(f"source {src.name}: url must be https:// (or file:// in tests)")
        if not src.ref.startswith("refs/"):
            errors.append(f"source {src.name}: ref must be a full ref name")
        if src.release not in data.get("release", {}):
            errors.append(f"source {src.name}: unknown release {src.release!r}")
        sources.append(src)
    for rel, r in data.get("release", {}).items():
        if r.get("baseline") not in names:
            errors.append(f"release {rel}: baseline {r.get('baseline')!r} is not a source")
        elif next(s for s in sources if s.name == r["baseline"]).role != "canonical":
            errors.append(f"release {rel}: baseline must be the canonical source")
        if not SHA_RE.match(str(r.get("fork_point", ""))):
            errors.append(f"release {rel}: fork_point must be a full SHA-1")
        if not isinstance(r.get("api_level"), int):
            errors.append(f"release {rel}: api_level must be an integer")
    if errors:
        raise ToolError("\n".join(errors))
    return data, sources


def load_records(path: Path, source_names: set[str]) -> dict:
    try:
        data = tomllib.loads(path.read_text())
    except (OSError, tomllib.TOMLDecodeError) as e:
        raise ToolError(f"{path}: {e}") from None
    errors = []
    if data.get("schema_version") != 1:
        errors.append(f"{path.name}: schema_version must be 1")
    ids = set()
    commits = {}
    for i, c in enumerate(data.get("candidate", [])):
        where = f"{path.name} candidate[{i}]"
        missing = [k for k in ("id", "source", "commit", "subject", "author", "files", "categories",
                               "decision", "rationale", "validation") if k not in c]
        if missing:
            errors.append(f"{where}: missing {', '.join(missing)}")
            continue
        if c["id"] in ids:
            errors.append(f"{where}: duplicate id {c['id']}")
        ids.add(c["id"])
        if c["source"] not in source_names:
            errors.append(f"{where}: unknown source {c['source']}")
        if not SHA_RE.match(c["commit"]):
            errors.append(f"{where}: commit must be a full SHA-1")
        if (c["source"], c["commit"]) in commits:
            errors.append(f"{where}: commit recorded twice for {c['source']}")
        commits[(c["source"], c["commit"])] = c["id"]
        if c["decision"] not in DECISIONS:
            errors.append(f"{where}: decision must be one of {sorted(DECISIONS)}")
        bad = set(c["categories"]) - CATEGORIES
        if bad or not c["categories"]:
            errors.append(f"{where}: unknown or empty categories {sorted(bad)}")
        if c["decision"] == "in_baseline" and not SHA_RE.match(c.get("upstream_equivalent", "")):
            errors.append(f"{where}: in_baseline needs upstream_equivalent = full SHA in the baseline")
        if c["decision"] == "duplicate" and not c.get("duplicate_of"):
            errors.append(f"{where}: duplicate needs duplicate_of = candidate id")
        if len(c["rationale"].strip()) < 20:
            errors.append(f"{where}: rationale too short to be reviewable")
        for dep in c.get("depends_on", []):
            if not isinstance(dep, str):
                errors.append(f"{where}: depends_on entries must be candidate ids")
    order = data.get("series", {}).get("order", [])
    if len(order) != len(set(order)):
        errors.append("series: duplicate entries in order")
    for s in order:
        match = [c for c in data.get("candidate", []) if c.get("id") == s]
        if not match:
            errors.append(f"series: unknown candidate {s}")
        elif match[0].get("decision") != "adopt":
            errors.append(f"series: {s} is not decision=adopt")
    adopted = {c["id"] for c in data.get("candidate", []) if c.get("decision") == "adopt"}
    if adopted - set(order):
        errors.append(f"series: adopted candidates missing from order: {sorted(adopted - set(order))}")
    for c in data.get("candidate", []):
        for dep in c.get("depends_on", []):
            if dep not in ids:
                errors.append(f"{c.get('id')}: depends_on unknown id {dep}")
            elif c.get("decision") == "adopt" and c["id"] in order and (dep not in order or order.index(dep) > order.index(c["id"])):
                errors.append(f"{c.get('id')}: adopted, but its dependency {dep} is not adopted earlier in the series")
        if c.get("decision") == "duplicate" and c.get("duplicate_of") not in ids:
            errors.append(f"{c.get('id')}: duplicate_of unknown id {c.get('duplicate_of')}")
    if errors:
        raise ToolError("\n".join(errors))
    return data


# ------------------------------------------------------------------------------- git operations

def remote_head(url: str, ref: str) -> str | None:
    """Commit a ref points to; annotated tags are peeled."""
    out = git(None, "ls-remote", url, ref, f"{ref}^{{}}")
    found = {}
    for line in out.splitlines():
        sha, name = line.split("\t", 1)
        found[name] = sha
    return found.get(f"{ref}^{{}}") or found.get(ref)


def local_ref(src: Source) -> str:
    return f"refs/zart/{src.name}"


def fetch(workdir: Path, sources: list[Source], filter_blobs: bool = True) -> None:
    if not (workdir / "HEAD").exists():
        workdir.mkdir(parents=True, exist_ok=True)
        git(workdir, "init", "-q", "--bare")
    for src in sources:
        remotes = git(workdir, "remote").split()
        if src.name not in remotes:
            git(workdir, "remote", "add", src.name, src.url)
        if filter_blobs:
            git(workdir, "config", f"remote.{src.name}.promisor", "true")
            git(workdir, "config", f"remote.{src.name}.partialclonefilter", "blob:none")
        args = ["fetch", "-q", "--no-tags"] + (["--filter=blob:none"] if filter_blobs else [])
        git(workdir, *args, src.name, f"+{src.ref}:{local_ref(src)}-tip")
        tip = git(workdir, "rev-parse", f"{local_ref(src)}-tip").strip()
        if not _is_ancestor(workdir, src.commit, tip):
            raise ToolError(f"{src.name}: pinned {src.commit} is not reachable from {src.ref} ({tip})")
        git(workdir, "update-ref", local_ref(src), src.commit)


def _is_ancestor(repo: Path, a: str, b: str) -> bool:
    return subprocess.run(["git", "-C", str(repo), "merge-base", "--is-ancestor", a, b],
                          capture_output=True).returncode == 0


def patch_id(repo: Path, commit: str) -> str:
    diff = git(repo, "show", "--format=", commit)
    out = git(repo, "patch-id", "--stable", input_=diff).split()
    return out[0] if out else ""


def analyze(workdir: Path, data: dict, sources: list[Source]) -> dict:
    report = {"releases": {}}
    for rel, r in data["release"].items():
        base = next(s for s in sources if s.name == r["baseline"])
        fork_point = r["fork_point"]
        if not _is_ancestor(workdir, fork_point, base.commit):
            raise ToolError(f"{rel}: fork_point is not an ancestor of the baseline")
        baseline_pids = {}
        for c in git(workdir, "rev-list", "--no-merges", f"{fork_point}..{base.commit}").split():
            baseline_pids[patch_id(workdir, c)] = c
        entry = {"api_level": r["api_level"], "baseline": base.name, "baseline_commit": base.commit,
                 "fork_point": fork_point, "baseline_extra_commits": sorted(baseline_pids.values()),
                 "sources": {}}
        for src in (s for s in sources if s.release == rel and s.role == "reference"):
            mb = git(workdir, "merge-base", fork_point, src.commit).strip()
            expected_mb = src.expected_merge_base or fork_point
            commits = []
            for c in git(workdir, "rev-list", "--reverse", "--no-merges", f"{fork_point}..{src.commit}").split():
                pid = patch_id(workdir, c)
                meta = git(workdir, "show", "-s", "--format=%an%x00%ad%x00%s", "--date=short", c).rstrip("\n").split("\x00")
                files = [f for f in git(workdir, "show", "--format=", "--name-only", c).splitlines() if f]
                commits.append({"commit": c, "author": meta[0], "date": meta[1], "subject": meta[2],
                                 "files": files, "patch_id": pid,
                                 "in_baseline_as": baseline_pids.get(pid)})
            entry["sources"][src.name] = {
                "commit": src.commit,
                "merge_base": mb,
                "expected_merge_base": expected_mb,
                "merge_base_as_expected": mb == expected_mb,
                "baseline_only_commits": int(git(workdir, "rev-list", "--count", f"{src.commit}..{base.commit}")),
                "fork_only_commits": commits,
            }
        # Same patch carried by several sources (e.g. an AOSP fix cherry-picked by forks).
        by_pid: dict[str, list[tuple[str, str]]] = {}
        for name, s in entry["sources"].items():
            for c in s["fork_only_commits"]:
                by_pid.setdefault(c["patch_id"], []).append((name, c["commit"]))
        for name, s in entry["sources"].items():
            for c in s["fork_only_commits"]:
                c["same_patch_in"] = [f"{n}:{x}" for n, x in by_pid[c["patch_id"]] if n != name]
        report["releases"][rel] = entry
    return report


def apply_check(workdir: Path, base_commit: str, series: list[str]) -> list[dict]:
    """Cherry-pick the series onto the baseline in a throwaway sparse worktree."""
    if not series:
        return []
    files = set()
    for c in series:
        files.update(f for f in git(workdir, "show", "--format=", "--name-only", c).splitlines() if f)
    results = []
    with tempfile.TemporaryDirectory(prefix="zart-apply-") as tmp:
        wt = Path(tmp) / "wt"
        git(workdir, "worktree", "add", "-q", "--no-checkout", "--detach", str(wt), base_commit)
        try:
            git(wt, "sparse-checkout", "set", "--no-cone", *sorted(files))
            git(wt, "checkout", "-q", "--detach", base_commit)
            for c in series:
                p = subprocess.run(["git", "-C", str(wt), "-c", "user.name=zart", "-c", "user.email=zart@invalid", "-c", "commit.gpgsign=false",
                                    "cherry-pick", "--no-commit", c], capture_output=True, text=True)
                ok = p.returncode == 0
                results.append({"commit": c, "applies": ok, "detail": (p.stderr or p.stdout).strip()[:500]})
                if not ok:
                    break
                git(wt, "-c", "user.name=zart", "-c", "user.email=zart@invalid", "-c", "commit.gpgsign=false", "commit", "-q",
                    "--no-verify", "-m", f"apply-check {c}")
        finally:
            git(workdir, "worktree", "remove", "--force", str(wt), check=False)
            git(workdir, "worktree", "prune", check=False)
    return results


def check(workdir: Path, data: dict, sources: list[Source], records_dir: Path) -> dict:
    report = analyze(workdir, data, sources)
    problems = []
    out = {"releases": {}}
    for rel, entry in report["releases"].items():
        rec = load_records(records_dir / rel / "candidates.toml", {s.name for s in sources})
        recorded = {(c["source"], c["commit"]): c for c in rec.get("candidate", [])}
        for name, s in entry["sources"].items():
            if not s["merge_base_as_expected"]:
                problems.append(f"{rel}/{name}: merge base {s['merge_base']} differs from the expected {s['expected_merge_base']}")
            for c in s["fork_only_commits"]:
                r = recorded.get((name, c["commit"]))
                if r is None:
                    problems.append(f"{rel}/{name}: {c['commit'][:10]} {c['subject']!r} has no decision record")
                elif c["in_baseline_as"] and r["decision"] != "in_baseline":
                    problems.append(f"{rel}/{name}: {c['commit'][:10]} is in the baseline as {c['in_baseline_as'][:10]} but recorded {r['decision']}")
                elif r["decision"] == "in_baseline" and r["upstream_equivalent"] != c["in_baseline_as"]:
                    problems.append(f"{rel}/{name}: {c['commit'][:10]} recorded in_baseline but no matching baseline patch")
                elif r["decision"] == "duplicate":
                    other = next((x for x in rec["candidate"] if x["id"] == r["duplicate_of"]), None)
                    if other is None or f"{other['source']}:{other['commit']}" not in c["same_patch_in"]:
                        problems.append(f"{rel}/{name}: {c['commit'][:10]} recorded duplicate of {r['duplicate_of']} but the patches differ")
        known = {(n, c["commit"]) for n, s in entry["sources"].items() for c in s["fork_only_commits"]}
        for key, r in recorded.items():
            if key not in known:
                problems.append(f"{rel}: record {r['id']} refers to a commit that is not fork-only on {key[0]}")
        order = rec.get("series", {}).get("order", [])
        by_id = {c["id"]: c for c in rec.get("candidate", [])}
        series = [by_id[i]["commit"] for i in order]
        applied = apply_check(workdir, entry["baseline_commit"], series)
        for a in applied:
            if not a["applies"]:
                problems.append(f"{rel}: series commit {a['commit'][:10]} does not apply: {a['detail']}")
        out["releases"][rel] = {"series": applied}
    out["problems"] = problems
    return out


# ------------------------------------------------------------------------------------- commands

def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--sources", type=Path, default=ROOT / "upstream" / "sources.toml")
    ap.add_argument("--records", type=Path, default=ROOT / "upstream")
    sub = ap.add_subparsers(dest="cmd", required=True)
    sub.add_parser("validate")
    sub.add_parser("refs")
    for name in ("fetch", "analyze", "check"):
        p = sub.add_parser(name)
        p.add_argument("--workdir", type=Path, required=True)
        if name == "fetch":
            p.add_argument("--no-filter", action="store_true", help="fetch blobs too")
    args = ap.parse_args(argv)
    try:
        data, sources = load_sources(args.sources)
        if args.cmd == "validate":
            for rel in data["release"]:
                load_records(args.records / rel / "candidates.toml", {s.name for s in sources})
            print(f"ok: {len(sources)} sources, releases {sorted(data['release'])}")
            return 0
        if args.cmd == "refs":
            drift = 0
            for s in sources:
                live = remote_head(s.url, s.ref)
                state = "pinned" if live == s.commit else ("ref missing" if live is None else f"moved to {live}")
                drift += live != s.commit
                print(f"{s.name:10} {s.ref:42} {s.commit[:12]}  {state}")
            return 1 if drift else 0
        if args.cmd == "fetch":
            fetch(args.workdir, sources, filter_blobs=not args.no_filter)
            print(f"fetched {len(sources)} sources into {args.workdir}")
            return 0
        if args.cmd == "analyze":
            print(json.dumps(analyze(args.workdir, data, sources), indent=2))
            return 0
        if args.cmd == "check":
            res = check(args.workdir, data, sources, args.records)
            print(json.dumps(res, indent=2))
            return 1 if res["problems"] else 0
    except ToolError as e:
        print(f"error: {e}", file=sys.stderr)
        return 2
    return 64


if __name__ == "__main__":
    sys.exit(main())
