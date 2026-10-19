# Upstream ART

## Canonical source

**AOSP `platform/art`**, `https://android.googlesource.com/platform/art`.
- The baseline is the tag `android-17.0.0_r1` (commit `b753cf97923c3695338d21466fa14c57b480a59a`).
- Annotated tags are peeled to their commit.
- Licence: Apache-2.0 (`NOTICE`, `MODULE_LICENSE_APACHE2`).

The full list of sources, refs and commits is in `upstream/sources.toml`. That file is the only
place pins are defined; documentation quotes it.

| Name | Role | Ref | Pinned commit |
|---|---|---|---|
| `aosp` | canonical (baseline) | `refs/tags/android-17.0.0_r1` | `b753cf97923c` |
| `aosp-security` | reference | `refs/heads/android17-security-release` | `ecd0dfa4cb72` |
| `graphene` | reference | `refs/heads/17` | `6a459adafb78` |
| `lineage` | reference | `refs/heads/lineage-24.0` | `747707d6ce1c` |
| `crdroid` | reference | `refs/heads/17.0` | `2dff036e7261` |

## Why r1, not the security branch

`android17-security-release` is not r1 plus fixes.
- It branched from `d9ee3abab2` before r1, and so lacks four r1 commits.
- `expected_merge_base` in `sources.toml` records this, so the tool reports it as expected
  instead of as drift.
- Its one extra commit (`70be2043e6`) is adopted as the first patch of the series.

## Using the tool

```sh
python3 tools/art_upstream.py validate                      # offline: sources and records
python3 tools/art_upstream.py refs                          # live refs vs pins (exit 1 on drift)
python3 tools/art_upstream.py fetch   --workdir /path/work  # partial clone of the pinned commits
python3 tools/art_upstream.py analyze --workdir /path/work  # JSON: merge bases, fork-only commits
python3 tools/art_upstream.py check   --workdir /path/work  # records complete + series applies
```

- **fetch:**
  - creates a bare repository with `blob:none` partial-clone filters, so blobs are fetched
    lazily;
  - stores each pin at `refs/zart/<name>` and the fetched tip at `refs/zart/<name>-tip`;
  - fails if a pinned commit is not reachable from its ref.
  - The workdir took about 100 MB, and a fresh fetch plus check took 2 min 13 s on 2026-10-19.
- **analyze:**
  - lists, for each reference source, the commits between the fork point and the source's pin,
    with author, date, files and `git patch-id --stable`;
  - marks commits that are already in the baseline (`in_baseline_as`);
  - marks patches carried by several sources (`same_patch_in`).
- **check:** fails when
  - a fork-only commit has no record;
  - a record names a commit that is not fork-only;
  - a `duplicate` or `in_baseline` claim does not match the patch ids;
  - a merge base is not the expected one;
  - the adopted series does not cherry-pick cleanly onto the baseline.

  The cherry-pick happens in a temporary sparse worktree, which is removed afterwards.

The tool never pushes, never edits `sources.toml` and never builds ART.

## Results on 2026-10-19

The full matrix is in `ART_UPSTREAM_MATRIX.md`.

```
$ python3 tools/art_upstream.py check --workdir …/artwork
{ "releases": { "android-17": { "series": [
    { "commit": "70be2043e6dbcd080b82022a3dee030de7caad80", "applies": true, ... },
    { "commit": "09613289ba14f60d59a17ff4cad31d1d5dca5c7c", "applies": true, ... },
    { "commit": "f51611098fea879d29fbdbd279f6b0a98418f0e8", "applies": true, ... } ] } },
  "problems": [] }
```
