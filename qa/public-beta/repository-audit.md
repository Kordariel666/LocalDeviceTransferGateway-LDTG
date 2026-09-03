# P1-Repository-, Herkunfts- und Datenschutz-Audit

Stand: 3. September 2026

Auditierter Quellstand: `a523770d8c20ae0695763acbb25c0e32fd552926`

Ergebnis: **BLOCKIERT – technische Prüfung abgeschlossen, eine Owner-Bestätigung offen**

Dieses Dokument prüft ausschließlich P1 des
[Plans bis zur Veröffentlichungsentscheidung](../../docs/PUBLIC_BETA_PLAN.md).
Es aktiviert keine Projektlizenz, veröffentlicht nichts und autorisiert weder
SignPath noch kostenpflichtige Dienste. R5.2 und Phase 6 bleiben pausiert.

## Kurzurteil

- Im Arbeitsbaum und in den vier ausdrücklich vorgesehenen öffentlichen Refs
  wurden keine bekannten Token-, Schlüssel- oder Zugangsdatenmuster gefunden. Ein
  Regex-Audit ersetzt keine mathematische Garantie, liefert für den geprüften
  Stand aber keinen Secret-Befund.
- Nach dem vom Owner gewählten destruktiven Rewrite enthalten `main` und die drei
  Release-Tags weder persönliche Commit-/Tag-E-Mail-Adressen noch absolute
  Benutzer- oder Werkzeugpfade. Sämtliche 34 Commit-Identitäten und drei
  annotierten Tagger verwenden die konfigurierte GitHub-noreply-Adresse.
- Es wurden keine eingecheckten ausführbaren Fremdbinärdateien, Archive, Fonts,
  PDFs oder WebAssembly-Dateien gefunden. Die 64 PNG/JPEG-Dateien enthalten
  keine erkannten Text-, EXIF- oder Kommentarmetadatenblöcke.
- Die Herkunft des Quellcodes, des Logos und der aus dem Logo abgeleiteten
  Icons kann aus Git allein nicht lückenlos bewiesen werden, weil der
  Wurzelcommit bereits fast das vollständige Projekt importierte. Dafür fehlt
  eine Bestätigung des Rechteinhabers.
- Die frühere Bezeichnung wurde wegen eines deutlichen
  Auffindbarkeits-/Verwechslungsrisikos ersetzt. Der Owner hat **LDTG – Local
  Device Transfer Gateway** gewählt. Die wiederholte exakte Vorprüfung ergab
  keinen offensichtlichen Konflikt mit einer Dateiübertragungssoftware; die
  Abkürzung besitzt jedoch fachfremde Verwendungen. Das ist keine formale
  Markenfreigabe.

Das P1-Gate bleibt deshalb nur noch an `PB-02` aus
[`blockers.json`](blockers.json) hängen. `PB-01` und `PB-03` sind abgeschlossen.

## Prüfumfang und reproduzierbare Nachweise

Der maschinenlesbare Nachweis
[`repository-evidence.json`](repository-evidence.json) wurde mit
[`scripts/public-beta-audit.mjs`](../../scripts/public-beta-audit.mjs) erzeugt.
Geprüft wurden der Arbeitsbaum und genau die für eine spätere Veröffentlichung
vorgesehenen Refs `main`, `v0.1.3`, `v0.2.0-rc.1` und `v0.2.0-rc.2`, deren
vollständiger Patchverlauf und alle eingecheckten PNG-/JPEG-Dateien. Der lokale
Vorbereitungsbranch besitzt denselben bereinigten Tip wie `main`.

Wesentliche Kennzahlen des Ausgangsstands:

| Merkmal | Ergebnis |
|---|---:|
| erreichbare Commits über die öffentlichen Ziel-Refs | 34 |
| geprüfte Refs | 4 |
| eingecheckte Dateien | 192 |
| Größe der eingecheckten Dateien | 10.595.210 Bytes |
| erreichbare Git-Objekte / eindeutige Blobs | 988 / 629 |
| größter Blob | 1.117.268 Bytes |
| Blobs ab 5 MiB | 0 |
| geprüfte PNG/JPEG-Dateien | 64 |

Remote-Tracking-, Dependabot-PR-, Codex-Werkzeug- und temporäre Rewrite-Refs sind
bewusst nicht Teil dieses Veröffentlichungsscans. Sie sind nicht zu pushen und
werden nicht in einen neuen öffentlichen Remote übernommen.

Ausgeführt beziehungsweise ausgewertet wurden unter anderem:

```powershell
git rev-list --objects refs/heads/main refs/tags/v0.1.3 refs/tags/v0.2.0-rc.1 refs/tags/v0.2.0-rc.2
git cat-file --batch-check="%(objectname) %(objecttype) %(objectsize)"
git log -p --no-ext-diff --no-textconv refs/heads/main refs/tags/v0.1.3 refs/tags/v0.2.0-rc.1 refs/tags/v0.2.0-rc.2
node scripts/public-beta-audit.mjs --public-ref=refs/heads/main --public-ref=refs/tags/v0.1.3 --public-ref=refs/tags/v0.2.0-rc.1 --public-ref=refs/tags/v0.2.0-rc.2
```

## Git-Historie und Datenschutz

Der Musterscan über den vollständigen, lokal erreichbaren Patchverlauf ergab:

| Klasse | Treffer |
|---|---:|
| Private-Key-Blöcke | 0 |
| AWS-, GitHub-, npm- oder Slack-Tokenmuster | 0 |
| JWT-Muster | 0 |
| Zugangsdaten in URLs | 0 |
| generische Secret-Zuweisungen | 0 |
| persönliche absolute Pfade | 0 |
| E-Mail-ähnliche Zeichenfolgen | 704 Mustertreffer |

Die Zahlen zählen Vorkommen in wiederholten Patches und sind keine Anzahl
eindeutiger Probleme. Die verbleibenden E-Mail-Treffer stammen aus öffentlichen
Dependency-Metadaten, Beispielen und GitHub-noreply-Identitäten; die persönliche
Adresse besitzt im Zielscan null Treffer. Der maschinenlesbare Nachweis speichert
Commit- und Tagger-Adressen nur als SHA-256-Fingerabdruck und Domain.

Vor dem Rewrite wurde ein vollständiges privates Git-Bundle außerhalb des
Repositorys erzeugt und mit `git bundle verify` geprüft. Es umfasst 19 damalige
Refs, ist 8.736.205 Bytes groß und besitzt SHA-256
`a10b3ed6b1da32eeb0c46cd02321cfe14348035c54b6fdd648ad4d57b1eef7d3`.
Anschließend wurden `main`, der Vorbereitungsbranch und alle drei Release-Tags
kontrolliert umgeschrieben. Alte Remote-PR- und Werkzeugrefs bleiben nur private
Altstände; bei Nutzung des bestehenden GitHub-Repositorys muss vor einer
Öffentlichschaltung zusätzlich sichergestellt werden, dass providerseitige
Pull-Request-/Cache-Refs nicht öffentlich erreichbar werden.

## Assets, Texte und erzeugte Dateien

| Gruppe | Befund | Herkunftsstatus |
|---|---|---|
| Quellcode, Projektdokumentation und UI-Texte | keine vendorten Fremdquellen erkannt; nahezu vollständig im Wurzelcommit importiert | Owner-Bestätigung ausstehend |
| `assets/ldtg-logo-lockup.png`, `assets/ldtg-app-icon.png`, `assets/ldtg-ui-icon.png` | ausgewählte und abgeleitete LDTG-Markenassets, mit KI-Unterstützung erzeugt | Owner-Bestätigung ausstehend |
| `src-tauri/icons/**` | mit der Tauri-CLI aus `assets/ldtg-app-icon.png` erzeugte PNG/ICO/ICNS- sowie Android-/iOS-Varianten | reproduzierbar; Owner-Bestätigung für das Quellicon ausstehend |
| `qa/*.png`, `qa/*.jpg` | Aufnahmen der LDTG-Oberflächen und zwei per `qa/make_comparison.py` erzeugte Vergleiche | Rechte-/Testdatenbestätigung ausstehend |
| `packages/shared/src/index.ts` | laut Dateikopf aus Rust-Verträgen erzeugt | im Repository reproduzierbar |
| `src-tauri/gen/schemas/*.json` | Tauri-generierte Konfigurations-/Berechtigungsschemata | Fremdherkunft über Tauri; Notices aus Lizenzinventur ableitbar |
| `pnpm-lock.yaml`, `src-tauri/Cargo.lock` | von pnpm beziehungsweise Cargo erzeugte Auflösung | reproduzierbare Paketmetadaten |
| Fonts | nur lokale Systemfont-Fallbacks, kein `@font-face`, keine Fontdatei | nichts wird gebündelt |
| mobile Webassets | aus eigenem Workspace gebaut und mit `rust-embed` in den Dienst eingebettet; keine externen Laufzeitressourcen | abhängig von Owner-Bestätigung für Projektquellen |

Alle 13 QA-Oberflächen-/Vergleichsbilder wurden zusätzlich visuell geprüft.
Erkennbar sind nur LDTG-Oberflächen, private LAN-Adressen, generische
`C:\LDTG\...`-Testordner, ein Testprofil sowie Zugangscodes/QR-Codes. Namen,
Benutzerprofile oder Dateiinhalte sind nicht sichtbar. Laut `qa/README.md`
wurden die Codes nach den Laufzustandsaufnahmen durch Stoppen des Dienstes
ungültig. Vor einer Veröffentlichung muss der Owner dennoch bestätigen, dass
LAN-Adressen, Ordner und Profil reine freigabefähige Testdaten sind.

Die dafür vorgesehene, noch **nicht bestätigte** Erklärung steht in
[`provenance-attestation.md`](provenance-attestation.md). Ohne diese Erklärung
kann Git keine Rechtekette vor dem Wurzelcommit belegen.

## Namens- und offensichtliche Konfliktprüfung

Geprüft am 3. September 2026:

- npm: kein exaktes Paket `ldtg`;
- crates.io/Cargo: kein exakter Treffer für `ldtg`;
- winget: kein exaktes Paket `LDTG`;
- DPMAregister-Basissuche, exakte Begriffe, nationale deutsche Marken,
  Unionsmarken und international registrierte Marken: weder `LDTG` noch
  `Local Device Transfer Gateway` mit Treffer;
- allgemeine Web- und GitHub-Suche: kein offensichtlicher exakter
  Produktnamenskonflikt für `Local Device Transfer Gateway`; das Kürzel `LDTG`
  wird fachfremd unter anderem als historische Logo-Design-Studio-Dateiendung
  und als Schreibweise `LDTg` in der Neurowissenschaft verwendet.

Die Registerprüfung erfolgte im offiziellen
[DPMAregister](https://register.dpma.de/DPMAregister/marke/basis). Sie war eine
reine Kollisions-Vorprüfung, keine Ähnlichkeitssuche, Klassenberatung oder
anwaltliche Markenrecherche. Der Owner hat die Umbenennung und das verbleibende
Risiko am 3. September 2026 entschieden. Details und reproduzierbare Suchziele
stehen in [`docs/BRANDING.md`](../../docs/BRANDING.md). `PB-03` ist damit
abgeschlossen; die öffentliche Erstnennung führt Kürzel und vollen Namen
gemeinsam.

## Gatebewertung

| P1-Kriterium | Status | Begründung |
|---|---|---|
| kein ungeklärtes fremdes/proprietäres Artefakt | blockiert | Herkunftsbestätigung für Projektquellen, Logo, Icons und QA-Testdaten fehlt |
| ausgelieferte Abhängigkeiten mit bekannter kompatibler Lizenz | erfüllt | 857 Pakete inventarisiert, 0 ohne deklarierte Lizenz; siehe Lizenz-Audit |
| erforderliche Drittanbieterhinweise ableitbar | erfüllt mit Auslieferungsauflage | Lizenz-/Notice-Dateien und Prüfsummen sind inventarisiert; das finale Notice-Bündel entsteht erst nach Lizenzentscheidung |
| keine Secrets oder privaten Nutzdaten im vorgesehenen öffentlichen Git-Stand | erfüllt | exakter Scan von `main` und drei Tags: 0 Secret-, 0 persönliche Pfad- und 0 persönliche Identitätstreffer; privates Vollbackup verifiziert |

P1 ist daher **nicht abgeschlossen**. P2, P3, R5.2, Phase 6, Lizenzaktivierung,
SignPath und Veröffentlichung werden durch dieses Audit weder begonnen noch
freigegeben.
