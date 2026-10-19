"""Tests for tools/art_upstream.py against small synthetic git repositories (no network)."""

from __future__ import annotations

import contextlib
import io
import json
import os
import subprocess
import sys
import tempfile
import textwrap
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
import art_upstream as au  # noqa: E402

# Isolate from the user's git configuration (signing, hooks, templates).
GIT_ENV = {
    "GIT_CONFIG_GLOBAL": os.devnull,
    "GIT_CONFIG_NOSYSTEM": "1",
    "GIT_AUTHOR_NAME": "Upstream Author",
    "GIT_AUTHOR_EMAIL": "author@example.invalid",
    "GIT_COMMITTER_NAME": "Upstream Author",
    "GIT_COMMITTER_EMAIL": "author@example.invalid",
}


def sh(repo: Path, *args: str) -> str:
    return subprocess.run(["git", "-C", str(repo), *args], check=True, capture_output=True,
                          text=True).stdout.strip()


def commit_file(repo: Path, name: str, content: str, msg: str) -> str:
    (repo / name).parent.mkdir(parents=True, exist_ok=True)
    (repo / name).write_text(content)
    sh(repo, "add", name)
    sh(repo, "commit", "-q", "-m", msg)
    return sh(repo, "rev-parse", "HEAD")


class GitEnv(unittest.TestCase):
    def setUp(self) -> None:
        self._old = {k: os.environ.get(k) for k in GIT_ENV}
        os.environ.update(GIT_ENV)
        self.tmp = Path(tempfile.mkdtemp(prefix="zart-test-"))

    def tearDown(self) -> None:
        for k, v in self._old.items():
            if v is None:
                os.environ.pop(k, None)
            else:
                os.environ[k] = v
        subprocess.run(["rm", "-rf", str(self.tmp)], check=False)


class Fixture(GitEnv):
    """aosp: base -> r1 (tag); security branch is a sibling of r1 carrying FIX.
    forka: r1 + FIX cherry-pick + a unique change. forkb: r1 + a change that conflicts with forka."""

    def setUp(self) -> None:
        super().setUp()
        aosp = self.tmp / "aosp"
        aosp.mkdir()
        sh(aosp, "init", "-q", "-b", "main")
        sh(aosp, "config", "uploadpack.allowFilter", "true")
        sh(aosp, "config", "uploadpack.allowAnySHA1InWant", "true")
        commit_file(aosp, "runtime/a.cc", "int a = 1;\n", "base")
        self.mb = commit_file(aosp, "runtime/b.cc", "int b = 1;\n", "merge base")
        sh(aosp, "checkout", "-q", "-b", "security")
        self.fix = commit_file(aosp, "runtime/a.cc", "int a = 1;\nint fixed = 1;\n", "security fix")
        sh(aosp, "checkout", "-q", "main")
        self.r1 = commit_file(aosp, "runtime/c.cc", "int c = 1;\n", "r1 only")
        sh(aosp, "tag", "-a", "-m", "r1", "r1")
        self.aosp = aosp

        forka = self.tmp / "forka"
        sh(self.tmp, "clone", "-q", str(aosp), str(forka))
        sh(forka, "checkout", "-q", "-b", "fork", self.r1)
        sh(forka, "cherry-pick", "-x", self.fix)
        self.forka_fix = sh(forka, "rev-parse", "HEAD")
        self.forka_own = commit_file(forka, "runtime/d.cc", "int d = 1;\n", "fork A change")
        sh(forka, "config", "uploadpack.allowFilter", "true")
        self.forka = forka

        forkb = self.tmp / "forkb"
        sh(self.tmp, "clone", "-q", str(aosp), str(forkb))
        sh(forkb, "checkout", "-q", "-b", "fork", self.r1)
        self.forkb_own = commit_file(forkb, "runtime/d.cc", "int d = 2;\n", "fork B change")
        sh(forkb, "config", "uploadpack.allowFilter", "true")
        self.forkb = forkb

        self.sources = self.tmp / "sources.toml"
        self.sources.write_text(textwrap.dedent(f"""\
            schema_version = 1
            [release.rel]
            api_level = 37
            baseline = "aosp"
            fork_point = "{self.r1}"
            [[source]]
            name = "aosp"
            role = "canonical"
            url = "file://{aosp}"
            release = "rel"
            ref = "refs/tags/r1"
            commit = "{self.r1}"
            [[source]]
            name = "sec"
            role = "reference"
            url = "file://{aosp}"
            release = "rel"
            ref = "refs/heads/security"
            commit = "{self.fix}"
            expected_merge_base = "{self.mb}"
            [[source]]
            name = "forka"
            role = "reference"
            url = "file://{forka}"
            release = "rel"
            ref = "refs/heads/fork"
            commit = "{self.forka_own}"
            [[source]]
            name = "forkb"
            role = "reference"
            url = "file://{forkb}"
            release = "rel"
            ref = "refs/heads/fork"
            commit = "{self.forkb_own}"
            """))
        self.records = self.tmp / "records"
        (self.records / "rel").mkdir(parents=True)
        self.work = self.tmp / "work"

    def write_records(self, series: list[str], decisions: dict[str, str] | None = None,
                      skip: tuple[str, ...] = ()) -> None:
        decisions = decisions or {}
        rows = [
            ("sec-fix", "sec", self.fix, "security fix", "adopt", ""),
            ("forka-fix", "forka", self.forka_fix, "security fix", "duplicate", 'duplicate_of = "sec-fix"'),
            ("forka-own", "forka", self.forka_own, "fork A change", "deferred", ""),
            ("forkb-own", "forkb", self.forkb_own, "fork B change", "rejected", ""),
        ]
        out = ["schema_version = 1", "[series]", f"order = {json.dumps(series)}", ""]
        for cid, src, sha, subj, dec, extra in rows:
            if cid in skip:
                continue
            dec = decisions.get(cid, dec)
            out += ["[[candidate]]", f'id = "{cid}"', f'source = "{src}"', f'commit = "{sha}"',
                    f'subject = "{subj}"', 'author = "Upstream Author"', 'files = ["runtime/x.cc"]',
                    'categories = ["runtime_correctness"]', f'decision = "{dec}"',
                    'rationale = "a reviewable rationale for this record"', 'validation = "test"', extra, ""]
        (self.records / "rel" / "candidates.toml").write_text("\n".join(out))

    def run_tool(self, *args: str) -> tuple[int, str, str]:
        out, err = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            code = au.main(["--sources", str(self.sources), "--records", str(self.records), *args])
        return code, out.getvalue(), err.getvalue()


class SourcesValidation(GitEnv):
    def write(self, body: str) -> Path:
        p = self.tmp / "s.toml"
        p.write_text(textwrap.dedent(body))
        return p

    def test_rejects_short_sha_bad_url_and_unknown_release(self) -> None:
        p = self.write("""\
            schema_version = 1
            [release.r]
            api_level = 37
            baseline = "a"
            fork_point = "abc"
            [[source]]
            name = "a"
            role = "canonical"
            url = "http://insecure.example"
            release = "r"
            ref = "refs/heads/x"
            commit = "abc"
            [[source]]
            name = "b"
            role = "reference"
            url = "https://example.invalid/b"
            release = "nope"
            ref = "heads/x"
            commit = "0000000000000000000000000000000000000000"
            """)
        with self.assertRaises(au.ToolError) as cm:
            au.load_sources(p)
        msg = str(cm.exception)
        for needle in ("commit must be a full SHA-1", "url must be https://", "unknown release",
                       "ref must be a full ref name", "fork_point must be a full SHA-1"):
            self.assertIn(needle, msg)

    def test_baseline_must_be_canonical(self) -> None:
        sha = "1" * 40
        p = self.write(f"""\
            schema_version = 1
            [release.r]
            api_level = 37
            baseline = "a"
            fork_point = "{sha}"
            [[source]]
            name = "a"
            role = "reference"
            url = "https://example.invalid/a"
            release = "r"
            ref = "refs/heads/x"
            commit = "{sha}"
            """)
        with self.assertRaisesRegex(au.ToolError, "baseline must be the canonical source"):
            au.load_sources(p)

    def test_unreadable_file_is_a_tool_error(self) -> None:
        with self.assertRaises(au.ToolError):
            au.load_sources(self.tmp / "missing.toml")


class RecordsValidation(GitEnv):
    SHA = "2" * 40

    def load(self, body: str) -> dict:
        p = self.tmp / "candidates.toml"
        p.write_text(textwrap.dedent(body))
        return au.load_records(p, {"src"})

    def rec(self, cid: str, decision: str, extra: str = "") -> str:
        sha = (cid.encode().hex() + "0" * 40)[:40]
        return textwrap.dedent(f"""\
            [[candidate]]
            id = "{cid}"
            source = "src"
            commit = "{sha}"
            subject = "s"
            author = "a"
            files = []
            categories = ["performance"]
            decision = "{decision}"
            rationale = "a reviewable rationale for this record"
            validation = "v"
            """) + extra + "\n"

    def test_reports_every_missing_key_and_keeps_checking(self) -> None:
        body = "schema_version = 1\n[[candidate]]\nid = \"x\"\n" + self.rec("y", "maybe")
        with self.assertRaises(au.ToolError) as cm:
            self.load(body)
        self.assertIn("missing source", str(cm.exception))
        self.assertIn("decision must be one of", str(cm.exception))  # second record still checked

    def test_series_must_list_exactly_the_adopted(self) -> None:
        body = "schema_version = 1\n[series]\norder = [\"b\"]\n" + self.rec("a", "adopt") + self.rec("b", "rejected")
        with self.assertRaises(au.ToolError) as cm:
            self.load(body)
        self.assertIn("b is not decision=adopt", str(cm.exception))
        self.assertIn("adopted candidates missing from order: ['a']", str(cm.exception))

    def test_duplicate_needs_known_target(self) -> None:
        body = "schema_version = 1\n" + self.rec("a", "duplicate", 'duplicate_of = "zz"')
        with self.assertRaisesRegex(au.ToolError, "duplicate_of unknown id zz"):
            self.load(body)

    def test_in_baseline_needs_full_sha(self) -> None:
        body = "schema_version = 1\n" + self.rec("a", "in_baseline", 'upstream_equivalent = "abc"')
        with self.assertRaisesRegex(au.ToolError, "upstream_equivalent"):
            self.load(body)

    def test_adopted_dependency_must_come_first(self) -> None:
        body = ("schema_version = 1\n[series]\norder = [\"b\", \"a\"]\n"
                + self.rec("a", "adopt") + self.rec("b", "adopt", 'depends_on = ["a"]'))
        with self.assertRaisesRegex(au.ToolError, "dependency a is not adopted earlier"):
            self.load(body)
        ok = ("schema_version = 1\n[series]\norder = [\"a\", \"b\"]\n"
              + self.rec("a", "adopt") + self.rec("b", "adopt", 'depends_on = ["a"]'))
        self.assertEqual(len(self.load(ok)["candidate"]), 2)

    def test_short_rationale_rejected(self) -> None:
        body = "schema_version = 1\n" + self.rec("a", "rejected").replace(
            "a reviewable rationale for this record", "too short")
        with self.assertRaisesRegex(au.ToolError, "rationale too short"):
            self.load(body)


class EndToEnd(Fixture):
    def test_fetch_analyze_and_patch_equivalence(self) -> None:
        au.fetch(self.work, au.load_sources(self.sources)[1])
        data, sources = au.load_sources(self.sources)
        rep = au.analyze(self.work, data, sources)["releases"]["rel"]
        self.assertEqual(rep["baseline_commit"], self.r1)
        sec = rep["sources"]["sec"]
        self.assertEqual(sec["merge_base"], self.mb)
        self.assertTrue(sec["merge_base_as_expected"])
        self.assertEqual(sec["baseline_only_commits"], 1)
        forka = rep["sources"]["forka"]
        self.assertEqual([c["commit"] for c in forka["fork_only_commits"]], [self.forka_fix, self.forka_own])
        self.assertIn(f"sec:{self.fix}", forka["fork_only_commits"][0]["same_patch_in"])
        self.assertEqual(forka["fork_only_commits"][1]["same_patch_in"], [])
        self.assertEqual(forka["fork_only_commits"][1]["author"], "Upstream Author")

    def test_check_passes_with_complete_records(self) -> None:
        self.write_records(["sec-fix"])
        self.assertEqual(self.run_tool("fetch", "--workdir", str(self.work))[0], 0)
        code, out, err = self.run_tool("check", "--workdir", str(self.work))
        self.assertEqual(code, 0, out + err)
        res = json.loads(out)
        self.assertEqual(res["problems"], [])
        self.assertEqual(res["releases"]["rel"]["series"], [{"commit": self.fix, "applies": True, "detail": ""}])

    def test_check_reports_unrecorded_commit(self) -> None:
        self.write_records(["sec-fix"], skip=("forkb-own",))
        self.run_tool("fetch", "--workdir", str(self.work))
        code, out, _ = self.run_tool("check", "--workdir", str(self.work))
        self.assertEqual(code, 1)
        self.assertTrue(any("has no decision record" in p for p in json.loads(out)["problems"]))

    def test_conflicting_series_fails_apply_check(self) -> None:
        self.write_records(["sec-fix", "forka-own", "forkb-own"],
                           decisions={"forka-own": "adopt", "forkb-own": "adopt"})
        self.run_tool("fetch", "--workdir", str(self.work))
        code, out, _ = self.run_tool("check", "--workdir", str(self.work))
        self.assertEqual(code, 1)
        res = json.loads(out)
        self.assertEqual([a["applies"] for a in res["releases"]["rel"]["series"]], [True, True, False])
        self.assertTrue(any("does not apply" in p for p in res["problems"]))
        self.assertEqual(sh(self.work, "worktree", "list").count("\n"), 0)  # throwaway worktree removed

    def test_wrong_duplicate_claim_detected(self) -> None:
        self.write_records(["sec-fix"])
        text = (self.records / "rel" / "candidates.toml").read_text()
        text = text.replace('decision = "deferred"', 'decision = "duplicate"\nduplicate_of = "sec-fix"')
        (self.records / "rel" / "candidates.toml").write_text(text)
        self.run_tool("fetch", "--workdir", str(self.work))
        code, out, _ = self.run_tool("check", "--workdir", str(self.work))
        self.assertEqual(code, 1)
        self.assertTrue(any("the patches differ" in p for p in json.loads(out)["problems"]))

    def test_pinned_commit_must_be_reachable(self) -> None:
        stray = self.tmp / "stray"
        sh(self.tmp, "clone", "-q", str(self.forka), str(stray))
        sha = commit_file(stray, "x", "x\n", "not upstream")
        self.sources.write_text(self.sources.read_text().replace(self.forka_own, sha))
        code, _, err = self.run_tool("fetch", "--workdir", str(self.work))
        self.assertEqual(code, 2)
        self.assertIn("forka", err)

    def test_refs_reports_drift(self) -> None:
        self.write_records(["sec-fix"])
        self.assertEqual(self.run_tool("refs")[0], 0)  # annotated tag is peeled
        commit_file(self.forkb, "runtime/e.cc", "e\n", "moved on")
        code, out, _ = self.run_tool("refs")
        self.assertEqual(code, 1)
        self.assertIn("moved to", out)


if __name__ == "__main__":
    unittest.main()
