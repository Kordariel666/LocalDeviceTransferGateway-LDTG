# P6 – Unveröffentlichter Beta-Releasekandidat

Stand: 4. September 2026
Status: **P6 bestanden; `0.3.0-rc.2` nach der Repository-Bereinigung erneut
lokal gebaut, aber nicht veröffentlicht, getaggt oder signiert**

## Festgelegter Kandidat

| Feld | Wert |
|---|---|
| Produkt | LDTG – Local Device Transfer Gateway |
| Version | `0.3.0-rc.2` |
| Quellcommit des Installers | `4956cd3de88d6d5d5cff6c3653a71d9695a60c32` |
| vorgesehener Tag | `v0.3.0-rc.2` – nicht angelegt |
| Windows-Artefakt | `LDTG_0.3.0-rc.2_x64-setup.exe`, 3.708.085 Bytes |
| Installer SHA-256 | `7c7263d37e94cac525ac7de6d5ec1ffaf3c3a3ed680dc13e79017886c19d6f9c` |
| SBOM SHA-256 | `6cb9766a383b6fca5e5c3e947022f187cd8598d4fdd0fcaa61c57be00eb8fdb4` |
| Buildmanifest SHA-256 | `889e7ef44b9171b94424b900fec5def372330a8a1c85c640962eb6769dc20645` |
| Signatur | keine; unsigniert muss sichtbar kommuniziert werden |
| Projektlizenz | `Apache-2.0` |
| Copyright-Pseudonym | `Kordariel666` |
| Beiträge | zunächst nur Issues, keine Pull Requests |
| Kosten | 0 Euro im lokalen Dry-Run; keine Verpflichtung eingerichtet |
| Veröffentlichung | nein; Repositorysichtbarkeit und Remotes bleiben unverändert |

Der exakte Quellcommit und alle Artefakthashes werden absichtlich vom privaten
Releasepfad in `build-manifest.json` und `SHA256SUMS.txt` geschrieben. Ein
Commit kann seinen eigenen Hash nicht sinnvoll in seinen Inhalt aufnehmen;
diese commitgebundenen Ausgabedateien sind deshalb die maßgebliche P6-Evidenz.

## Inhalt der Entscheidungsmappe

- Projekt- und Drittanbieterrechte: `LICENSE`, `NOTICE`,
  `THIRD_PARTY_NOTICES.md`, Dependency-Inventar und CycloneDX-SBOM;
- Nutzerinformationen: README, Datenschutz, Support, Sicherheitsrichtlinie,
  Codesignierungsrichtlinie und Release-Notes;
- Beitragspfad: `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md` und zwei aktive
  Issues-Formulare; keine aktive Pull-Request-Vorlage;
- reale Evidenz: P4-Matrix mit bestandenen Kernpfaden und ausdrücklich
  akzeptierten Plattform-, Stress- und Barrierefreiheitslücken;
- SignPath: nur Vorprüfung; kein Antrag, Zertifikat, Token oder Versprechen;
- Build: sauberer Commit, feste Toolchains und Lockfiles, vollständiges Gate,
  frischer NSIS-Build, Buildlog, SBOM, Manifest und SHA-256-Prüfsummen;
- Veröffentlichungsentwurf: `docs/RELEASE_NOTES_0.3.0-rc.2.md`; kein Befehl im
  Dry-Run kann pushen, taggen, signieren oder einen Release anlegen.

## Build- und Prüfergebnis

Der lokale Clean-Commit-Dry-Run vom 4. September 2026 bestand von Commit
`4956cd3de88d6d5d5cff6c3653a71d9695a60c32`:

- 36 Desktop-, 39 Mobile- und 122 Rusttests sowie Typecheck, ESLint, Coverage,
  Webbuild, Vertragsprüfung, `rustfmt` und Clippy waren grün;
- genau ein frischer x64-NSIS-Installer wurde erzeugt;
- die direkte Prüfung der PE-Zertifikatstabelle meldete erwartungsgemäß
  `NotSigned`, ohne vom PowerShell-Modul-Autoloading abzuhängen;
- alle sieben Einträge in `SHA256SUMS.txt` wurden erneut gegen die erzeugten
  Dateien geprüft;
- SBOM, Buildlog, Projektlizenz, Project-Notice und die Notices für 344
  ausgelieferte Laufzeitkomponenten liegen als getrennte gehashte Dateien vor;
- der Build hinterließ den eingecheckten Quellbaum unverändert;
- auf dem Quellcommit liegt kein Tag.

## Manuelle Kernabnahme und finaler Neuaufbau

Der Owner bestätigte am 4. September 2026 für den vorausgehenden, exakt
gehashten P6-Installer aus Commit
`a70c1612f2c673b0be7cede2676b851ae134eccd`:

- Microsoft Defender meldete bei der gezielten Prüfung keinen Fund;
- Installation über den vorhandenen Stand und anschließender Programmstart
  funktionierten;
- die Desktop-Seitenleiste zeigte die tatsächliche Version `0.3.0-rc.2`;
- Windows „Installierte Apps“ zeigte `LDTG`, Version `0.3.0-rc.2`, Publisher
  `Kordariel666`;
- Freigabeordner, bestätigtes Netzwerk, Port 8876 und weitere Einstellungen
  blieben erhalten;
- Dienststart, Verbindung vom Handy und abschließender Dienststopp
  funktionierten ohne Defender-Meldung.

Die anschließende lesende Kontrolle bestätigte beide installierten EXE-Dateien
mit Produktversion `0.3.0-rc.2`, genau eine aktivierte eingehende LDTG-Regel für
TCP 8876, Programmpfad und `LocalSubnet`, erhaltene lokale Datenverzeichnisse
und keinen Listener auf 8765 oder 8876 nach dem Stopp.

Danach wurden ausschließlich öffentliche Dokumentation, QA-Nachweise,
Paket-Skripte und lokale Releasewerkzeuge bereinigt. Zwischen dem manuell
abgenommenen Stand und dem finalen Quellcommit gibt es keine Änderung unter
`apps/`, `src-tauri/src/`, an Tauri-Konfiguration, Markenassets, Lockfiles oder
den gebündelten Lizenz-/Notice-Dateien. Das Programmverhalten und der
ausgelieferte Laufzeitinhalt sind deshalb unverändert; die Funktionsabnahme muss
nicht wiederholt werden.

Der finale NSIS-Installer ist wegen der dokumentierten, nicht bytegleichen
Buildgrenzen trotzdem eine neue Binärdatei. Der Owner prüfte deshalb am
4. September 2026 genau den finalen Installer mit SHA-256
`7c7263d37e94cac525ac7de6d5ec1ffaf3c3a3ed680dc13e79017886c19d6f9c`
gezielt mit Microsoft Defender; die Prüfung meldete keinen Fund. Dieser
manuelle Schritt ist keine Veröffentlichungsfreigabe.

## Bereits akzeptierte Restrisiken

- Kein SmartScreen-Reputationstest ist mit einer rein lokal erzeugten Datei
  belegbar. Eine Warnung wegen unbekanntem Herausgeber bleibt für eine erste
  unsignierte Beta möglich.
- Reale Abdeckung ist auf Windows 11 25H2 und Android 16 mit Firefox begrenzt.
- Große/zahlreiche Dateien, Last-Pause/-Retry, voller oder langsamer Datenträger,
  Netzwerkwechsel, iOS und die Barrierefreiheitsmatrix sind keine bestandenen
  Realsystemtests. Sie werden in den Release-Notes nicht verschwiegen.
- Die beobachtete Übertragungsanzeige ist funktional erklärt, aber ohne
  Großdatei-Realszenario nicht vollständig erneut bewertet.

Diese bekannten Lücken verändern keine P0-/P1-Bewertung. Ein neuer Defender-
Fund, Installations-/Deinstallationsfehler, Datenverlust, falsche Version oder
Fehler im Kerntransfer blockiert P6 dagegen unmittelbar.

## Veröffentlichungsgrenze

Nach bestandenem P6 bleibt der Zustand unveröffentlicht. Tag, Push,
Repositorysichtbarkeit, GitHub Release, Private Vulnerability Reporting und ein
späterer SignPath-Antrag sind externe Zustandsänderungen und erfordern jeweils
die im archivierten `docs/archive/project-history/PUBLIC_BETA_PLAN.md`
vorgesehene ausdrückliche Entscheidung. Bis
dahin gilt weder ein öffentliches Supportversprechen noch eine Signaturzusage.
Die noch offene Entscheidung ist in
[`docs/PUBLICATION_DECISION.md`](../../docs/PUBLICATION_DECISION.md) vorbereitet.
