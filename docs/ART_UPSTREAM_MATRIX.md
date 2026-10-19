# ART upstream matrix: android-17

Data as of 2026-10-19, from `art_upstream.py analyze` and `check` on the pins in
`upstream/sources.toml`. The authoritative, machine-checked records are in
`upstream/android-17/candidates.toml`; this page summarises them. The decision table below was
rendered from that file, and each rationale is in the file.

## Sources against the baseline `android-17.0.0_r1` (`b753cf9792`)

| Source | Pinned commit | Merge base with r1 | As expected | r1 commits it lacks | Fork-only commits |
|---|---|---|---|---|---|
| aosp-security | `ecd0dfa4cb` | `d9ee3abab2` (sibling branch) | yes (`expected_merge_base`) | 4 | 1 |
| graphene | `6a459adafb` | `b753cf9792` | yes | 0 | 19 |
| lineage | `747707d6ce` | `b753cf9792` | yes | 0 | 1 |
| crdroid | `2dff036e72` | `b753cf9792` | yes | 0 | 18 |

## Identical patches across sources (`git patch-id --stable`)

| Patch | Carried by |
|---|---|
| Make the runtime-generated app image read-only | aosp-security `70be2043e6`, graphene `0a490e74a7`, lineage `747707d6ce`, crdroid `747707d6ce` |
| Revert "Remove strip.keep_symbols from libart defaults" | graphene `f67f38168b`, crdroid `263fb6cfcf` |

## Decisions

| Source | adopt | duplicate | blocked | deferred | rejected | total |
|---|---|---|---|---|---|---|
| aosp-security | 1 | 0 | 0 | 0 | 0 | 1 |
| graphene | 1 | 1 | 6 | 3 | 8 | 19 |
| lineage | 0 | 1 | 0 | 0 | 0 | 1 |
| crdroid | 1 | 2 | 0 | 7 | 8 | 18 |
| **total** | **3** | **4** | **6** | **10** | **16** | **39** |

The adopted series, in order:
1. `aosp-security-70be2043e6`
2. `graphene-09613289ba`
3. `crdroid-f51611098f`

All three cherry-pick cleanly onto r1 (`check`, 2026-10-19). None has been built or run.

### What was not taken, in short

- **GrapheneOS:**
  - Its dynamic-code-loading restrictions, ART Service hooks and MTE signal option need
    GrapheneOS frameworks or bionic code (**blocked**).
  - Its dexopt policy and sandboxed-Play changes are ROM policy (**rejected**).
- **crDroid:**
  - Removes debug checks and tracing, or builds with `-O3`, without measurements (**rejected**).
  - CPU variant table additions need verified feature data, and its GC and codegen changes need
    tests and measurements (**deferred**).
- **LineageOS:** carries only the AOSP security fix.

## All records

| Record | Subject | Author | Decision | Categories |
|---|---|---|---|---|
| `aosp-security-70be2043e6` | Make the runtime-generated app image read-only | Nicolas Geoffray | adopt | security_hardening |
| `crdroid-f51611098f` | art: add check for __ARM_FEATURE_ATOMICS that indicates LSE atomics availability on target | XeonDead | adopt | runtime_correctness, architecture_abi |
| `graphene-09613289ba` | constify JNINativeMethod tables | Daniel Micay | adopt | security_hardening |
| `crdroid-747707d6ce` | Make the runtime-generated app image read-only | Nicolas Geoffray | duplicate of `aosp-security-70be2043e6` | security_hardening |
| `crdroid-263fb6cfcf` | Revert "Remove strip.keep_symbols from libart defaults" | Dmitry Muhomor | duplicate of `graphene-f67f38168b` | build_system, observability |
| `graphene-0a490e74a7` | Make the runtime-generated app image read-only | Nicolas Geoffray | duplicate of `aosp-security-70be2043e6` | security_hardening |
| `lineage-747707d6ce` | Make the runtime-generated app image read-only | Nicolas Geoffray | duplicate of `aosp-security-70be2043e6` | security_hardening |
| `graphene-544fee9b5b` | skip "speed"-optimized packages at an earlier dexopt stage | Dmitry Muhomor | blocked | art_service |
| `graphene-741eacd8bc` | report progress of the initial BackgroundDexoptJob to PackageManager | Dmitry Muhomor | blocked | art_service |
| `graphene-b1a63cc68e` | ask PackageManager for per-package compiler filter overrides | Dmitry Muhomor | blocked | art_service |
| `graphene-abe3c27eb4` | sigchainlib: add option to force default handling of MTE SEGV signals | Dmitry Muhomor | blocked | memory_safety |
| `graphene-7fe16a53d2` | add support for blocking InMemoryDexFile class loading API | Dmitry Muhomor | blocked | security_hardening, dex_verification |
| `graphene-2fd67903f1` | add support for blocking DexFile class loading API | Dmitry Muhomor | blocked | security_hardening, dex_verification |
| `crdroid-a868f723dd` | art: Add support for Cortex-A510 | Jake Weinstein | deferred | architecture_abi |
| `crdroid-e72f556642` | art: add kryo785 architecture variant. | Sungwook Jung | deferred | architecture_abi |
| `crdroid-ed9c4eecec` | art: Kryo785 supports dotprod | minaripenguin37 | deferred | architecture_abi |
| `crdroid-52138fedd5` | art: Add Kryo585 support | Raphiel Rollerscaperers | deferred | architecture_abi |
| `crdroid-d4a22d0a08` | optimized hprof speed | Guanyou Chen | deferred | observability |
| `crdroid-817f0d8c8b` | batch cmc sigbus prefetch | rmp22 | deferred | gc_memory, performance |
| `crdroid-6580aa54a2` | use ldp/stp for arm64 native call args | rmp22 | deferred | compiler_codegen, performance |
| `graphene-51c08d8fd7` | always start BackgroundDexoptJob at boot | Dmitry Muhomor | deferred | art_service, startup |
| `graphene-f67f38168b` | Revert "Remove strip.keep_symbols from libart defaults" | Dmitry Muhomor | deferred | build_system, observability |
| `graphene-13061d813a` | reduce log level of argv logging | Dmitry Muhomor | deferred | observability |
| `crdroid-a2fd5b9c84` | art: Disable debug tracing on production builds | Danny Lin | rejected | observability, performance |
| `crdroid-bb5646abdb` | Revert "Add visibility attributes in runtime/arch" | John Galt | rejected | build_system |
| `crdroid-f1218ce5ac` | ART: dex2oat and instruction_set_features: utilize modified runtime capabilities set | XeonDead | rejected | architecture_abi, jit_aot |
| `crdroid-3cdf637f8f` | art: Optimize the entirity of ART for performance | Vaisakh Murali | rejected | performance, build_system |
| `crdroid-5a6a39d5d0` | runtime: Disable kCheckDebugDisallowReadBarrierCount | rmp22 | rejected | runtime_correctness |
| `crdroid-0c1a852569` | gc: Disable dead objects poisoning on debug builds | rmp22 | rejected | gc_memory, memory_safety |
| `crdroid-07197785e3` | always use speed profile for large apps | rmp22 | rejected | rom_specific, art_service |
| `crdroid-2dff036e72` | art: Optimize IsPathologicalCase with unlikely hints | Igor Putko | rejected | performance |
| `graphene-55a4cbfd1c` | gmscompat: add support for loading DEX files from a file descriptor | Dmitry Muhomor | rejected | rom_specific |
| `graphene-97b9f0b7e0` | disable JIT profile saving | Dmitry Muhomor | rejected | rom_specific, jit_aot |
| `graphene-00b5d21ab1` | customize AOT compiler filter adjustment logic | Dmitry Muhomor | rejected | rom_specific, art_service |
| `graphene-7be9bb2105` | use speed-profile compilerFilter for pre-reboot dexopt of 3rd party apps | Dmitry Muhomor | rejected | rom_specific, art_service |
| `graphene-2c1741d06b` | raise concurrency from 1 to 2 for pre-reboot and background dexopt | Dmitry Muhomor | rejected | rom_specific, art_service |
| `graphene-131dd0fe4e` | allow GmsCompatLibrary code to access platform APIs | Dmitry Muhomor | rejected | rom_specific |
| `graphene-00f0ae3c98` | [temporary] disable asynchronous pre-reboot dexopt | Dmitry Muhomor | rejected | rom_specific, art_service |
| `graphene-6a459adafb` | fixup! gmscompat: add support for loading DEX files from a file descriptor | Daniel Micay | rejected | rom_specific |
