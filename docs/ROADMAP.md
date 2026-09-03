# LDTG-Roadmap

Stand: 4. September 2026

LDTG `0.3.0-rc.2` ist als unveröffentlichter Public-Beta-Kandidat technisch
und auf den dokumentierten realen Kernpfaden abgenommen. P0 bis P6 des
Veröffentlichungsprogramms sind abgeschlossen. Als Nächstes steht ausschließlich
die bewusste Veröffentlichungsentscheidung an.

## Unveränderliche Leitplanken

- Downloads bleiben lesend und Uploads add-only.
- Download- und Uploadwurzel bleiben vollständig getrennt.
- Mobilgeräte können weder Dienst noch Firewall oder Konfiguration steuern.
- Zugangscode und Sitzungstoken gelangen nicht in URL, QR-Code oder Logs.
- Der Dienst bindet nur an die gewählte private IPv4-Adresse und stoppt bei
  relevanten Netzwerk- oder Freigabeänderungen.
- Neue Funktionen erhalten feste globale und clientbezogene Ressourcenlimits.
- Persistente Daten bleiben sparsam, sichtbar und kontrollierbar.

## Abgeschlossen

| Bereich | Ergebnis |
|---|---|
| Baseline und CI | feste Toolchains, vollständiges Qualitätsgate und bereinigte Git-Historie |
| Laufzeitrobustheit | deterministische Uploadqueue, begrenztes Blocking-I/O und versionierte Einstellungen |
| Architektur | aus Rust erzeugte TypeScript-Verträge, typisierte Fehler und modulare Servergrenzen |
| Transferkomfort | Pause, Retry, Batchstatus, ehrliche Fortschrittsanzeige und Laufzeitverlauf |
| Netzwerk und Geräte | vertrauenswürdige Netzwerkprofile, verständliche Geräteidentität und Sitzungswiderruf |
| Freigabeprofile | mehrere gespeicherte Profile mit genau einem aktiven Laufzeitprofil |
| Releasevorbereitung | Apache-2.0, Issues-only, SBOM, Notices, privater Releasepfad und reale Kernabnahme |

Der ausführliche Entwicklungsverlauf wurde nach Abschluss archiviert:
[ROADMAP_HISTORY_2026-09-04.md](archive/project-history/ROADMAP_HISTORY_2026-09-04.md).

## Nächster Entscheidungspunkt

Die noch offene Entscheidung steht in
[PUBLICATION_DECISION.md](PUBLICATION_DECISION.md):

- `GO-APACHE`: unsignierte erste Beta unter Apache-2.0 veröffentlichen;
- `HOLD`: Kandidat privat lassen;
- `ARCHIVE`: Entwicklung beenden;
- `COMMERCIAL-DISCOVERY`: Veröffentlichung für einen getrennten Geschäfts- und
  Lizenzweg anhalten.

Ohne ausdrückliches `GO-APACHE` werden weder Repositorysichtbarkeit noch Tag,
GitHub Release, Signierung oder externe Dienste verändert.

## Nach einer möglichen Beta

Diese Punkte sind mögliche Folgearbeiten, keine Zusage für die erste Beta:

1. reale Rückmeldungen und Fehlerberichte triagieren;
2. SignPath-Eignung nach einer öffentlichen Version neu bewerten;
3. Browser-E2E und zusätzliche reale Geräte-/Barrierefreiheitstests ergänzen;
4. nur bei belegtem Bedarf sichere Wiederaufnahme nach Reload oder Neustart
   entwerfen;
5. Ordner-/Sammeltransfers erst nach festgelegten Datei-, Pfad-, CPU- und
   Speichergrenzen entwickeln;
6. Transportverschlüsselung ausschließlich als eigenes Protokoll- und
   Vertrauensmodell für eine spätere Hauptversion untersuchen;
7. einen signaturgeprüften Updatepfad erst nach eingeführter Codesignierung
   planen.

## Bewusst nicht Teil der ersten Beta

- mehrere gleichzeitig aktive Freigabeprofile;
- Internetzugriff, Portweiterleitung oder Fernadministration;
- automatisches Ausführen oder Öffnen empfangener Dateien;
- unverschlüsselte öffentliche Nutzung;
- automatische Updates;
- behauptete Unterstützung nicht real geprüfter Plattformen.
