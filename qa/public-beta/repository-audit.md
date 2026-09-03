# P1-Repository-, Herkunfts- und Datenschutz-Audit

Stand: 3. September 2026

Auditierter Quellstand: `af632670b29346f1700f97e1bc00048712a00475`

Ergebnis: **BLOCKIERT – technische Prüfung abgeschlossen, zwei Owner-Entscheidungen offen**

Dieses Dokument prüft ausschließlich P1 des
[Plans bis zur Veröffentlichungsentscheidung](../../docs/PUBLIC_BETA_PLAN.md).
Es aktiviert keine Projektlizenz, veröffentlicht nichts und autorisiert weder
SignPath noch kostenpflichtige Dienste. R5.2 und Phase 6 bleiben pausiert.

## Kurzurteil

- Im Arbeitsbaum und in allen lokal erreichbaren Git-Objekten wurden keine
  bekannten Token-, Schlüssel- oder Zugangsdatenmuster gefunden. Ein
  Regex-Audit ersetzt keine mathematische Garantie, liefert für den geprüften
  Stand aber keinen Secret-Befund.
- Der aktuelle Arbeitsbaum enthält nach einer lokalen Textredaktion keinen
  absoluten persönlichen Benutzerprofilpfad. Die alte Git-Historie enthält
  weiterhin eine persönliche Commit-/Tag-E-Mail-Adresse und frühere absolute
  Benutzer- und Werkzeugpfade.
- Es wurden keine eingecheckten ausführbaren Fremdbinärdateien, Archive, Fonts,
  PDFs oder WebAssembly-Dateien gefunden. Die 58 PNG/JPEG-Dateien enthalten
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

Das P1-Gate bleibt deshalb an `PB-01` und `PB-02` aus
[`blockers.json`](blockers.json) hängen.

## Prüfumfang und reproduzierbare Nachweise

Der maschinenlesbare Nachweis
[`repository-evidence.json`](repository-evidence.json) wurde mit
[`scripts/public-beta-audit.mjs`](../../scripts/public-beta-audit.mjs) erzeugt.
Geprüft wurden der Arbeitsbaum, alle mit `git rev-list --objects --all`
erreichbaren Objekte, lokale Branch-/Remote-/Tag-Refs, der vollständige Patch-
Verlauf sowie alle eingecheckten PNG-/JPEG-Dateien.

Wesentliche Kennzahlen des Ausgangsstands:

| Merkmal | Ergebnis |
|---|---:|
| lokal erreichbare Commits über alle Refs | 39 |
| geprüfte Refs | 17 |
| eingecheckte Dateien | 178 |
| Größe der eingecheckten Dateien | 4.252.726 Bytes |
| erreichbare Git-Objekte / eindeutige Blobs | 848 / 503 |
| größter Blob | 528.331 Bytes |
| Blobs ab 5 MiB | 0 |
| geprüfte PNG/JPEG-Dateien | 58 |

Die 17 Refs umfassen `main`, den Audit-Branch, `origin/main`, neun lokale
Dependabot-PR-Refs, drei Tags und einen lokalen Codex-Diff-Capture-Ref. Ein
späterer öffentlicher Push darf nur die bewusst ausgewählten Branches und Tags
enthalten; lokale Werkzeugrefs gehören nicht in den Zielzustand.

Ausgeführt beziehungsweise ausgewertet wurden unter anderem:

```powershell
git rev-list --objects --all
git cat-file --batch-check="%(objectname) %(objecttype) %(objectsize)"
git log --all -p --no-ext-diff --no-textconv
git for-each-ref
pnpm audit:public-beta
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
| persönliche absolute Pfade | 5 Mustertreffer |
| E-Mail-ähnliche Zeichenfolgen | 100 Mustertreffer |

Die Zahlen zählen Vorkommen in wiederholten Patches und sind keine Anzahl
eindeutiger Probleme. Die persönlichen Pfadtreffer gehen auf zwei historische
Sachverhalte zurück: einen inzwischen redigierten Benutzerprofilbezug in
`docs/RE_AUDIT_REPORT_2026-08-30.md` und frühere fest eingetragene
Benutzer-/Werkzeugpfade in `qa/make_comparison.py`. Die aktuelle Fassung des
Skripts besitzt solche Pfade nicht.

Die E-Mail-Treffer enthalten öffentliche Dependency-Beispiele und
GitHub-Adressen, vor allem aber die persönliche Adresse in 30
Autor-/Committer-Identitäten sowie in annotierten Tags. Der maschinenlesbare
Nachweis speichert davon nur SHA-256-Fingerabdruck und Domain, nicht die Adresse
selbst. Für künftige Commits ist lokal im Repository die GitHub-noreply-Adresse
konfiguriert. Diese lokale Einstellung ändert die bestehende Historie nicht und
ersetzt nicht die optionale GitHub-Kontoeinstellung zum Verbergen der Adresse
oder Blockieren versehentlicher Pushes.

Vor einem Historien-Rewrite sind gemäß Plan eine getrennte Sicherung und eine
ausdrückliche Freigabe erforderlich. Zulässige Owner-Entscheidungen sind:

1. die bestehenden Offenlegungen bewusst akzeptieren;
2. nach privater Sicherung alle vorgesehenen öffentlichen Branches und Tags
   kontrolliert umschreiben;
3. einen neuen beziehungsweise gesquashten öffentlichen Verlauf aus dem
   bereinigten Arbeitsbaum beginnen und die bisherige Historie privat halten.

Bis zu dieser Entscheidung ist die vorhandene Historie kein freigegebener
öffentlicher Git-Stand.

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

Alle zehn QA-Oberflächen-/Vergleichsbilder wurden zusätzlich visuell geprüft.
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
| keine Secrets oder privaten Nutzdaten im vorgesehenen öffentlichen Git-Stand | blockiert | bereinigter Arbeitsbaum unauffällig, aber Zielstrategie für E-Mail-/Pfadspuren in der Historie fehlt |

P1 ist daher **nicht abgeschlossen**. P2, P3, R5.2, Phase 6, Lizenzaktivierung,
SignPath und Veröffentlichung werden durch dieses Audit weder begonnen noch
freigegeben.
