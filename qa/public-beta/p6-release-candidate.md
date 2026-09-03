# P6 – Unveröffentlichter Beta-Releasekandidat

Stand: 4. September 2026
Status: **in Vorbereitung; automatischer Build und finale manuelle Kernabnahme
von `0.3.0-rc.2` stehen noch aus**

## Festgelegter Kandidat

| Feld | Wert |
|---|---|
| Produkt | LDTG – Local Device Transfer Gateway |
| Version | `0.3.0-rc.2` |
| vorgesehener Tag | `v0.3.0-rc.2` – nicht angelegt |
| Windows-Artefakt | `LDTG_0.3.0-rc.2_x64-setup.exe` – noch zu erzeugen |
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

## Verbleibende P6-Schritte

1. Alle generierten Verträge, Notices, Tests, Lints und Rust-Gates bestehen.
2. Der vorbereitete Stand wird lokal committed; es wird nichts gepusht.
3. Der private Releasepfad erzeugt aus exakt diesem sauberen Commit den
   unsignierten Installer und die Nachweisdateien.
4. Der Owner prüft genau diesen Installer mit Microsoft Defender, installiert
   ihn und bestätigt sichtbar `0.3.0-rc.2`, Einstellungserhalt, Dienststart,
   Handyzugriff und Dienststopp.
5. Erst danach wird P6 als bestanden markiert und die getrennte
   Veröffentlichungsentscheidung vorbereitet.

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
die in `docs/PUBLIC_BETA_PLAN.md` vorgesehene ausdrückliche Entscheidung. Bis
dahin gilt weder ein öffentliches Supportversprechen noch eine Signaturzusage.
