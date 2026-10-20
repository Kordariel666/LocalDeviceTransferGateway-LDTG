# P03b remediation report — F05–F08

Date: 2026-10-20. Branch: `dev`. Base: `0a3b9ce72d38ec1161678458273e5d08d916a8c6`.
The implementation and validation below were recorded before finalization.
On 2026-10-20 the owner authorized a scoped commit and push to `dev`, a manual
Windows Quality run, and a fast-forward of `main` only after successful CI.
No release, installation, firewall mutation, or alert dismissal is authorized.

## Changes and disposition

| Finding | Implementation | Regression evidence |
| --- | --- | --- |
| F05 | Failed rows expose the existing Cancel action. Cancellation aborts existing work and sends the existing session/CSRF-protected DELETE. An unconfirmed cleanup retains the server ID and a visible retry action; neither individual removal nor Clear finished discards that ID. Confirmed DELETE or 404 permits local removal. Failures before server creation need no DELETE. Late Create cleanup uses the same result handling; an interrupted worker cannot reattach a cleaned ID. | **PASS:** App tests cover offline DELETE, HTTP 500, delayed cleanup retry/204, already absent/404, removal without server creation, and delayed Create followed by cleanup/removal without chunk transmission. Reducer verifies retention after failed cleanup and removal after confirmation. Native Rust API regression creates four incomplete uploads, rejects their premature completion, observes the address limit, cancels one, creates another, and waits for physical partial deletion. |
| F06 | Aggregate percentage remains at most 99 while any selected row lacks successful server confirmation. Delivered byte totals remain unchanged and separately visible. Existing completed/cancelled row counts retain their queue-management meaning; cancelled files do not qualify the batch for 100 percent. | **PASS:** delayed Complete shows 99 percent and 0/1 finished, followed by 100 only after successful response. Failed Complete remains below 100. Reducer boundary cases cover uploading, finalizing, failed, paused, queued and cancelled states, empty files and mixed batches with a completed file. |
| F07 | Shared byte-unit index is clamped at zero. Positive sub-byte rates have one decimal; rates below 0.1 B/s display `< 0,1 B/s` instead of rounding to zero. Integer sizes and larger units retain their formatting. | **PASS:** shared presentation tests through the desktop consumer cover zero, negative, NaN, Infinity, smallest positive number, 0.099/0.1/0.5/0.99/1, 1023/1024, and TiB boundaries, plus direct fractional-byte formatting. Both UIs import the same shared implementation. |
| F08 | Active threat model now names its current dev base and local P03b delta, native COM firewall configuration, elevated self-executable, separate uninstall cleanup helper, current IPC consumer and P03a receipt/accounting behavior. Stale line coordinates are replaced with file/symbol references. Public-beta status points to the dated publication decision and separates historical device evidence from current validation. | **PASS:** source comparison against platform functions, NSIS hooks, firewall helper, IPC registration, upload API/state and repository publication/security documents; Markdown link check. Historical handoff is explicitly identified as an older snapshot; its contents are untouched. |

Production edits are confined to `apps/mobile/src/App.tsx`,
`UploadQueueView.tsx`, `uploadQueue.ts`, `i18n.ts`, and
`packages/shared/src/presentation.ts`. Regression additions are in the existing
mobile App/queue tests, desktop presentation tests, and Rust API tests.
Documentation changes are this report and `docs/THREAT_MODEL.md`.
No Rust production code, API DTO, dependency, lockfile, workflow, security
control, or transfer protocol was changed. Existing finalization guards,
ownership, CSRF, IP/session binding, containment, No-Replace publication and
resource limits remain in place.

## Validation

Pinned tools: Node **24.19.0**, pnpm **11.19.0**, Rust **1.98.0**.
The existing ignored `artifacts/p03-toolchain/pnpm.cmd` was prepended to PATH
so nested pnpm commands also use 11.19.0; the global installation was not changed.

- **PASS:** targeted mobile run, 49 tests before the final additional
  no-server-object removal case (`artifacts/p03b-mobile-targeted.log`).
- **PASS:** targeted desktop run, 50 tests
  (`artifacts/p03b-desktop-targeted.log`).
- **PASS:** final Rust suite, 131/131, none ignored
  (`artifacts/p03b-rust-targeted-final.log`), including the four-slot regression.
- **PASS:** complete pinned `pnpm check`, exit 0
  (`artifacts/p03b-check-final.log`): generated contracts, notices, Markdown
  links, TypeScript, ESLint, 16 script tests, desktop 50 and mobile 50 coverage
  tests, both web builds, 131 Rust tests, rustfmt and all-target Clippy with
  `-D warnings`. This run includes the final no-server-object removal test.
- **PASS:** `git diff --check`; branch remains `dev`, HEAD remains the base above.
  Final report/model edits were followed by another Markdown link check.

Finalization review: the complete code/test diff matches F05–F08 and this report.
Two active-model links to untracked local audit/handoff documents were converted
to explicitly local path references, so the committed documentation does not
depend on files deliberately excluded from the commit. The staged snapshot's
Markdown links were checked separately from the working tree. Only the ten
reviewed modified files and this report are included in the finalization commit.
CI and branch synchronization results are reported in the final chat handoff.

Initial test attempts exposed Windows stdin/text encoding in inserted test
strings and an out-of-subnet IP in the new Rust fixture. Both were corrected;
the final runs above pass without relaxing production controls.

## Preserved evidence and limitations

The untracked `docs/analysis/` handoff and all existing P02 audit documents
remain untouched and untracked. Historical handoff SHA-256 before and after:
`A876F2078E9B6CCCA9A304FCB48F2F1B625BE0CD4700DB721BFF977F4A6E5625`.
Historical security archive and the P03a report are unchanged.

- Cleanup confirmation means the server accepted cancellation, or the ID is
  no longer accessible in the current session. Physical deletion remains the
  existing asynchronous best-effort operation. The native test proves deletion
  on a functioning temporary filesystem, not guaranteed deletion under arbitrary
  filesystem/driver failures. HTTP 500/offline failure tests do not simulate
  physical disk failure.
- Offline/unconfirmed cleanup remains visible and requires the user to retry
  with connectivity. No persistent background cleanup queue was introduced.
  Closing/reloading the page loses the in-memory queue; existing server idle,
  session-expiry/logout and shutdown cleanup policies still apply. A 404 is not
  proof that a previously published file was removed; cancellation never removes
  published files. Finalizing/completed uploads remain non-cancellable.
- Percent progress reserves 100 for an entirely successful selected batch;
  a cancelled row may keep the batch below 100 until removed. Byte counters
  continue to describe transferred bytes and do not prove durable completion.
- **NOT_TESTED:** new real-device/browser visual acceptance, physical WLAN
  interruption, UAC/install/uninstall, packaging, disk-full/driver fault injection,
  GitHub/alert state, or release readiness. Frontend evidence uses jsdom/mocked
  requests; backend evidence uses native Windows temporary files/in-process API.
- R01 mutable-source Range resume and the P03a limitations remain unchanged.
