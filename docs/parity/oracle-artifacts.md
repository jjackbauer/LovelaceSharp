# TEST-004 — the approved oracle artifacts and the amendment record

The contract (REQ-001, C4) requires every comparison to identify the frozen C# oracle, and TEST-004
requires the *amendments* to that oracle to be recorded rather than assumed. This document is that
record: the pinned sources, the approved artifacts with their SHA-256 and location, the inputs the
gates feed them, and the amendment log.

Pin metadata lives in `parity/oracle.json`; the artifact hashes below were measured on
2026-11-13 (UTC) by the commands in §6.

## 1. The pinned oracle

| Field | Value |
|---|---|
| Commit | `1cc28f255eca6029849f880949fc7be1d40ab8af` (branch `rust`) |
| Captured | 2026-11-10T14:39:48Z |
| Project | `Lovelace.Run/Lovelace.Run.csproj` |
| Build command | `dotnet build Lovelace.Run/Lovelace.Run.csproj -c Release` |
| Runner path | `Lovelace.Run/bin/Release/net10.0/Lovelace.Run.dll` |
| .NET SDK | `10.0.103` |
| Source directories | 36 (the list in `parity/oracle.json`) |
| Envelope | `{"ok":true,"protocolVersion":1,…}` for `--eval 1/3 --json` |

The freeze rule (C1) is that the C# sources are never modified. Today `git status --porcelain`
reports **0** modified `.cs` files, and `parity/build-oracle.ps1 -Verify` exits 0 with
`oracle: pinned sources clean (git diff --quiet and git status --porcelain are both empty)`.
The current HEAD (`bfaf8c10b9153b1ffa1eab9b6348988891460e4a`) differs from the pin; the verifier
checks only the 36 pinned paths, which is the property the contract needs.

## 2. Approved artifacts

| Artifact | Bytes | SHA-256 | Verified |
|---|---:|---|---|
| `Lovelace.Run/bin/Release/net10.0/Lovelace.Run.dll` | 275 968 | `a8e4ac385ee81edfc5717c39b29f7cd82fe68b181c958419b12bcb7faea7f808` | 2026-11-13T09:49:37Z by `parity/build-oracle.ps1` (build + verify, exit 0) |

The same hash is written by the gate run into `parity/build/rust-binaries.json` (`oracleDll.sha256`),
so the number in the CLI/fixture evidence file is the number in this table.

## 3. Inputs the oracle is compared through

The oracle is never compared "in general": every comparison has a named input. These are the
approved ones.

### 3.1 Golden protocol fixtures — `Lovelace.Run.Tests/fixtures`

20 fixtures, each an input (`.ls`) plus the oracle's recorded envelope (`.json`). The gates run
both sides on the `.ls` and compare the `.json`:

| Fixture | Input bytes | Input SHA-256 | Golden bytes | Golden SHA-256 |
|---|---:|---|---:|---|
| absent_field | 34 | `1569a9f956413aa04dd9572c21af6b821f904df23a040ca5db2553844bbc4f3e` | 6 017 | `f387697ae4005de5d2b542d3df6fbb2775b7a1960c87ba27c918f9ce03b5bcf5` |
| array | 51 | `89ea0fe5c5597cb2e6aa7e846fc7127562cda1e9dc2cfa4bb38cbbe7d08c7fde` | 3 683 | `ec0df96bddc565eb0c7c89ea32d201178aa5982967497d88ddd76801dd0fdf14` |
| capabilities | 15 | `f342dcfcce5e0a038e8df7cbdcb7ab6a8ec9c007157e10dd9ab51f2dc3d316fc` | 85 507 | `a35d9b325bc8f9994c5fb2a60836d1a16a034b14692bd5ef422d6ce70f35de70` |
| compilation | 44 | `b5b9969b951f80fd0135c63cdcf0e2495396b551127f5bdc153d6a411e9a313a` | 7 283 | `cf2e5a244e73234c3c21027ba6420a9e1bf2d63fff6f8a7258aee3150f85ed66` |
| complex | 15 | `5cedf69a1bf831ed6c937a4647e034e55faf312b975fdef0f5206a9aaf2947ec` | 1 975 | `0a8f543a321a2208bfbcc2a118ab9badb83f4e06e3e1d75e8bc7b30f0dff5bb9` |
| error_envelope | 45 | `f23a7b6cc4900f38d112af38388a77f87c8137804fb01d0cfad5f4c7678da67e` | 964 | `96d357ff294fee4b9ca099b8b1f3c9e56e61485e63bfd10639c9e2d0c1a70b16` |
| integration_result | 39 | `86de9bf5a96f9a51364576c58155daab8ba374843fb8003871db6ffa11aeebb9` | 5 222 | `9e142e9520fba32eeac975198fb8403292f79e08ac084355c5ff5c57d0f854aa` |
| limit_result | 44 | `c7c72b3311d6685599b4c4cf136759d6f85fb2ff9d8871d50d908a1050922649` | 5 756 | `e4f56799c18b572392b24cddfffed85b0a5cf9dad96c2347036d947c8c9a6264` |
| matrix_inverse_result | 43 | `d9f4d14fe1a2cba80ac5dba0e4f0ac1fbd63735352bd47f72e856ef7b3343949` | 7 815 | `a714992a09d2a5756eb91a5821a8e8523b46cbd5ad9d300ec56f4849335af15c` |
| matrix_solve_result | 56 | `e26858aad409dde625dcdbe565c45626d29833291d4fb79221240df17899f564` | 7 813 | `9ac74695e9f22093da364c9141002ad8cdf85ac1166a1771858991df7af75d8a` |
| nested_record | 44 | `f04f0d977c0c49646b1f83545bb484cd3e4d1671a16ea53ec712925959f9e70f` | 12 376 | `2bc81975828b2ffd4439979a73135a7c4756ab43e2e8cf8ac2ad3df5188c635c` |
| optimization_result | 50 | `ac6446ff3e93615139d07e1f2ebc503e61cb2181e36218341efbd579ab48a910` | 7 191 | `d0c0de62068647332d8220cdc5e0d7b1fcc801270601d7357b15e61ff4acc50c` |
| print_purity | 38 | `8db9583d4bc5723d844cacf971a7de00b672e54ca2ac49ef43e132395ca8c057` | 727 | `7c633bd4a1f4cbb0c3faf2513a6cc5573bab3348be890426d7b51c716add9fa1` |
| record | 45 | `3612569b3343bd321bb8ad2c7a9921de9c00a7ffd85f5731250f6ac0274bf17b` | 13 454 | `8545803228a8c6878a35e9dd2d33ee76ddf858cf5b0efdd21edb10f20ef1e456` |
| solve_nosolutions | 52 | `9bb635a390dc5c7635f0d6f8c88045607586cc01068eba6ff06c21b9b7fb9d03` | 10 925 | `3cfe3aa2ceca8e8f0f8ea445eb0398f73799869b6188c175d6846c2340a15957` |
| solve_result | 51 | `3e91acc3a9ed447bb00b5c5d66a18fcbd8e07356e00a8efd2d06ac469c60c583` | 18 524 | `9322d7c52ae17845aa861b7dccf82a08dfd78645e5bf01a987c53ca1c9af9bd6` |
| symbolic | 25 | `11bd52d6fc005e39736c352faf4add17deba5004fc2262be081c710e545ac0d1` | 1 399 | `26118f02c3f1a60bbf91606aa4f71ef0573c0d45455a6ab84daec1e99241cc82` |
| system_solve | 86 | `5d04bb667b38b3a01ab79d18a41fdfa7cb6f059f4d1d67b1758f1b92e30d3549` | 12 576 | `775673be3f6fa35c10450fa056624ef9e45fdc8303ecad1b75a0c28b13facb86` |
| transform_result | 52 | `4c873688c31bb8b6c64cd2daaedd1cad0aabcf5cb60accc59a1336fb89d18b01` | 10 562 | `71ed51b0cbe2e8b981ec604facd5282aadc8c7d01852fc17e44f45b11a93acfd` |
| vector | 27 | `ee2d74a73a440380f054eb4395580d16fced7512a5c182d26e1f6cd5b23921e7` | 2 769 | `0b9f4b2fa93364785e678fc5e6ba4e7ac214889bae00b61cdf9deae98e7327dc` |

### 3.2 CLI matrix and corpus inputs

| Input | Bytes | SHA-256 |
|---|---:|---|
| `parity/cli-matrix.json` (16 rows, RUN-001..014 plus the two compound rows) | 9 411 | `7461bb073438b4ef031f4ade09c999a5c127d06b2f5b604ffc4aaf522f817852` |
| `parity/corpus/lang.json` (214 documentation examples) | 23 035 | `2fc22adcd52e394223f85b319bf783012167405ed655890da06c925aab5ee1de` |
| `knowledge-graph-demo.json` (FMT-001 sample, seed 20240617) | 217 668 | `d445672bafad0af23ef4d5596144201bdf01ce3e961909eeaa8f18be50ef8f76` |

### 3.3 Ledger inputs a comparison depends on

| Ledger | Bytes | SHA-256 (2026-11-13) |
|---|---:|---|
| `parity/oracle.json` | 1 322 | `597b03c4bcf4fc930a224effb28d4a608816c0a3499b6a2a7fc5f072dbeb433f` |
| `parity/projects.toml` | 4 939 | `78aeaf3c37f43613d35a64428112d2955c13b0ec88a3fa668a9908edd92140ec` |
| `parity/census.json` | 3 998 378 | `3cb60e8cc20896999341f25f2f64ecb2584d780187816245844f96309786be02` |
| `parity/manifest.toml` | 4 330 659 | `48474a603629ea317a6971396493b346a2e4bacefd59ee559e8568892a1f72c9` |
| `parity/requirements.json` | 143 751 | `e0eeaa58da89348b38b34930d9f841ac49717f82031f0feaffe8a5f193aaa900` |

These move when other lanes edit the ledgers; the hashes record the state the 2026-11-13 evidence
run used, not a permanent pin.

## 4. Amendment log

An **amendment** is any change to what the oracle *is*: the pinned commit, the runner path, the
build command, the SDK, or the artifact bytes the gates invoke. Amendments that change the pinned
sources or the recorded goldens require the user's approval (REQ-001, TEST-003). Rebuilds of
unchanged sources are *artifact refreshes*: they are recorded, they do not need approval, and they
must be shown behaviour-preserving before the record is updated.

| ID | Date (UTC) | Kind | Change | Evidence | Approval |
|---|---|---|---|---|---|
| AMD-000 | 2026-11-10T14:39:48Z | baseline | `parity/oracle.json` pinned to `1cc28f2…`; first verification recorded artifact `f292249f…` at 14:53:27Z | `parity/build/oracle-verified.json` (overwritten by AMD-002, history in this table) | user (cycle 8 freeze) |
| AMD-001 | 2026-11-12T21:11Z (observed) | artifact refresh | on-disk Release DLL was rebuilt outside this lane, hash moved from `f292249f…` to `383527d8…` | `build-oracle.ps1 -Verify` exit 0 with pinned sources clean; fixtures oracle 20/20 and CLI matrix 16/16 against that DLL | not required — sources unchanged; recorded here because the hash moved silently |
| AMD-002 | 2026-11-13T09:49:37Z | artifact refresh + re-verification | this round rebuilt the oracle through `parity/build-oracle.ps1` (the pinned build command) and re-ran both gates; hash `a8e4ac38…` | build + verify exit 0; `CLI matrix: 16/16`; `oracle: 20/20`; `parity/build/oracle-verified.json` now carries `a8e4ac38…` | not required — sources unchanged, behaviour identical |

## 5. Why the hash is provenance, not identity

Three builds of the same pinned sources on this machine produced three hashes:
`f292249f…` (2026-11-10), `383527d8…` (2026-11-12), `a8e4ac38…` (2026-11-13). The .NET build is
not byte-reproducible here even though the source pin is checked by git, so:

* the oracle's **identity** is (pinned commit, project, runner path, SDK) plus its behaviour on the
  approved inputs;
* the **hash** identifies the exact binary a given evidence run used, and the gate writes it into
  its evidence file so a comparison can be traced to a binary;
* a hash mismatch is not by itself an amendment, and a matching hash is not by itself a freeze. The
  freeze check is `git diff --quiet` and `git status --porcelain` against the pinned commit, which
  `parity/build-oracle.ps1` performs before it does anything else.

The same lesson holds on the Rust side, where `-C link-arg=/Brepro` makes the bytes reproducible;
see [release-targets.md](release-targets.md) §4.

## 6. Re-verify

```powershell
pwsh -NoProfile -File parity/build-oracle.ps1 -Verify   # pinned sources clean + envelope + DLL hash
node parity/tools/gate-fixtures.mjs                     # oracle 20/20, rust 20/20
node parity/tools/gate-cli.mjs                          # CLI matrix 16/16 (both runners)
node parity/tools/ci-binaries.mjs --locked              # records oracleDll.sha256 in parity/build/rust-binaries.json
```

The script `parity/tools/verify-doc-citations.mjs` re-checks that every path cited in this document
still exists.
