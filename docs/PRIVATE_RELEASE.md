# Privater Release-Dry-Run

Stand: 3. September 2026  
Status: **technisch vorbereitet, noch nicht als P3-Gate bestanden**

Diese Strecke erzeugt ausschließlich private, unsignierte Prüfartefakte. Sie
veröffentlicht keinen GitHub Release, ändert keine Repositorysichtbarkeit,
aktiviert keine Lizenz, meldet keinen Dienst an und benötigt keine dauerhaften
Zugangsdaten. Der lokale Dry-Run verwendet keine kostenpflichtige Leistung.

## Festgelegte Eingaben

Ein kanonischer Dry-Run akzeptiert nur:

- Windows und einen sauberen Git-Arbeitsbaum;
- einen vollständigen 40-stelligen Commit-SHA, der exakt `HEAD` entspricht;
- die eingecheckten `pnpm-lock.yaml` und `src-tauri/Cargo.lock`;
- identische Versionen in Root- und Workspace-`package.json`, `Cargo.toml`,
  `tauri.conf.json` und einem datierten Changelogabschnitt;
- einen zum Lockfile passenden vollständigen Abhängigkeitsaudit ohne unbekannte
  Lizenzen oder Lizenzblocker;
- Node.js `24.19.0`, pnpm `11.19.0` und Rust `1.98.0` einschließlich `rustfmt`
  und Clippy;
- vollständige Commit-Hashes für jede externe GitHub Action.

`pnpm release:check` prüft diese Bedingungen ohne ein Paket zu bauen. Der
kanonische lokale Lauf wird so gestartet:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/private-release.ps1 `
  -ExpectedRevision <vollstaendiger-commit-sha>
```

Ein abweichender Commit, ein schmutziger Arbeitsbaum, eine Versionsabweichung,
ein überholter Audit oder eine nicht fixierte Action beendet den Lauf vor
Installation, Test und Paketierung. Anschließend werden Abhängigkeiten nur aus
den Lockfiles installiert, der vollständige `pnpm check` läuft vor
`pnpm build`, und Cargo arbeitet nach dem kontrollierten Fetch offline. Eine
durch den Build veränderte Quelldatei lässt den Lauf ebenfalls fehlschlagen.

## Ergebnis

Der Standardpfad `artifacts/private-release/<version>/<commit-kurz>/` ist
absichtlich ignoriert. Ein bereits gefülltes Ausgabeverzeichnis wird nicht
überschrieben. Ein erfolgreicher Lauf enthält:

- genau einen frisch erzeugten, unsignierten NSIS-Installer;
- `SHA256SUMS.txt` für Installer, SBOM, Buildprotokoll und Manifest;
- `sbom.cdx.json` im CycloneDX-1.6-Format, an Commit und Lockfile-Hashes
  gebunden;
- `build.log` mit den tatsächlich ausgeführten Gates;
- `build-manifest.json` mit Quellrevision, Tags, Toolchain, Runnerbild,
  Lockfile-Hashes, Action-Pins, Auditbasis, Dateigrößen und Artefakthashes.

Die aktuelle Tauri-Konfiguration erzeugt nur NSIS. Eine portable ZIP-Datei ist
für diesen Dry-Run bewusst **nicht** Teil des Lieferumfangs; das Manifest hält
dies mit `portableArtifactIncluded: false` fest.

## Privater CI-Pfad

`.github/workflows/private-release.yml` ist ausschließlich manuell auslösbar.
Die Eingabe `expected_revision` wird ausgecheckt und im Build erneut gegen
`HEAD` geprüft. Der Workflow besitzt nur `contents: read`, verwendet keine
Secrets, ruft keine Release- oder Signier-API auf und bewahrt sein privates
Prüfarbeitsartefakt sieben Tage auf. Er wurde in P3 nicht ausgeführt; dadurch
entstanden weder externer Zustand noch Runnerkosten.

Die Fremd-Actions sind auf die zum Prüfdatum verifizierten vollständigen
Revisionen festgelegt:

- `actions/checkout` v7.0.0:
  <https://github.com/actions/checkout/commit/9c091bb21b7c1c1d1991bb908d89e4e9dddfe3e0>
- `actions/upload-artifact` v7.0.1:
  <https://github.com/actions/upload-artifact/commit/043fb46d1a93c77aae656e7c1c64a875d1fc6a0a>
- `pnpm/setup` v2.1.0:
  <https://github.com/pnpm/setup/commit/703c52620218391530e48b9e8870d5c0082e1b9b>

Die gewählte Node-LTS-Version besitzt veröffentlichte signierte Downloads:
<https://nodejs.org/download/release/v24.19.0/>. Eine spätere Pin-Aktualisierung
ist eine überprüfte Quelländerung und kein automatisch bewegliches Buildziel.

## Reproduzierbarkeit und Grenzen

Deterministisch beziehungsweise eindeutig gebunden sind:

- Quellcommit, Appversion, Lockfile-Hashes und Dependencygraph;
- die für denselben Commit und dieselben Lockfiles erzeugte SBOM-Seriennummer;
- die konkreten Action-Revisionen;
- SHA-256-Prüfsummen jedes tatsächlich erzeugten Artefakts.

P3 behauptet bewusst **keine bytegleiche Reproduzierbarkeit** des Installers.
GitHub aktualisiert das Image hinter `windows-2025`; Windows SDK, MSVC und
WebView-/Systemwerkzeuge können sich damit ändern. PE- und NSIS-Erstellung,
Kompressionsreihenfolge, lokale Pfade sowie Paketmetadaten können Zeit- oder
Umgebungsanteile enthalten. Buildprotokoll und Manifest enthalten absichtlich
Zeitstempel. Eine spätere Authenticode-Signatur verändert den Installer und
erfordert neue Prüfsummen.

Das Manifest zeichnet `ImageOS`, `ImageVersion`, Architektur und konkrete
Toolausgaben auf. Damit bleibt ein Artefakt einem kontrollierten Quellstand und
einer beobachteten Buildumgebung zuordenbar, auch wenn ein späterer Neubau nicht
bytegleich ist.
