# Abhängigkeitsstrategie

Stand: 2. September 2026

DMDC aktualisiert Abhängigkeiten kontrolliert und ohne automatisches Zusammenführen:

- Dependabot prüft npm/pnpm, Cargo und GitHub Actions wöchentlich.
- Minor- und Patch-Updates werden je Ökosystem gebündelt; Major-Updates bleiben
  einzeln und benötigen eine bewusste Migrationsprüfung.
- Jeder Update-PR muss Manifest und Lockfile gemeinsam enthalten und das komplette
  Windows-Qualitätsgate bestehen.
- Sicherheitsupdates werden priorisiert. Ein grüner Build ersetzt trotzdem nicht
  die Prüfung von Changelog, Berechtigungen, Laufzeitverhalten und neuen
  transitive Abhängigkeiten.
- Es gibt kein Auto-Merge. Erst nach Review und erfolgreicher realer Abnahme bei
  betriebssystemnahen Änderungen wird zusammengeführt.

Die CI-Actions erhalten nur Leserechte auf Repository-Inhalte. Fremde Actions
werden auf einen vollständigen Commit-Hash festgelegt; GitHub-eigene Actions
verwenden einen gepflegten Hauptversions-Tag und werden von Dependabot beobachtet.

`ts-rs` bleibt auf der 11.x-Linie, solange DMDC Rust 1.85 als Mindestversion
unterstützt. Diese Linie benötigt mindestens Rust 1.78; `ts-rs` 12.x würde die
Projektanforderung auf Rust 1.88 anheben. Die Abhängigkeit läuft ausschließlich
zur Ableitung der TypeScript-Verträge aus Rust-Datentypen und beeinflusst deren
JSON-Laufzeitformat nicht.
