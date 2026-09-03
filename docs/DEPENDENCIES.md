# Abhängigkeitsstrategie

Stand: 2. September 2026

LDTG aktualisiert Abhängigkeiten kontrolliert und ohne automatisches Zusammenführen:

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

`ts-rs` bleibt auf der 11.x-Linie, solange LDTG Rust 1.85 als Mindestversion
unterstützt. Diese Linie benötigt mindestens Rust 1.78; `ts-rs` 12.x würde die
Projektanforderung auf Rust 1.88 anheben. Die Abhängigkeit läuft ausschließlich
zur Ableitung der TypeScript-Verträge aus Rust-Datentypen und beeinflusst deren
JSON-Laufzeitformat nicht.

## Lizenzinventur und SBOM

Der P1-Audit erzeugt aus `pnpm-lock.yaml`, `src-tauri/Cargo.lock`, den
Cargo-Metadaten und den Paketmanifesten eine versionsgenaue Inventur sowie einen
CycloneDX-1.6-SBOM-Entwurf:

```powershell
pnpm audit:public-beta
```

Der normale Lauf verwendet die bereits eingecheckte npm-Metadateninventur als
Offline-Cache, sofern ihr Lockfile-Hash exakt passt. Nach einer bewussten
Lockfile-Änderung darf ein rein lesender Registry-Lauf nur mit gesonderter
Netzfreigabe erfolgen:

```powershell
pnpm audit:public-beta:online
```

Der Generator bricht bei fehlender Lizenzdeklaration oder unvollständigen
Paketmetadaten ab. Ergebnisse und die noch geltenden Notice-/MPL-Auflagen sind
im [P1-Abhängigkeits- und Lizenz-Audit](../qa/public-beta/dependency-license-audit.md)
dokumentiert. Das ist noch kein finales Auslieferungs-Notice und keine Wahl der
Projektlizenz.
