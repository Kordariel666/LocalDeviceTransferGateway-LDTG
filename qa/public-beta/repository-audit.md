# P1-Repository-, Herkunfts- und Datenschutz-Audit

Stand: 4. September 2026

Auditierter Quellstand: `4956cd3de88d6d5d5cff6c3653a71d9695a60c32`

Ergebnis: **ERFÜLLT – P1 abgeschlossen**

Dieses Dokument prüft ausschließlich P1 des
[Plans bis zur Veröffentlichungsentscheidung](../../docs/archive/project-history/PUBLIC_BETA_PLAN.md).
Es veröffentlicht nichts und autorisiert weder SignPath noch kostenpflichtige
Dienste. Die separat gewählte Projektlizenz ist Apache-2.0; R5.2 und Phase 6
waren zu diesem damaligen P1-Auditzeitpunkt noch pausiert.

## Nachträgliche GitHub-Aufräumung

Nach P6 löschte der Owner am 4. September 2026 die drei historischen
DMDC-Tags `v0.1.3`, `v0.2.0-rc.1` und `v0.2.0-rc.2` lokal und aus dem privaten
GitHub-Repository. Als einziger Release-Tag verbleibt `v0.3.0-rc.2`; er zeigt
weiterhin auf den unveränderten Quellcommit
`4956cd3de88d6d5d5cff6c3653a71d9695a60c32`. Die normale `main`-Historie und
die für Upgrades erforderliche Erkennung der alten Firewallregel bleiben
absichtlich erhalten.

Zusätzlich wurden eine kurze Repositorybeschreibung und thematisch passende
GitHub-Topics gesetzt sowie das ungenutzte Projects-Feature deaktiviert.
Repository, Tag und Prerelease bleiben privat. Der nachfolgende P1-Nachweis
bildet weiterhin bewusst den damaligen Auditzeitpunkt vor der Tagbereinigung ab
und wird nicht rückwirkend umgedeutet.

## Kurzurteil

- Im Arbeitsbaum und in den zum Auditzeitpunkt vier ausdrücklich vorgesehenen
  öffentlichen Refs wurden keine bekannten Token-, Schlüssel- oder
  Zugangsdatenmuster gefunden. Ein Regex-Audit ersetzt keine mathematische
  Garantie, liefert für den geprüften Stand aber keinen Secret-Befund.
- Nach dem vom Owner gewählten destruktiven Rewrite enthalten der vorgesehene
  Quellcommit und die drei Release-Tags keine persönlichen
  Commit-/Tag-E-Mail-Adressen. Sämtliche 47 Commit-Identitäten und drei
  annotierten Tagger verwenden die konfigurierte GitHub-noreply-Adresse. Der
  einzige unterschiedliche Benutzerpfad-Mustertreffer ist eine synthetische
  Windows-Testadresse, kein realer Benutzer- oder Werkzeugpfad.
- Es wurden keine eingecheckten ausführbaren Fremdbinärdateien, Archive, Fonts,
  PDFs oder WebAssembly-Dateien gefunden. Die 51 PNG/JPEG-Dateien enthalten
  keine erkannten Text-, EXIF- oder Kommentarmetadatenblöcke.
- Die Herkunft des Quellcodes, des Logos und der aus dem Logo abgeleiteten
  Icons kann aus Git allein nicht lückenlos bewiesen werden, weil der
  Wurzelcommit bereits fast das vollständige Projekt importierte. Der
  Repositoryinhaber hat die Herkunft und Rechte deshalb gesondert bestätigt.
- Die frühere Bezeichnung wurde wegen eines deutlichen
  Auffindbarkeits-/Verwechslungsrisikos ersetzt. Der Owner hat **LDTG – Local
  Device Transfer Gateway** gewählt. Die wiederholte exakte Vorprüfung ergab
  keinen offensichtlichen Konflikt mit einer Dateiübertragungssoftware; die
  Abkürzung besitzt jedoch fachfremde Verwendungen. Das ist keine formale
  Markenfreigabe.

Das P1-Gate ist erfüllt. `PB-01`, `PB-02` und `PB-03` sind in
[`blockers.json`](blockers.json) als abgeschlossen dokumentiert.

## Prüfumfang und reproduzierbare Nachweise

Der maschinenlesbare Nachweis
[`repository-evidence.json`](repository-evidence.json) wurde mit
[`scripts/public-beta-audit.mjs`](../../scripts/public-beta-audit.mjs) erzeugt.
Geprüft wurden der Arbeitsbaum und genau der vorgesehene Quellcommit `HEAD` sowie
die historischen Release-Tags `v0.1.3`, `v0.2.0-rc.1` und `v0.2.0-rc.2`, deren
vollständiger Patchverlauf und alle eingecheckten PNG-/JPEG-Dateien. Lokale
Remote-Tracking-, PR-, Rewrite- und Werkzeugrefs sind nicht Bestandteil dieses
Veröffentlichungsscans.

Wesentliche Kennzahlen des Ausgangsstands:

| Merkmal | Ergebnis |
|---|---:|
| erreichbare Commits über die öffentlichen Ziel-Refs | 47 |
| geprüfte Refs | 4 |
| eingecheckte Dateien | 211 |
| Größe der eingecheckten Dateien | 8.769.834 Bytes |
| erreichbare Git-Objekte / eindeutige Blobs | 1.230 / 781 |
| größter Blob | 1.117.268 Bytes |
| Blobs ab 5 MiB | 0 |
| geprüfte PNG/JPEG-Dateien | 51 |

Remote-Tracking-, Dependabot-PR-, Codex-Werkzeug- und temporäre Rewrite-Refs sind
bewusst nicht Teil dieses Veröffentlichungsscans. Sie sind nicht zu pushen und
werden nicht in einen neuen öffentlichen Remote übernommen.

Ausgeführt beziehungsweise ausgewertet wurden unter anderem:

```powershell
git rev-list --objects HEAD refs/tags/v0.1.3 refs/tags/v0.2.0-rc.1 refs/tags/v0.2.0-rc.2
git cat-file --batch-check="%(objectname) %(objecttype) %(objectsize)"
git log -p --no-ext-diff --no-textconv HEAD refs/tags/v0.1.3 refs/tags/v0.2.0-rc.1 refs/tags/v0.2.0-rc.2
node scripts/public-beta-audit.mjs --public-ref=HEAD --public-ref=refs/tags/v0.1.3 --public-ref=refs/tags/v0.2.0-rc.1 --public-ref=refs/tags/v0.2.0-rc.2
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
| absolute Benutzerpfad-Muster | 5 Vorkommen derselben synthetischen Testadresse |
| E-Mail-ähnliche Zeichenfolgen | 1.000 Mustertreffer |

Die Zahlen zählen Vorkommen in wiederholten Patches und sind keine Anzahl
eindeutiger Probleme. Die verbleibenden E-Mail-Treffer stammen aus öffentlichen
Dependency-Metadaten, Beispielen und GitHub-noreply-Identitäten; eine persönliche
Adresse wurde im Zielscan nicht festgestellt. Die fünf Pfadtreffer sind
Vorkommen derselben offensichtlich fiktiven Adresse eines Firewall-Matcher-
Tests und seiner Auditdokumentation. Der maschinenlesbare Nachweis speichert
Commit- und Tagger-Adressen nur als SHA-256-Fingerabdruck und Domain.

Vor dem Rewrite wurde ein vollständiges privates Git-Bundle außerhalb des
Repositorys erzeugt und mit `git bundle verify` geprüft. Es umfasst 19 damalige
Refs, ist 8.736.205 Bytes groß und besitzt SHA-256
`a10b3ed6b1da32eeb0c46cd02321cfe14348035c54b6fdd648ad4d57b1eef7d3`.
Anschließend wurden `main`, der Vorbereitungsbranch und alle drei Release-Tags
kontrolliert umgeschrieben. Der bereinigte `main`-Tip und die drei neuen
Tagobjekte wurden am 3. September 2026 mit exakten Force-with-Lease-Altwerten auf
das weiterhin private GitHub-Repository übertragen und dort zurückgelesen; das
Repository wurde anschließend in `LocalDeviceTransferGateway-LDTG` umbenannt.
Zehn geschlossene providerseitige Dependabot-PR-Refs und ihre Aktionshistorie
bleiben Bestandteil des bestehenden GitHub-Repositoryverlaufs. Sie sind keine
Branch- oder Tag-Refs und lassen sich durch die gezielte Tagbereinigung nicht
entfernen. Eine vollständig spurenfreie Veröffentlichung würde ein neues
Repository und eine erneute Releasebindung erfordern; dieser Weg wurde zugunsten
der normalen, nachvollziehbaren Projekthistorie nicht gewählt.

## Assets, Texte und erzeugte Dateien

| Gruppe | Befund | Herkunftsstatus |
|---|---|---|
| Quellcode, Projektdokumentation und UI-Texte | keine vendorten Fremdquellen erkannt; nahezu vollständig im Wurzelcommit importiert | vom Owner bestätigt |
| `assets/ldtg-logo-lockup.png`, `assets/ldtg-app-icon.png`, `assets/ldtg-ui-icon.png` | ausgewählte und abgeleitete LDTG-Markenassets, mit KI-Unterstützung erzeugt | vom Owner zur späteren Veröffentlichung freigegeben |
| `src-tauri/icons/**` | mit der Tauri-CLI aus `assets/ldtg-app-icon.png` erzeugte PNG/ICO/ICNS- sowie Android-/iOS-Varianten | reproduzierbar; Quellicon vom Owner bestätigt |
| historische QA-Aufnahmen | Aufnahmen der damaligen LDTG-Oberflächen und daraus erzeugte Vergleiche; vor der Public-Beta-Vorbereitung aus dem aktuellen Arbeitsbaum entfernt | Rechte und Freigabefähigkeit der damaligen Testdaten vom Owner bestätigt; Ergebnis bleibt als Textprotokoll archiviert |
| `packages/shared/src/index.ts` | laut Dateikopf aus Rust-Verträgen erzeugt | im Repository reproduzierbar |
| `src-tauri/gen/schemas/*.json` | Tauri-generierte Konfigurations-/Berechtigungsschemata | Fremdherkunft über Tauri; Notices aus Lizenzinventur ableitbar |
| `pnpm-lock.yaml`, `src-tauri/Cargo.lock` | von pnpm beziehungsweise Cargo erzeugte Auflösung | reproduzierbare Paketmetadaten |
| Fonts | nur lokale Systemfont-Fallbacks, kein `@font-face`, keine Fontdatei | nichts wird gebündelt |
| mobile Webassets | aus eigenem Workspace gebaut und mit `rust-embed` in den Dienst eingebettet; keine externen Laufzeitressourcen | Projektquellen vom Owner bestätigt |

Alle 13 damaligen QA-Oberflächen-/Vergleichsbilder wurden zusätzlich visuell
geprüft und vor der Public-Beta-Vorbereitung aus dem aktuellen Arbeitsbaum
entfernt, weil sie noch die frühere DMDC-Bezeichnung zeigten.
Erkennbar sind nur LDTG-Oberflächen, private LAN-Adressen, generische
`C:\LDTG\...`-Testordner, ein Testprofil sowie Zugangscodes/QR-Codes. Namen,
Benutzerprofile oder Dateiinhalte sind nicht sichtbar. Laut `qa/README.md`
wurden die Codes nach den Laufzustandsaufnahmen durch Stoppen des Dienstes
ungültig. Der Owner hat bestätigt, dass LAN-Adressen, Ordner und Profil reine
freigabefähige Testdaten sind.

Die bestätigte Erklärung steht in
[`provenance-attestation.md`](provenance-attestation.md). Sie dokumentiert die
vom Git-Verlauf allein nicht belegbare Rechtekette vor dem Wurzelcommit.

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
| kein ungeklärtes fremdes/proprietäres Artefakt | erfüllt | technische Prüfung ohne Fremdartefakt-Befund; Herkunft, Assets und QA-Testdaten zusätzlich vom Owner bestätigt |
| ausgelieferte Abhängigkeiten mit bekannter kompatibler Lizenz | erfüllt | 857 Pakete inventarisiert, 0 ohne deklarierte Lizenz; siehe Lizenz-Audit |
| erforderliche Drittanbieterhinweise ableitbar | erfüllt | Lizenz-/Notice-Dateien, Prüfsummen und `THIRD_PARTY_NOTICES.md` sind vorhanden und werden automatisch gegengeprüft |
| keine Secrets oder privaten Nutzdaten im vorgesehenen öffentlichen Git-Stand | erfüllt | exakter Scan von vorgesehenem Quellcommit und drei Tags: 0 Secret- und 0 persönliche Identitätstreffer; die Pfadtreffer sind Vorkommen derselben dokumentierten synthetischen Testadresse; privates Vollbackup verifiziert |

P1 bleibt damit **abgeschlossen**. Die nachfolgenden Vorbereitungspakete P2 bis
P6 sind ebenfalls abgeschlossen. R5.2, Phase 6, SignPath und Veröffentlichung
werden durch dieses Audit weder begonnen noch freigegeben.
