# P3 – Releasepipeline und Herkunftsnachweis

Stand: 3. September 2026  
Basisrevision vor den lokalen Änderungen: `950e4301a61bbef79d4ecca3ed81b646baa356ca`  
Paketstatus: **abgeschlossen; kanonischer Clean-Commit-Dry-Run bestanden**

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

## Kanonischer P3-Dry-Run

Der Owner erteilte `PB-05` am 3. September 2026. Der erste Orchestrierungsversuch
stoppte vor der Paketierung, weil Windows PowerShell 5.1 gewöhnliche native
`stderr`-Ausgabe trotz erfolgreichem pnpm-Exit als abbrechenden Fehler behandelte.
Das unvollständige Verzeichnis enthielt nur ein Buildlog und wurde entfernt. Der
plattformkompatible Logwrapper wurde im selben logisch freigegebenen Commit
korrigiert.

Der anschließende kanonische Lauf aus dem sauberen Commit
`4c48058fc1b438ae1f0d5a76a2b17408a6b4b25e` bestand ohne GitHub Actions und ohne
kostenpflichtigen Dienst. Ausgabe:
`artifacts/private-release/0.3.0-rc.1/4c48058fc1b4/`.

| Nachweis | Ergebnis |
|---|---|
| NSIS-Installer | `LDTG_0.3.0-rc.1_x64-setup.exe`, 3.679.860 Bytes |
| Installer SHA-256 | `328c9d8b02e0173cbe8150dffa0adae32915a450abc317e2e6686e322f03a4f3` |
| Authenticode | `NotSigned`, wie für P3 vorgesehen |
| Buildlog SHA-256 | `bd3fa43c3dfb961303ccc41899fcc2096056b202f4b23426dc8b5ba0ef264d6f` |
| Buildmanifest SHA-256 | `5daf6fa54c9952a80ec0133e01b0b1295b15474d8eb14eef8c9f28336f3586c2` |
| CycloneDX-SBOM SHA-256 | `f80c1120d1d2afa097990e981e2226fad5f5d83f15699ef2f8d8b936afb169df` |
| SBOM-Komponenten | 857 |
| Portable ZIP | bewusst nicht enthalten |

Alle vier Einträge in `SHA256SUMS.txt` wurden nach dem Lauf unabhängig gegen die
Dateien geprüft. Manifest und SBOM nennen beide exakt den Buildcommit und den
Status `private-dry-run-not-published`; der Arbeitsbaum blieb sauber. Seit der
P2-Basisrevision ist genau ein neuer Commit erreichbar. Damit ist **Gate P3
bestanden**. Der private GitHub-Workflow wurde nicht ausgeführt und nichts wurde
veröffentlicht oder signiert.
