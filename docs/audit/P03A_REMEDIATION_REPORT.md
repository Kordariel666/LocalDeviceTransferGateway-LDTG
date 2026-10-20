# P03a remediation report - F01 through F04

Date: 2026-10-20. Branch: `dev`. Base/unchanged HEAD:
`fd57da4422804f9da25eb6c281c80bbd86871ac7`.
Changes are local and uncommitted. No push, merge, release, installation, firewall
mutation, or alert dismissal was performed. Existing P02 audit documents and the
historical technical handoff were preserved.

## Fixes and evidence

| Finding | Implementation | Deterministic regression / result |
| --- | --- | --- |
| F01 | Source-only metadata preflight precedes frozen pnpm installation and locked Cargo fetch; full installed notice validation follows both bootstraps. SHA and clean-tree checks remain first. Source-only mode cannot emit release artifacts. | **PASS**: four real PowerShell control-flow tests with mocked external commands cover cold ordering, incorrect SHA, dirty tree, and install failure. Real SHA/dirty-tree entrypoint refusals and source-only artifact refusal each returned the expected exit 1. |
| F02 | Create checks receipts and active tokens under serialization, before global/per-address capacity and I/O permits. Finalization keeps the state lock from active removal through receipt insertion. Status/Complete check the receipt and active record under that same state lock. | **PASS**: gated publication with a waiting Create returns the original ID/full offset and creates no extra partial. Retry works at 64 active records and exhausted per-address I/O capacity. A separate gate pauses the active-to-receipt transition; Status and Complete wait and then succeed rather than returning unknown. New Creates still fail at capacity. |
| F03 | Correct token plus matching sanitized metadata and a currently authenticated, CSRF-validated Create grants receipt access to that exact recovery session. Existing original-address receipt access remains unchanged; active-upload ownership remains session-bound. | **PASS**: expanded existing receipt regression exercises a fresh session/new IP, subsequent Status and Complete, identical saved name, and exactly one published file. Wrong token and failed CSRF do not grant access; another session at the new IP is denied; the old cookie used from the new IP remains unauthorized. |
| F04 | A separate async publication lock covers publication through accounting/receipt insertion and covers Create's scan/reservation path. Lock order is publication then state. The state/cancellation lock is not held across finalization filesystem I/O. Cancellation cleanup keeps publication serialization until its attempted deletion completes. | **PASS**: post-rename gate blocks a competing scan; after commit, totals are exactly `(completed files=1, bytes=4, active files=1, bytes=0)`. A second upload can consume the remaining byte budget and completes at exactly `(2,8,0,0)`. Cancel, shutdown cleanup, and final-size failure regressions preserve reservations/release semantics. |

The publication and receipt gates exist only in test builds. Work remains in the
existing service-owned async/blocking jobs with their I/O and chunk permits.
No filesystem containment, handle validation, No-Replace publication, request
session/CSRF checking, or configured limit was removed.

## Changed files

- `scripts/private-release.ps1`: split source preflight from installed validation.
- `scripts/release-metadata.mjs`: source-only mode; prohibit source-only artifact output.
- `scripts/private-release.test.mjs` (new): four release-order/safety regressions;
  automatically included by the existing `test:scripts` glob.
- `src-tauri/src/service/api/upload.rs`: idempotency ordering, synchronized receipt
  lookup/transition, publication serialization, test gates.
- `src-tauri/src/service/state.rs`: publication lock, bounded recovery-session
  receipt field, test-only gates.
- `src-tauri/src/service/state/uploads.rs`: session-specific receipt recovery;
  test-only exact usage snapshot accessor.
- `src-tauri/src/service/api/tests.rs`: six new regression tests and expanded
  existing recovery test, plus local test helpers.
- `docs/PRIVATE_RELEASE.md`: bootstrap prerequisites and `-ValidateOnly` behavior.
- `docs/audit/P03A_REMEDIATION_REPORT.md`: this disposition.

No API DTO, frontend, dependency version, lockfile, workflow, or product feature
was changed. The original handoff SHA256 remains
`A876F2078E9B6CCCA9A304FCB48F2F1B625BE0CD4700DB721BFF977F4A6E5625`.

## Validation

Toolchain: Node **24.19.0**, pnpm **11.19.0**, Rust **1.98.0**. The pinned pnpm
package was unpacked under ignored `artifacts/p03-toolchain`; the global pnpm
11.25.0 installation was not changed.

- **PASS**: targeted Rust run (`artifacts/p03-rust-targeted-final.log`),
  130/130 tests, no ignored tests.
- **PASS**: full pnpm check (`artifacts/p03-check-final.log`), exit 0:
  generated contracts, 344 notice components/238 notice texts, Markdown links,
  TypeScript, ESLint, 16 script tests, Desktop 36 and Mobile 39 coverage tests,
  both web builds, all 130 Rust tests, rustfmt, Clippy all-targets with `-D warnings`.
- **PASS**: `git diff --check`; branch and HEAD unchanged.
- **PASS (expected rejections)**: wrong SHA (`artifacts/p03-wrong-sha.log`),
  dirty tree (`artifacts/p03-dirty-tree.log`), and
  source-only artifact generation (`artifacts/p03-source-artifact-rejected.log`).

An initial cancellation test expected physical deletion immediately after the
Complete response. Windows can retain a delete-pending directory entry until
scheduled cleanup closes its last handle. The regression now waits, with a
bounded deadline and cooperative yields, for that cleanup before asserting the
physical directory and rescanned counters. Logical cancellation is asserted
while publication I/O is still gated. No production workaround was added for
this test timing issue. Final targeted and full runs pass.

## Isolated cold bootstrap and packaging boundary

Created a fresh local clone on `dev` at `artifacts/p03-cold` without node_modules,
then copied only the two candidate release scripts. Used a separate initially
empty `CARGO_HOME` (`artifacts/p03-cold-cargo`) and pnpm store
(`artifacts/p03-cold-store`). Successfully executed, in order:

1. `cargo metadata --manifest-path src-tauri/Cargo.toml --format-version 1 --no-deps --locked`.
2. `node scripts/release-metadata.mjs --check --source-only --json`.
3. `pnpm install --frozen-lockfile --store-dir ../p03-cold-store`.
4. `cargo fetch --manifest-path src-tauri/Cargo.toml --locked`.
5. `node scripts/release-metadata.mjs --check --json` (including offline Cargo
   metadata and full installed notice checks).

Evidence: preflight metadata (`artifacts/p03-cold-preflight.json`),
pnpm bootstrap (`artifacts/p03-cold-pnpm.log`),
Cargo bootstrap (`artifacts/p03-cold-cargo.log`),
full metadata (`artifacts/p03-cold-metadata.json`).
Both lockfile hashes and the third-party notices hash stayed identical.

**PARTIAL release verification:** this proves a cold dependency environment,
not a complete pristine-runner release. Copying the uncommitted candidate scripts
makes the clone dirty. The canonical script correctly rejects that state, so its
external command sequence was validated separately with deterministic mocks and
with the real bootstrap commands above. No commit was made to bypass this gate.

**NOT_TESTED:** full candidate NSIS packaging, fresh release-binary timestamps,
installer contents/install/uninstall, UAC cancellation, installer hashmanifest,
code signing/AV/SmartScreen, and a GitHub Windows-runner execution. No new release
readiness or real-device approval is claimed.

## Remaining risks and recommendation choices

- F01 follows the audit's split-preflight option rather than adding a CI-only
  bootstrap. `-ValidateOnly` now installs/fetches and verifies notices before
  returning; it writes a log and requires a fresh output directory.
- F03 uses the audit's session-bound receipt permission option, with no API
  contract change. Only the latest token-proven recovery session is retained per
  receipt, keeping storage bounded; another recovery can replace that session's
  receipt grant. The original-address policy is preserved. A receipt remains
  ephemeral (24-hour TTL, 256-entry bound); recovery after expiry/restart is not
  added. Token knowledge plus valid authentication remains required to recover
  from another address.
- F04 uses consistent serialization rather than reconstructing quota counts from
  pending filenames. This serializes scans and finalization and may delay new
  Creates during slow publication. It does not make cancel/stop acquire the
  publication lock. Existing Create preparation/state-lock behavior is retained;
  arbitrary filesystem/driver stalls outside the gated finalization paths are
  not certified by these tests.
- Local external file mutations and physical disk/driver failures remain outside
  transactional in-memory accounting guarantees. No persistent journal or crash
  recovery was introduced; delete failures remain best-effort as before.
- Real WLAN/IP changes, multiple physical clients, near-limit large files, slow/full
  volumes, and real UI/device acceptance remain untested. The new evidence uses
  native Windows temporary files and in-process API requests with controlled IPs.
- F05-F08 and R01 were intentionally left outside this authorized batch. The P02
  audit and historical handoff were not rewritten as a new security assessment.
