# Upstream update policy

## Principles

- **Pins are explicit.** A pin is a full commit plus the ref it was taken from, held in
  `upstream/sources.toml`. Tools never move a pin; a person changes it in a reviewed pull request.
- **One canonical baseline per release line.** It is an AOSP release tag. Forks and AOSP side
  branches are `reference` sources: they are analysed and never merged wholesale.
- **No silent gaps.** Every fork-only commit of every pinned reference source has a decision
  record. `art_upstream.py check` fails otherwise.

## When to update

| Trigger | Action |
|---|---|
| The weekly CI job `upstream-drift` (`refs`) reports a moved ref | Open an update pull request (procedure below), or record why it waits |
| New AOSP security bulletin, or a new commit on `android17-security-release` | Same day: analyse it. Security fixes are reviewed first. |
| New AOSP tag on the same release line (`android-17.0.0_r2`, …) | Re-baseline: move the `aosp` pin and `fork_point` only after the forks have rebased onto it, or record the mismatch in `expected_merge_base` |
| New Android release line (e.g. android-18) | Add a `[release.<name>]` table, its sources and an `upstream/<release>/candidates.toml`. Keep the old line until it is retired explicitly. |

## Procedure for an update pull request

1. `python3 tools/art_upstream.py refs`. Note every moved ref.
2. Edit `upstream/sources.toml`: new `commit` values only, with the ref unchanged unless a
   branch was renamed.
3. Fetch and analyze:
   ```sh
   python3 tools/art_upstream.py fetch   --workdir W
   python3 tools/art_upstream.py analyze --workdir W
   ```
4. For each new fork-only commit:
   - read the full diff;
   - add a record to `candidates.toml` following `ART_PATCH_PORTING_POLICY.md`;
   - remove records for commits that are no longer fork-only, for example because they were
     upstreamed. `check` lists them as stale.
5. `python3 tools/art_upstream.py check --workdir W`. It must report `"problems": []`.
6. In the pull request, state:
   - the old and new pins;
   - the new records and their decisions;
   - whether the adopted series still applies;
   - that ART was **not** built, unless it was. If it was, attach the build and test logs.

## Review requirements

- Every change under `upstream/` needs human review. ART is runtime code; per `AGENTS.md`,
  high-risk changes always need a human reviewer.
- A record changing to `adopt` needs a reviewer other than its author, plus the evidence listed
  in `ART_PATCH_PORTING_POLICY.md`.

## Retiring a release line

Delete its sources and records in one pull request, which states the reason (end of support).
Git history keeps the decisions.
