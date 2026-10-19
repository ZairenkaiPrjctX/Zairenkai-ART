# Building ART with the adopted series

**Status: BLOCKED_BY_ENVIRONMENT.** Nothing in this guide has been executed by this project.
The steps come from ART's own `build/README.md` at `android-17.0.0_r1`. Building needs a
`master-art` or full platform checkout, which was not available. Until someone runs these steps
and records the result, every adopted patch is `validation = "apply-check only"`.

## 1. Check out sources

ART's README describes two ways to build.

- **Module build (recommended upstream).** Uses the `master-art` manifest:
  ```sh
  repo init -b master-art -u https://android.googlesource.com/platform/manifest
  repo sync
  ```
  `master-art` follows ART development, not `android-17.0.0_r1`. Building r1 plus the series
  this way needs a matching manifest. Which manifest that is has not been verified.
- **Platform build.** Use a full tree at the release tag:
  ```sh
  repo init -b android-17.0.0_r1 -u https://android.googlesource.com/platform/manifest
  repo sync
  ```

## 2. Apply the series in `art/`

The order comes from `upstream/android-17/candidates.toml` (`[series] order`). Use
`cherry-pick -x` so every commit keeps its original author and records where it came from.

```sh
cd art
git checkout -b zairenkai-art android-17.0.0_r1   # or the tree's checked-out r1 commit
git fetch https://android.googlesource.com/platform/art refs/heads/android17-security-release
git cherry-pick -x 70be2043e6dbcd080b82022a3dee030de7caad80
git fetch https://github.com/GrapheneOS/platform_art refs/heads/17
git cherry-pick -x 09613289ba14f60d59a17ff4cad31d1d5dca5c7c
git fetch https://github.com/crdroidandroid/android_art refs/heads/17.0
git cherry-pick -x f51611098fea879d29fbdbd279f6b0a98418f0e8
```

`art_upstream.py check` runs the same three cherry-picks and verifies that they apply cleanly.

## 3. Build

Commands from ART `build/README.md` (r1):

- **Module (`master-art`):**
  ```sh
  source build/envsetup.sh
  banchan com.android.art <arch>
  m apps_only dist
  ```
- **Platform:**
  - `lunch <product>-trunk_staging-<variant>`, then `m droid`.
  - Check `get_build_var RELEASE_APEX_CONTRIBUTIONS_ART` first. A non-empty value usually
    means the release configuration uses **prebuilt** ART, and a source change would not be
    built at all.

## 4. Tests to run before changing `validation`

| Patch | Minimum evidence |
|---|---|
| `70be2043e6` app image read-only | `atest art_standalone_runtime_tests` (named in the commit's `Test:` line) |
| `09613289ba` constify JNINativeMethod | Full ART build with `-Werror`, plus run-tests `004-JniTest`, `139-register-natives`, `2263-method-trace-jit` |
| `f51611098f` LSE detection | `art_standalone_runtime_tests` (instruction set features tests) on an arm64 build. Confirm no "Mismatch between dex2oat instruction set features" message for an ARMv8.2+ target. |

ART's test runner is `test/testrunner/testrunner.py`, and `tools/run-gtests.sh` runs the gtests.
Both exist at r1; their flags are documented in the ART tree.

## 5. Never

- Never install the result on a production device.
- Never ship `libart.so`, `dex2oat`, boot images, `.odex`, `.vdex` or `.art` files outside a
  signed ART APEX or a full ROM build.
- Never disable APEX signature checks or `-Xverify` to make a build run.
