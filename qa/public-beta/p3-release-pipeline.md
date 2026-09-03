# P3 – Releasepipeline und Herkunftsnachweis

Stand: 3. September 2026  
Basisrevision vor den lokalen Änderungen: `950e4301a61bbef79d4ecca3ed81b646baa356ca`  
Paketstatus: **technisch vorbereitet; kanonischer Clean-Commit-Dry-Run offen**

Dieser Nachweis betrifft nur lokale und private Vorbereitung. Es erfolgten kein
GitHub-Actions-Lauf, keine Veröffentlichung, keine Lizenzaktivierung, keine
Anmeldung, keine Signierung und keine kostenpflichtige Maßnahme.

## Implementierte Gates

- `.node-version`, `packageManager` und `rust-toolchain.toml` fixieren Node
  `24.19.0`, pnpm `11.19.0` und Rust `1.98.0`.
- `scripts/release-metadata.mjs` gleicht Root- und Workspaceversionen, Cargo,
  Tauri und Changelog ab. Es verlangt eingecheckte Lockfiles, passende
  Audit-/SBOM-Hashes, null unbekannte Lizenzen, null Lizenzblocker und
  vollständige Action-Commit-Hashes.
- `scripts/private-release.ps1` akzeptiert ausschließlich einen sauberen
  Arbeitsbaum am erwarteten vollständigen Commit. Erst nach Lockfileinstallation
  und Cargo-Fetch läuft das vollständige Qualitätsgate; erst danach wird NSIS
  gebaut. Eine nachträgliche Quelländerung stoppt den Lauf.
- Der Ausgabeordner wird nicht überschrieben. Ein Erfolg bündelt Installer,
  commitgebundene CycloneDX-SBOM, Buildlog, Manifest und SHA-256-Prüfsummen.
- Der manuelle private Workflow besitzt nur Leserechte, keine Secrets und keinen
  Release- oder Signierschritt. Fremd-Actions sind an volle Commit-Hashes
  gebunden; das private CI-Artefakt wäre auf sieben Tage begrenzt.
- `CODE_SIGNING.md` hält Build, manuelle Freigabe, spätere Signierung und
  Veröffentlichung als vier getrennte Grenzen fest, ohne SignPath zu starten.

## Lokale Prüfung

`pnpm release:check` bestand am 3. September 2026 mit:

- vier npm-Manifeste, Cargo, Tauri und Changelog auf `0.3.0-rc.1`;
- `pnpm-lock.yaml` SHA-256
  `95af3687f770be22de3aff4693c25c7f8de5f7882a500726a298b77186fcdaa3`;
- `src-tauri/Cargo.lock` SHA-256
  `a5eb92fc2dff06c8ca035f290345e731825fe38072a8ed4831cf32bc345856e4`;
- 857 SBOM-Komponenten und keinem Audit-/Lizenzblocker;
- sechs unveränderlich fixierten Action-Verwendungen;
- Node `24.19.0`, pnpm `11.19.0`, rustc und Cargo `1.98.0`.

Anschließend bestand auch das vollständige `pnpm check`: 36 Desktoptests,
39 Mobiletests und 122 Rusttests sowie Vertragsprüfung, Typprüfung, ESLint,
Coverage, beide Produktions-Webbuilds, Rustfmt und Clippy waren grün.

Der PowerShell-Parser akzeptierte `scripts/private-release.ps1`. Ein negativer
Validierungslauf am absichtlich noch veränderten Arbeitsbaum brach vor
Installation und Paketierung mit „nur einen sauberen Arbeitsbaum“ ab. Damit ist
das wichtigste Fail-closed-Eingangsgate belegt.

Die verifizierten Primärquellen zu den Action-Revisionen und zum Node-Build sind:

- <https://github.com/actions/checkout/commit/9c091bb21b7c1c1d1991bb908d89e4e9dddfe3e0>
- <https://github.com/actions/upload-artifact/commit/043fb46d1a93c77aae656e7c1c64a875d1fc6a0a>
- <https://nodejs.org/download/release/v24.19.0/>

## Verbleibendes P3-Gate

Ein voller kanonischer Dry-Run kann erst nach einem lokalen Commit ausgeführt
werden, weil derselbe Schutzmechanismus den derzeit beabsichtigt veränderten
Arbeitsbaum korrekt zurückweist. Dieser Lauf muss noch den frisch erzeugten
Installer, dessen `NotSigned`-Status, alle vier SHA-256-Einträge, die
commitgebundene SBOM und das Buildmanifest bestätigen. Der private
GitHub-Workflow muss dafür nicht ausgeführt werden.

Der Owner hat die dafür nötige Berechtigung mit `PB-05` am 3. September 2026
erteilt: genau ein lokaler Commit für die geprüften P2-/P3-Änderungen und danach
der kostenfreie lokale Clean-Commit-Dry-Run. Die Berechtigung allein gilt noch
nicht als bestandener Lauf.
