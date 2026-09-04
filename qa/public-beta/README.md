# Public-Beta-Nachweise P0–P6

Dieser Ordner enthält die abgeschlossene interne Vorbereitung des
nicht öffentlich freigegebenen Kandidaten `0.3.0-rc.2`. Tag und private
GitHub-Prerelease sind vorbereitet. Die Dateien sind Nachweise und keine
zusätzliche Bedienungsanleitung.

## Lesbare Zusammenfassungen

- [Repository, Herkunft und Datenschutz](repository-audit.md)
- [Abhängigkeiten und Lizenzen](dependency-license-audit.md)
- [P2: Sicherheit, Datenschutz und Support](p2-security-privacy-support.md)
- [P3: private Releasepipeline](p3-release-pipeline.md)
- [P4: Windows- und Android-Kernabnahme](p4-real-device-matrix.md)
- [P5: Lizenz, Beiträge und SignPath](p5-license-contribution-signpath.md)
- [P6: privater Releasekandidat](p6-release-candidate.md)
- [Herkunftsbestätigung](provenance-attestation.md)

## Maschinenlesbare Nachweise

- `blockers.json`: Entscheidungen, geschlossene Blocker und Restrisiken;
- `dependency-licenses.json`: versionsgenaue Abhängigkeits- und Lizenzinventur;
- `repository-evidence.json`: überprüfte Git- und Dateievidenz;
- `sbom.cdx.json`: CycloneDX-SBOM-Vorlage für den privaten Releasepfad.

Die veröffentlichungsnahe SBOM wird beim Clean-Commit-Dry-Run erneut an den
exakten Quellcommit gebunden. Der vollständige Plan hinter P0–P6 liegt nur noch
als [historischer Projektplan](../../docs/archive/project-history/PUBLIC_BETA_PLAN.md)
im Archiv.
