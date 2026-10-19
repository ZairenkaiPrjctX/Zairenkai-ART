# ART patch porting policy

How a commit from a fork or an AOSP side branch becomes, or does not become, part of
Zairenkai-ART's series. The machine-checked form is `upstream/<release>/candidates.toml`; its
schema is enforced by `tools/art_upstream.py` (`load_records`, `check`).

## Record fields

| Field | Required | Meaning |
|---|---|---|
| `id` | yes | `<source>-<first 10 hex digits>`, unique |
| `source`, `commit` | yes | Source name from `sources.toml` and the full 40-hex commit |
| `subject`, `author`, `date`, `files` | yes (`date` optional) | Copied from git by `analyze`. `author` is the upstream author, never Zairenkai. |
| `categories` | yes | One or more of the categories below |
| `decision` | yes | `in_baseline`, `adopt`, `duplicate`, `deferred`, `rejected`, `blocked` |
| `rationale` | yes, ≥ 20 characters | Why, in terms a reviewer can check against the diff |
| `validation` | yes | What was actually done: `diff reviewed; not applied`, `apply-check only…`, or the build and test evidence |
| `upstream_equivalent` | for `in_baseline` | Full SHA of the identical patch in the baseline |
| `duplicate_of` | for `duplicate` | Record id with an identical `git patch-id --stable` |
| `depends_on` | optional | Record ids. An adopted record's dependencies must be adopted earlier in the series. |
| `notes` | optional | Caveats, such as the symlink artefact in `graphene-09613289ba` |

**Categories:**
- runtime and code: `security_hardening`, `runtime_correctness`, `memory_safety`,
  `compiler_codegen`, `jit_aot`, `interpreter`, `gc_memory`, `dex_verification`, `startup`;
- tooling and platform: `art_service`, `build_system`, `architecture_abi`, `performance`,
  `observability`;
- scope: `rom_specific`, `obsolete`.

## Decisions

| Decision | Use when |
|---|---|
| `in_baseline` | The same patch (same patch id) is already in the baseline |
| `duplicate` | Another record carries the identical patch. The earliest or most authoritative source is the one recorded (AOSP over forks). |
| `adopt` | All adoption criteria below hold |
| `blocked` | The change depends on code outside ART that AOSP does not have (a ROM's frameworks/base API, a bionic extension, a framework package) |
| `deferred` | Plausible value, but evidence is missing (measurement, tests, hardware documentation), or the risk is high |
| `rejected` | Policy choices for one ROM; removes checks or diagnostics; unmeasured performance claims; behaviour AOSP already exposes as configuration |

## Adoption criteria

All of these must hold:

1. **Demonstrated value**, of one of these kinds:
   - a security fix or hardening with no behaviour change;
   - a correctness fix for a reproducible defect;
   - a performance change with a measurement procedure and results.
2. **Self-contained in ART.** No dependency on non-AOSP code.
3. **Nothing weakened.** No verification, security check or debug check is disabled. Logging
   or tracing is not removed for unmeasured gains.
4. **Applies cleanly** to the baseline after the earlier series entries (`check`).
5. **Provenance preserved.** Apply with `git cherry-pick -x`, keep the author and the existing
   `Signed-off-by` lines, and never squash, re-author or reword.

Before an adopted patch is called *validated*, it must also be built, and the tests in
`ART_BUILD_GUIDE.md` §4 must pass, with the logs referenced in `validation`. Until then the
status is `apply-check only`, and the compatibility matrix says BLOCKED_BY_ENVIRONMENT.

## Modifying a patch

Upstream commits are adopted unchanged. If a fix is needed (for example restoring the symlink
that `graphene-09613289ba` replaced):
- add it as a separate commit authored by the person who wrote it;
- reference the original;
- keep the original commit untouched in the series.

## Never

- Merge a fork branch wholesale.
- Adopt a commit without a record, or a record without reading the full diff.
- Describe an imported patch as authored by Zairenkai.
- Mark a patch tested when it was only applied or compiled.
