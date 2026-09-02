# Changelog

Alle wesentlichen Änderungen an DMDC werden in dieser Datei dokumentiert.
Das Format orientiert sich an [Keep a Changelog](https://keepachangelog.com/de/1.1.0/),
die Versionsnummern folgen [Semantic Versioning](https://semver.org/lang/de/).

## [Unreleased]

Geplante Änderungen und deren Reihenfolge stehen in [`docs/ROADMAP.md`](docs/ROADMAP.md).

### Changed

- API-, Architektur- und Testdokumentation an die tatsächlich implementierten
  Uploadbudgets, Chunkgrößen, Absturzbehandlung und Codesperre angeglichen.
- Frühere Audit- und Behebungsstufen eindeutig als historische Zwischenstände
  gekennzeichnet.
- QA-Bilderzeugung von persönlichen absoluten Pfaden gelöst und aktuelle Mobile-
  sowie Desktop-Aufnahmen für gestoppten und laufenden Dienst erneuert.
- Die mobile Uploadwarteschlange besitzt nun einen expliziten Reducer als einzige
  fachliche Zustandsquelle und unterscheidet dauerhafte von transienten Fehlern.
- Blockierende Uploadanlage, Inbox-Prüfung, Chunk-Persistierung, Abschluss und
  Partial-Bereinigung laufen in einem eigenen fairen I/O-Pool außerhalb der
  Async-Worker; die periodische Rootprüfung blockiert den Accept-Loop ebenfalls
  nicht mehr.

### Added

- Windows-CI-Gate für Typprüfung, ESLint, Frontend-Coverage, Rust-Tests,
  Formatierung, Clippy und Produktions-Webbuilds.
- Wöchentliche, nicht automatisch zusammengeführte Dependabot-Updates für pnpm,
  Cargo und GitHub Actions mit dokumentierter Reviewstrategie.

### Fixed

- Pause, Abbruch und Sitzungsverlust unterbrechen mobile Upload-Retrys sofort und
  geben die nächste Datei frei; laufende Create-Anfragen bleiben ohne Duplikate
  nachverfolgbar.
- Strukturierte PATCH-Fehlercodes bleiben im mobilen Uploadstatus erhalten.
- Uploadoffset, Inbox-Bytebudget und Transferfortschritt werden erst nach
  erfolgreichem `sync_data` gemeinsam bestätigt. Abgebrochene HTTP-Waiter lassen
  den dienstbesessenen Chunkjob samt Blocking-Permits konsistent zu Ende laufen.
- Doppeltes Login erzeugt nicht mehr mehrere parallele Auth-Anfragen; Logout räumt
  den lokalen Zustand auch bei einem bereits gestoppten Dienst auf.

- Die vollständige Prüfkette baut eingebettete Mobile-Webassets nun vor Rust-
  Tests und Clippy; `pnpm test:rust` bereitet sie auch einzeln selbst vor und
  funktioniert dadurch auf einem frischen Checkout.
- Windows-Netzwerkprofile werden über `Get-NetConnectionProfile` einschließlich
  stabiler Profil-ID ermittelt; der Dienststart hängt damit nicht mehr von der auf
  manchen Systemen fehlschlagenden dynamischen `NetworkListManager`-COM-Abfrage ab.

## [0.1.3] - 2026-09-02

### Added

- Erster versionierter und vollständig geprüfter Ausgangsstand des Projekts.
- Desktop-Anwendung zur Konfiguration lokaler Download- und Uploadfreigaben.
- Mobile Weboberfläche für authentifizierte Dateiübertragungen im lokalen Netzwerk.
- Sicherheits-, Architektur-, API- und Testdokumentation.
