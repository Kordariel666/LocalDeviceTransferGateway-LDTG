# DMDC Entwicklungsroadmap

Stand: 2. September 2026  
Ausgangsversion: 0.1.3  
Ziel: DMDC schrittweise von einem sicherheitsgehärteten v1-Kern zu einer wartbaren, alltagstauglichen und veröffentlichungsreifen Anwendung ausbauen.

## 1. Feste Leitplanken

Diese Eigenschaften gelten für alle Phasen als unveränderliche Abnahmekriterien, solange nicht ausdrücklich eine neue Hauptversion mit aktualisiertem Threat Model beschlossen wird:

- Downloads bleiben ausschließlich lesbar.
- Uploads bleiben add-only und überschreiben keine vorhandenen Dateien.
- Der Upload-Eingang wird gegenüber LAN-Clients nicht aufgelistet.
- Start, Stop, Konfiguration, Firewall und Diagnose bleiben ausschließlich über die lokale Tauri-Oberfläche erreichbar.
- Zugangscode und Sitzungstoken erscheinen weder in URL/QR-Code noch in Logs oder Diagnoseexporten.
- Download- und Uploadwurzeln bleiben vollständig getrennt und gegen Pfad-, Link- und Namespace-Wechsel abgesichert.
- Der Dienst bindet nur an die ausgewählte private IPv4-Adresse und stoppt bei einem relevanten Netzwerk- oder Rootwechsel.
- Neue Funktionen erhalten weiterhin feste globale, gerätebezogene und soweit nötig sitzungsbezogene Ressourcenlimits.
- Persistente Verlaufs- oder Geräteinformationen sind datensparsam, transparent und löschbar.

## 2. Aktueller verifizierter Ausgangsstand

- 90 Rust-Tests bestehen.
- 9 Desktop- und 28 Mobile-Tests bestehen.
- TypeScript-Typprüfung, `cargo fmt --check` und Clippy mit `-D warnings` bestehen.
- Beide Produktions-Webbuilds bestehen.
- Der verifizierte Ausgangsstand ist im Git-Commit `d4e4751` und Tag `v0.1.3`
  gesichert; nachfolgende Arbeit wird in thematischen Commits fortgeführt.
- Die Windows-Netzwerkprofilerkennung sowie der echte Start-/Stopp-Ablauf wurden
  am 2. September 2026 auf `Ethernet` mit einem öffentlichen Windows-Profil geprüft.
- Ein Online-Abhängigkeitsaudit und die physische Windows-/Browser-Abnahmematrix sind noch nicht Bestandteil dieses Nachweises.

## 3. Definition of Done

Ein Arbeitspaket ist erst abgeschlossen, wenn:

1. Verhalten und Fehlerfälle spezifiziert sind.
2. Die Änderung in kleine, nachvollziehbare Module eingeordnet wurde.
3. Relevante Unit-, Integrations- oder UI-Regressionstests bestehen.
4. Typecheck, Frontendtests, Rust-Tests, Formatierung, Clippy und Webbuild bestehen.
5. API-, Architektur-, Sicherheits- und Bedienungsdokumentation mit dem Code übereinstimmen.
6. Neue Dateisystem-, Netzwerk- oder Persistenzpfade gegen die festen Leitplanken geprüft wurden.
7. Für UI-Änderungen aktuelle Desktop- und Mobile-Screenshots erzeugt und visuell geprüft wurden.
8. Keine temporären Dateien, Diagnosewerte, Zugangsdaten oder benutzerspezifischen Pfade eingecheckt werden.

## 4. Release- und Phasenübersicht

| Phase | Zielrelease | Schwerpunkt | Abhängigkeit | Relative Größe |
|---|---|---|---|---|
| 0 | Baseline | Git-Sicherung, Dokumentationswahrheit, CI-Grundlage | keine | S |
| 1 | 0.1.4 | Korrektheit, Queue-Robustheit, Async-I/O, Einstellungen | Phase 0 | M |
| 2 | 0.2.0-alpha | Typisierte Verträge, modulare Architektur, Statuspfad | Phase 1 | M–L |
| 3 | 0.2.0 | Transferkomfort und Desktopintegration | Phase 2 | M |
| 4 | 0.2.x | Netzwerkvertrauen und Geräteverwaltung | Phase 2 | M |
| 5 | 0.3.0 | Freigabeprofile und optional mehrere benannte Freigaben | Phasen 3–4 | L |
| 6 | 0.4.0 | Inhaltsgeprüfte Wiederaufnahme und Sammeltransfers | Phase 5 | L–XL |
| 7 | 1.0-Kandidat | Release-Härtung, Signierung, Updatepfad, reale Abnahme | alle freizugebenden Phasen | L |

Die Größen sind relativ: S entspricht ungefähr einem kleinen, M mehreren zusammenhängenden und L einem größeren Arbeitspaket mit eigenem Design- und Testzyklus.

## 5. Phase 0 – Baseline und belastbare Arbeitsweise

### R0.1 Git-Ausgangsstand herstellen

Status: abgeschlossen am 2. September 2026.

Aufgaben:

- `.gitignore` gegen Build-, IDE-, Log-, QA- und temporäre Artefakte prüfen.
- Projektdateien auf Zugangsdaten, persönliche Pfade und versehentlich erzeugte Binärartefakte prüfen.
- Den aktuell verifizierten Stand als initialen Commit sichern.
- Danach ausschließlich kleine thematische Commits pro Arbeitspaket verwenden.
- Für Releases Tags und ein Changelog-Schema festlegen.

Abnahme:

- `git status` ist nach dem Baseline-Commit sauber.
- Ein kompletter Test-/Buildnachweis gehört zum Commit oder Releaseprotokoll.
- Ein Rollback auf den unveränderten Ausgangsstand ist möglich.

### R0.2 Dokumentation mit dem Code synchronisieren

Status: abgeschlossen am 2. September 2026.

Aufgaben:

- Veraltete Aussagen in `docs/ARCHITECTURE.md` korrigieren: Crash-Partials werden bewahrt, es gibt keine deklarierte Restgrößenreservierung, Zwischenchunks sind exakt 8 MiB groß, der Code rotiert bei globaler Sperre nicht und korrekte Codes werden während der Sperre ebenfalls blockiert.
- Nicht existente Reservierungsfehler aus `docs/API.md` entfernen und die tatsächlich implementierten Inbox-Limits dokumentieren.
- Die Aussage über abbrechbare Retry-Backoffs erst nach R1.1 wieder aufnehmen.
- Historische Auditberichte klar als historische Momentaufnahme kennzeichnen; aktuelle Abschlussberichte bleiben die maßgebliche Referenz.
- QA-Screenshots und Vergleichsbilder mit dem aktuellen achtstelligen Code neu erzeugen.
- Falsch benannte beziehungsweise nicht ihrem Dateiformat entsprechende QA-Bilder korrigieren.

Abnahme:

- Jede dokumentierte API-Route und jeder genannte Fehlercode lässt sich im aktuellen Code wiederfinden.
- README, API, Architektur, Security und Testplan widersprechen sich nicht.
- Aktuelle Screenshots zeigen ausschließlich den aktuellen UI- und Protokollstand.

### R0.3 Kontinuierliche Prüfungen einführen

Status: abgeschlossen am 2. September 2026.

Aufgaben:

- Windows-CI für Typecheck, Frontendtests, Rust-Tests, Formatierung, Clippy und Webbuild hinzufügen.
- ESLint für React/TypeScript mit Hooks-Regeln einführen.
- Coverage zunächst sichtbar machen, danach realistische Mindestwerte pro Modul festlegen.
- Dependency-Update-Automation erst nach Festlegung der Review- und Lockfile-Strategie aktivieren.
- Einen dokumentierten lokalen Befehl für die vollständige Prüfkette bereitstellen.

Abnahme:

- Jeder Pull Request erhält reproduzierbare grüne oder rote Qualitäts-Gates.
- Fehler der Laufzeitumgebung werden klar von Produktfehlern getrennt.

Umgesetzt:

- `pnpm check` bildet lokal und in GitHub Actions dieselbe vollständige Prüfkette ab.
- ESLint prüft React/TypeScript einschließlich der empfohlenen Hooks- und React-Refresh-Regeln; drei absichtliche Lifecycle-Abhängigkeiten sind eng begrenzt und im Code begründet.
- V8-Coverage ist für Desktop und Mobile sichtbar. Der erste Nachweis erreicht 77,49 % beziehungsweise 92,3 % Statements/Lines; verbindliche Mindestwerte werden nach dem ersten Trend festgelegt.
- Dependabot ist mit wöchentlichen, gruppierten Minor-/Patch-Aktualisierungen ohne Auto-Merge eingerichtet; Review-, Lockfile- und Sicherheitsregeln stehen in `docs/DEPENDENCIES.md`.
- Die komplette Prüfkette bestand lokal mit 9 Desktop-, 14 Mobile- und 90 Rust-Tests sowie beiden Produktions-Webbuilds.

## 6. Phase 1 – Korrektheit und Laufzeitrobustheit

### R1.1 Mobile Uploadwarteschlange deterministisch machen

Status: abgeschlossen am 2. September 2026.

Aufgaben:

- Einen expliziten Uploadzustandsautomaten beziehungsweise reducer-basierten Queue-Kern einführen.
- Retry-Verzögerungen mit einem abbrechbaren Signal implementieren, sodass Pause, Abbruch und Sitzungsverlust sofort reagieren.
- Nach Pause oder Abbruch darf die nächste Datei ohne unnötige Backoff-Wartezeit starten.
- XHR-Fehler als `ApiError` parsen und stabile Backendcodes erhalten.
- Logout lokal auch bei Verbindungsverlust zuverlässig abschließen; technische Fehler optional anzeigen.
- Mehrfaches Absenden des Loginformulars während einer laufenden Anmeldung verhindern.
- Fehlgeschlagene Uploads beim Retry sichtbar wieder in `queued`/`uploading` überführen.

Regressionstests:

- Pause und Abbruch in jeder Retry-Stufe.
- Queue-Fortschritt unmittelbar nach Pause/Abbruch der ersten Datei.
- Strukturierte Anzeige eines PATCH-Fehlers.
- Doppelklick auf Login erzeugt höchstens eine neue Sitzung.
- Logout bei bereits gestopptem Dienst räumt den lokalen Zustand auf.

Umgesetzt:

- Ein reiner Reducer verwaltet Reihenfolge, Zustände, Fortschritt, Server-ID und
  ausstehende Uploads als einzige fachliche Queue-Zustandsquelle.
- Pause, Abbruch und Sitzungsverlust unterbrechen Chunk-Requests, Statusabfragen,
  Create-Wartezeit und alle Retry-Stufen. Eine bereits gesendete Create-Anfrage
  darf serverseitig zu Ende laufen, wird clientseitig aber als geteiltes Promise
  nachverfolgt, damit weder die Queue blockiert noch eine zweite Upload-ID entsteht.
- PATCH-Fehler übernehmen strukturierte `ApiError`-Codes; dauerhafte 4xx-Fehler
  wechseln ohne sinnlose Wiederholungen in `failed`, während transiente Fehler
  weiterhin begrenzt wiederholt werden.
- Login ist während der laufenden Anmeldung gegen Mehrfachabsenden gesperrt.
  Logout räumt den lokalen Zustand in einem `finally`-Pfad auf und zeigt einen
  nicht bestätigten Server-Logout als technischen Hinweis an.
- 14 neue Mobile-Regressionstests prüfen Reducer-Invarianten, alle drei
  abbrechbaren Backoff-Stufen, unmittelbaren Queue-Fortschritt, strukturierte
  PATCH-Fehler, sichtbaren Retry, Doppel-Login und Offline-Logout.

### R1.2 Blockierende Uploadarbeit aus Async-Workern entfernen

Status: abgeschlossen am 2. September 2026.

Aufgaben:

- Inbox-Scan, Dateischreiben, `sync_data` und weitere blockierende Dateisystemarbeit in begrenzte `spawn_blocking`-Jobs verschieben.
- Bestehende globale und IP-bezogene Dateisystemlimits auf diese Jobs anwenden oder einen separaten fairen Upload-I/O-Pool definieren.
- Stabile Dateihandles, exakter Offset, Uploadbesitz und Cancellation-Sicherheit beibehalten.
- Festlegen, wann eine Chunk-Antwort als dauerhaft gespeichert gilt; diese Semantik dokumentieren.
- Event- und Progressfrequenz messen, damit schnelle Transfers nicht durch Statusarbeit ausgebremst werden.

Regressionstests:

- Langsamer `sync_data` blockiert weder neue Verbindungen noch Stop/Shutdown.
- Abgebrochene HTTP-Waiter geben Blocking-Permits nicht zu früh frei.
- Parallele Uploads überschreiten den I/O-Pool nicht.
- Offset, Bytebudget und Transferstatus bleiben nach Schreibfehlern konsistent.

Umgesetzt:

- Uploadanlage, Inbox-Scan, Speicherprüfung, Chunk-Schreiben, `sync_data`,
  Abschlussprüfung, Veröffentlichung und Live-Partial-Löschung laufen außerhalb
  der Async-Worker in begrenzten Blocking-Jobs. Auch die periodische Rootprüfung
  teilt sich nun den abbrechbaren Blocking-Pfad der Netzwerkprüfung.
- Ein eigener fairer Upload-I/O-Pool erlaubt höchstens vier clientgetriebene
  Dateisystemjobs gleichzeitig und höchstens zwei pro Client-IP. Die bereits vor
  dem Body geltende Grenze von acht Chunks und genau einem Chunk pro Upload-ID
  bleibt als separate frühe Schutzschicht bestehen.
- Offene Partial-Dateien werden über stabile geteilte Handles und positionsfeste
  Schreibzugriffe verwendet. Offset, Aktivitätszeit, Bytebudget und
  Transferfortschritt wechseln erst nach erfolgreichem `sync_data` gemeinsam auf
  den neuen Stand; RAII-Reservierungen rollen Fehler und Taskabbrüche zurück.
- Create- und Chunkarbeit bleibt nach Verlust des HTTP-Waiters dienstbesessen.
  Blocking-Permits werden bis zum tatsächlichen Jobende gehalten. Stop,
  Sitzungswiderruf und Ablauf können während langsamer I/O sofort signalisieren;
  die exklusive, begrenzte Löschung folgt nach dem laufenden Job.
- Pro erfolgreich bestätigtem Block entsteht genau ein Progress-Update. Wegen
  der festen Zwischenblockgröße entspricht das höchstens einem Update je 8 MiB;
  nur der letzte Block darf kleiner sein.
- Fünf neue Rust-Regressionstests prüfen globale und IP-bezogene I/O-Grenzen,
  Permitbesitz nach Waiter-Abbruch, reaktionsfähige Dienstbereinigung, den
  dienstbesessenen Abschluss eines Chunks sowie konsistente Fehlerzustände.

### R1.3 Versionierte Einstellungs-Migration

Status: abgeschlossen am 2. September 2026.

Aufgaben:

- `version` als echtes Konfigurationsschema behandeln.
- Migrationen schrittweise und idempotent implementieren.
- Unbekannte neuere Schemas sicher ablehnen, statt sie still zu übernehmen.
- Semantisch ungültige, aber syntaktisch korrekte Einstellungen erkennen und mit Recovery-Backup behandeln.
- Die laufende Appversion nicht als veraltbaren Benutzerwert persistieren; Diagnose und UI lesen die Buildversion direkt.
- Migrationstests für fehlende Felder, alte Version, zukünftige Version und beschädigte Werte ergänzen.

Umgesetzt:

- `version` bezeichnet nun ausschließlich das aktuelle Konfigurationsschema 2.
  Versionslose Dateien werden als Schema 0 eingeordnet und anschließend wie
  Schema 1 schrittweise migriert; jeder Schritt lässt bereits migrierte Daten
  unverändert.
- Schema 1 verliert beim Übergang auf Schema 2 das früher persistierte
  `uiVersion`. Die Desktopdiagnose erhält die tatsächliche Buildversion separat
  aus dem Backend, und der Diagnoseexport liest sie weiterhin direkt aus dem
  laufenden Build.
- Ein neueres als das unterstützte Schema, eine ungültige Versionsangabe,
  strukturell beschädigte Werte und semantisch unzulässige Grenzen aktivieren
  sichere Standardwerte mit sichtbarer Warnung. Die Quelldatei bleibt bis zu
  einem bewussten Speichern unverändert und wird davor als nummerierte
  Recovery-Datei gesichert.
- Der atomare Speicherpfad validiert erneut, normalisiert ältere Entwürfe auf
  Schema 2 und verweigert zukünftige Schemata. Erfolgreiche Migrationen werden
  beim nächsten Speichern reproduzierbar im aktuellen Format persistiert.
- Sechs zusätzliche Rust-Regressionstests prüfen fehlende Felder, schrittweise
  idempotente Altversionen, zukünftige Versionen, falsche Feldtypen sowie
  semantische Fehler samt Recovery. Ein Desktoptest verankert die vom
  Konfigurationsschema unabhängige Buildversionsanzeige.

### R1.4 Desktop-Validierung und ungespeicherte Änderungen

Status: abgeschlossen am 2. September 2026.

Aufgaben:

- Einen expliziten Dirty-State anzeigen und Speichern nur bei tatsächlichen Änderungen aktivieren.
- Port-, Größen-, Datei- und Sharevalidierung feldbezogen darstellen.
- Gleiche, verschachtelte und kanonisch überlappende Freigaben über einen Backend-Validierungsbefehl bereits vor dem Start melden.
- Beim Verlassen einer Seite keine Entwürfe verlieren; vor Beenden mit ungespeicherten Änderungen sinnvoll warnen.

Umgesetzt:

- Der Desktop vergleicht den bearbeiteten Entwurf strukturell mit dem letzten
  gespeicherten Snapshot. Ein global sichtbarer Dirty-Hinweis erscheint nur bei
  einer Abweichung; die Speichern-Schaltflächen sind ausschließlich dann und bei
  fehlerfreiem Entwurf aktiv. Die bewusste Übernahme sicherer Standardwerte nach
  einer Recovery-Warnung bleibt als ausdrücklich gekennzeichnete Ausnahme möglich.
- Port, Uploadgröße, Inbox-Gesamtgröße, Inbox-Dateizahl und aktivierte Freigaben
  besitzen direkt zugeordnete Fehlermeldungen, `aria-invalid` und beschreibende
  Hilfetexte. Abhängige Größenlimits markieren beide verantwortlichen Felder.
- Der neue Tauri-Befehl `validate_share_settings` prüft aktivierte Ordner in
  einem Blocking-Task mit denselben kanonischen Pfad- und Sicherheitsregeln wie
  der Dienststart. Feldfehler und gleiche, verschachtelte oder anderweitig
  kanonisch überlappende Wurzeln erscheinen bereits im Entwurf; ein ausstehendes
  oder negatives Prüfergebnis blockiert Speichern, Start und Firewalländerung.
- Der Entwurf bleibt beim Wechsel zwischen allen Desktopseiten erhalten.
  Browser-Unload, natives Fensterschließen und Beenden aus dem Tray kennen den
  Dirty-State. Ein tatsächliches Beenden erfordert eine ausdrückliche Bestätigung
  zum Verwerfen; die getrennte Warnung vor laufenden Übertragungen bleibt erhalten.
- Zwei Rust-Regressionstests prüfen feldbezogene und kanonische
  Freigabevalidierung. Vier neue Desktoptests verankern Dirty-State,
  feldbezogene Validierung, Backend-Überlappung und den nativen Quit-Pfad; der
  bestehende Entwurfstest deckt weiterhin Seitenwechsel und Hintergrundereignisse ab.

Phasen-Gate 1:

Status: erfüllt am 2. September 2026.

- Sämtliche bisherigen Tests plus neue Regressionstests bestehen.
- Unter künstlich langsamem Datenträger bleiben UI, Accept-Loop und Stop reaktionsfähig.
- Konfigurationsupgrade und -recovery sind reproduzierbar.

Nachweis: Das vollständige Qualitätsgate besteht mit 103 Rust-, 14 Desktop- und
28 Mobile-Tests. Die R1.2-Regressionen halten Accept, Stop und Bereinigung bei
blockierter Dateiarbeit reaktionsfähig; R1.3 prüft Upgrade und Recovery
einschließlich beschädigter, zukünftiger und semantisch ungültiger Konfigurationen.

## 7. Phase 2 – Wartbare Architektur

### R2.1 Verträge aus einer Quelle generieren

Status: abgeschlossen am 2. September 2026.

Aufgaben:

- Rust-DTOs als maßgebliche Quelle definieren und TypeScript-Verträge daraus generieren oder ein gemeinsames Schema verwenden.
- Generierung für `AppSettings`, `AppSnapshot`, `ServiceStatus`, Sessions, Transfers und HTTP-Antworten einführen.
- Stringfelder mit endlicher Wertemenge in echte Enums/Unions überführen.
- Einen CI-Check ergänzen, der nicht aktualisierte generierte Verträge erkennt.

Umgesetzt:

- Die serialisierbaren Rust-DTOs in `domain` sind die maßgebliche Quelle für
  Desktop-, Mobile- und HTTP-Verträge. `ts-rs` leitet daraus deterministisch das
  öffentliche `@dmdc/shared`-Modul ab; der frühere manuelle TypeScript-Bestand
  wurde vollständig ersetzt.
- Der Export umfasst Einstellungen, App-Snapshot, Netzwerk- und Firewallstatus,
  Sitzungen, Transfers, Freigabevalidierung sowie sämtliche strukturierten
  HTTP-Antworten einschließlich Fehler- und Uploadabschlusskörpern. Die für JSON
  sicher als JavaScript-Zahlen übertragenen 64-Bit-Felder sind im Vertrag
  ausdrücklich als `number` markiert.
- Dienstzustand, Transferrichtung, Transferzustand und Art eines Downloadeintrags
  sind nun echte Rust-Enums. Serde erzeugt daraus dieselben stabilen Kleinbuchstabenwerte
  wie zuvor; TypeScript erhält benannte String-Unions statt duplizierter Literale.
- `pnpm contracts:generate` aktualisiert die Datei bewusst, während
  `pnpm contracts:check` ausschließlich vergleicht. Der Drift-Check läuft als
  erster Bestandteil von `pnpm check` und damit identisch lokal und im
  Windows-CI-Gate.
- Rust-Kompilation, 103 Rust-Tests und die TypeScript-Prüfung beider Frontends
  bestätigen die unveränderte Serialisierung und vollständige Konsumierbarkeit
  des generierten Vertrags.

### R2.2 Strukturierte Tauri-Fehler

Status: abgeschlossen am 2. September 2026.

Aufgaben:

- Delimiter-Strings wie `NETWORK_UNTRUSTED|...`, `BROAD_SHARE|...` und `ACTIVE_TRANSFERS|...` durch serialisierbare Fehlerobjekte ersetzen.
- Stabile Fehlercodes, sichere Nutzermeldung und typisierte Kontextfelder definieren.
- Desktop-Dialoge ausschließlich anhand von Fehlercodes und Kontext aufbauen.
- Interne Fehlerdetails weiterhin nur datensparsam protokollieren.

Umgesetzt:

- Alle falliblen Tauri-Befehle verwenden an der IPC-Grenze denselben generierten
  `CommandError` mit stabilem `CommandErrorCode`, sicherer Nutzermeldung und
  optionalem diskriminiertem `CommandErrorContext`. Nackte Anwendungsfehlerstrings
  verlassen das Rust-Backend nicht mehr.
- Netzwerkvertrauen und breite Freigaben liefern Bestätigungstoken sowie
  Netzwerkname beziehungsweise Pfad in typisierten Kontextvarianten. Aktive
  Übertragungen liefern ihre Anzahl; ungespeicherte Änderungen benötigen keinen
  zusätzlichen Kontext.
- Der Desktop erkennt Bestätigungsabläufe nur noch über exakte Fehlercodes und
  passende Kontextarten. Sämtliches `startsWith`-, Delimiter- und
  `split("|")`-Parsing wurde entfernt; ein Regressionstest verwendet bewusst
  einen Freigabepfad mit `|`.
- Unerwartete Task-, Datei-, Firewall- und Dienstfehler werden auf konstante,
  sichere UI-Texte abgebildet. Das lokale Log enthält dazu ausschließlich den
  stabilen Fehlercode und die betroffene Operation, niemals rohe interne Details
  oder darin vorkommende Pfade.
- Der generierte Shared-Vertrag enthält Fehlercodes, Kontextunion und Fehlerkörper.
  Zwei Rust-Regressionstests verankern JSON-Form und Detailabschirmung; die nun
  15 Desktoptests decken Netzwerk-, Freigabe-, Stop- und Quit-Bestätigungen ab.

### R2.3 Große Module zerlegen

Status: abgeschlossen am 2. September 2026.

Zielstruktur:

- `service/api/`: Auth, Directory, Download, Upload, Static Assets und gemeinsame Fehler/Middleware.
- `service/state/`: Sessions, Limits, Cursors, Uploads, Downloads und Transferjournal.
- `apps/mobile/`: API-Client, Session-Hook, Directory-Browser, Upload-Queue und Präsentationskomponenten.
- `apps/desktop/`: Tauri-Client, Lifecycle-Hook, Settings-Draft und Seitenkomponenten.

Zusätzlich:

- Nicht verwendete alte Stylesheets entfernen oder klar archivieren.
- Testhilfen aus Produktionsdateien in dedizierte Testmodule verschieben, soweit dies die Invarianten nicht versteckt.
- Keine Verhaltensänderung in reinen Refactoring-Commits.

Umgesetzt:

- Der HTTP-Einstieg `service/api.rs` enthält nur noch Router und Modulverdrahtung.
  Authentifizierung, gemeinsame Fehler und Middleware, Verzeichnisauflistung,
  Download, Upload und eingebettete Assets liegen in benannten Untermodulen.
- `service/state.rs` beschränkt sich auf gemeinsame Datentypen, Konstanten und
  Initialisierung. Sitzungen, Ressourcenlimits, Verzeichniscursor, Uploads,
  Downloads und Transferjournal besitzen getrennte Implementierungsmodule.
- Die zuvor eingebetteten Rust-Testmodule wurden nach `api/tests.rs` und
  `state/tests.rs` verschoben. Damit liegen umfangreiche Fixtures und
  Parallelitätsregressionen nicht mehr in Produktionsdateien, prüfen aber
  weiterhin dieselben privaten Invarianten.
- Mobile trennt HTTP-Client, Session-State, Directory-Browser,
  Upload-Queue-Darstellung und Formatierung. Desktop trennt Tauri-Client,
  Lifecycle, Settings-Draft, Basiskomponenten und Seiten.
- Die beiden nicht importierten komprimierten Alt-Stylesheets `styles.css`
  wurden entfernt; die aktiven Redesign-Stylesheets bleiben unverändert.
- Das vollständige Qualitätsgate bleibt mit 105 Rust-, 15 Desktop- und
  28 Mobile-Tests grün. Der Schritt ist ein reines Refactoring ohne
  Vertrags- oder Verhaltensänderung.

### R2.4 Stabiler und sparsamer Statuspfad

Status: abgeschlossen am 2. September 2026.

Aufgaben:

- Verhindern, dass `try_lock` bei kurzer Sperrbelegung scheinbar leere Session- oder Transferlisten liefert.
- Transferfortschritt im Backend zeitlich oder bytebasiert drosseln.
- Kleine Event-Payloads direkt in der Desktop-App anwenden; vollständigen Status nur zur Resynchronisierung abrufen.
- Polling als Fallback beibehalten, aber unnötige Vollabfragen reduzieren.

Umgesetzt:

- `TransferServiceState::status` erzeugt Sitzungs- und Transfersnapshots nun
  asynchron über die regulären Mutex-Sperren. Kurze parallele Änderungen lassen
  den Status warten und können nicht mehr als scheinbar leere Listen erscheinen.
- Der maßgebliche Transferzustand wird weiterhin nach jedem gelesenen oder
  geschriebenen Block aktualisiert. Ein UI-Ereignis folgt dagegen nur bei einem
  Terminalzustand, bei einem Fortschrittsupdate nach mindestens 250 Millisekunden
  oder nach mindestens 1 MiB zusätzlichem Fortschritt.
- Die generierten Ereignisverträge `SessionChangedEvent` und
  `TransferChangedEvent` enthalten die Dienst-ID. Der Desktop verwirft dadurch
  verspätete Ereignisse einer alten Dienstinstanz und wendet passende
  Upserts, Entfernungen, Resets und Transferfortschritte direkt lokal an.
- Lifecycle- und Netzwerkereignisse lösen weiterhin eine gedrosselte
  Resynchronisierung aus. Das Vollstatus-Polling bleibt als Ausfallsicherung
  erhalten, läuft aber nur noch alle 30 statt alle 5 Sekunden.
- Zwei neue Rust-Regressionen prüfen vollständige Statussnapshots unter
  Sperrbelegung und die Fortschrittsdrosselung. Vier neue Desktoptests prüfen
  Ereignisreducer, direkte UI-Anwendung ohne Vollabfrage und das
  30-Sekunden-Fallback. Das vollständige Gate ist mit 107 Rust-, 19 Desktop-
  und 28 Mobile-Tests grün.

Phasen-Gate 2:

Status: erfüllt am 2. September 2026.

- Kein manueller Rust/TypeScript-Vertragsduplikatbestand bleibt übrig.
- Frontends verarbeiten keine Backendfehler mehr durch String-Splitting.
- Die Modulgrenzen sind dokumentiert und die Tests bleiben mindestens gleichwertig.

## 8. Phase 3 – Transferkomfort für Version 0.2

### R3.1 Erweiterte Queue-Steuerung

Status: abgeschlossen am 3. September 2026.

- „Alle pausieren“, „Alle fortsetzen“, „Fehlgeschlagene wiederholen“ und „Erledigte entfernen“ ergänzen.
- Einzelne wartende Elemente aus der Queue entfernen und optional umsortieren.
- Summenfortschritt für einen Dateibatch anzeigen.
- Queue-Zustand bei Sitzungsverlust eindeutig erklären.

Umgesetzt:

- Die mobile Batchleiste pausiert und setzt alle noch steuerbaren Uploads fort,
  stellt sämtliche fehlgeschlagenen Dateien in stabiler Reihenfolge erneut an
  und entfernt abgeschlossene oder abgebrochene Einträge gesammelt.
- Wartende Dateien lassen sich einzeln aus der Queue entfernen. Bereits laufende
  oder finalisierende Einträge können dadurch nicht versehentlich aus der
  Zustandsverwaltung verschwinden; eine optionale manuelle Umsortierung wurde
  bewusst nicht ergänzt.
- Der Gesamtfortschritt wird über die Dateigrößen gewichtet und zeigt
  übertragene Bytes sowie erledigte und gesamte Dateien. Abgebrochene Dateien
  gelten als erledigt, ihre nicht übertragenen Bytes werden jedoch nicht als
  Fortschritt ausgegeben.
- Bei Sitzungsverlust werden servergebundene IDs und nicht mehr belastbare
  Fortschrittswerte verworfen. Wartende oder laufende Dateien starten nach der
  nächsten Anmeldung neu, ausdrücklich pausierte Dateien bleiben pausiert. Eine
  ausblendbare Meldung erklärt dieses Verhalten; ohne wiederherstellbare Queue
  erscheint keine irreführende Verlustmeldung.
- Reducer- und Oberflächentests prüfen Sammelaktionen, Reihenfolge, Entfernen,
  größenbewerteten Fortschritt und den sichtbaren Sitzungsverlust. Das
  vollständige Qualitätsgate bleibt mit 107 Rust-, 19 Desktop- und 36
  Mobile-Tests grün.

### R3.2 Geschwindigkeit, Dauer und ETA

- Startzeit, letzte Fortschrittszeit, geglättete Geschwindigkeit und verbleibende Zeit modellieren.
- Unbekannte oder instabile ETA ehrlich kennzeichnen.
- Desktop und Mobile verwenden dieselbe Formatierungslogik.
- Fortschrittswerte dürfen keine zusätzliche Dateiinhalts- oder Pfadfreigabe erzeugen.

### R3.3 Benachrichtigungen und automatisches Ende

- Lokale Desktop-Benachrichtigung für Batch abgeschlossen, fehlgeschlagen und Netzwerkverlust anbieten.
- „Dienst stoppen, wenn alle aktuellen Übertragungen beendet sind“ als einmalige Laufzeitoption ergänzen.
- Bestehenden allgemeinen Idle-Timeout unverändert daneben anbieten.
- Niemals ohne sichtbare Aktivierung einen laufenden Dienst stoppen.

### R3.4 Verlauf sinnvoll ausbauen

- Verlauf um Start, Ende, Dauer und Ergebnis ergänzen.
- Filter nach Richtung und Status sowie „Verlauf leeren“ anbieten.
- Entscheidungspunkt: nur aktueller Dienstlauf oder optionale lokale Persistenz.
- Bei Persistenz standardmäßig kurze Aufbewahrung, explizites Löschen und keine vollständigen Pfade speichern.

Phasen-Gate 3:

- Ein Nutzer kann einen mehrteiligen Batch vollständig steuern und seinen Zustand ohne Diagnosewissen verstehen.
- Pause/Abbruch/Retry bleiben mit Netzwerk- und Sitzungsverlust konsistent.

## 9. Phase 4 – Netzwerkvertrauen und Geräteverwaltung

### R4.1 Vertraute Netzwerke verwalten

- Aus `Vec<String>` ein migrierbares Modell mit stabiler ID, Anzeigename, Kategorie und letzter Verwendung entwickeln.
- Vertrauensliste in der Desktop-App anzeigen.
- Einzelnes Netzwerk oder alle Netzwerke vergessen können.
- Änderungen nur bei gestopptem Dienst oder mit klar definierter Wirkung auf den aktuellen Dienst zulassen.
- Nicht mehr auflösbare Profile sichtbar als veraltet kennzeichnen.

### R4.2 Verständliche Geräteidentität

- User-Agent lokal in verständliche Browser-/Gerätebezeichnungen umwandeln.
- Optionalen sitzungsbezogenen Gerätenamen beim Login erlauben.
- IP, Erstellungszeit, letzte Aktivität und aktive Transfers nachvollziehbar darstellen.
- Namen als nicht vertrauenswürdige Eingabe behandeln und bidi-isoliert anzeigen.

### R4.3 Optionale strengere Kopplung

Design-Spike vor Implementierung:

- Einmal-Code beziehungsweise automatische Code-Rotation nach der ersten erfolgreichen Kopplung bewerten.
- Optional pro Sitzung nur Upload, nur Download oder beide Rollen freigeben.
- Bedienbarkeit für mehrere legitime Geräte gegen zusätzlichen Sicherheitsgewinn abwägen.
- Threat Model und Rate-Limit-Tests vor der Umsetzung aktualisieren.

## 10. Phase 5 – Profile und mehrere Freigaben

### R5.1 Freigabeprofile zuerst

- Mehrere gespeicherte Profile mit jeweils einem Download- und einem Uploadordner ermöglichen.
- Jeweils nur ein Profil pro Dienstlauf aktivieren; damit bleiben API und Sicherheitsgrenzen zunächst unverändert.
- Profile duplizieren, umbenennen und löschen können.
- Pro Profil Netzwerk, Port und Limits entweder erben oder explizit überschreiben.

### R5.2 Mehrere gleichzeitig benannte Freigaben nur bei belegtem Bedarf

- API v2 mit stabilen, nicht pfadbasierten Share-IDs entwerfen.
- Mobile Auswahlseite für benannte Downloadfreigaben und Uploadziele erstellen.
- Jede Freigabe separat verankern, limitieren und autorisieren.
- Überlappungen über alle aktiven Roots hinweg verbieten.
- Freigabenamen dürfen keine Dateisystempfade oder sensible Struktur verraten.

Phasen-Gate 5:

- Profile beschädigen keine bestehenden v1-Einstellungen und können vollständig migriert werden.
- Jede gleichzeitig aktive Root besitzt eigene Tests für Traversal, Austausch, Rolle und Überlappung.

## 11. Phase 6 – Strategische Transferfunktionen

### R6.1 Wiederaufnahme nach Seitenreload oder erneuter Auswahl

Diese Funktion darf nicht nur Dateiname, Größe und Änderungszeit vergleichen.

Vorgehen:

1. Protokollentwurf für eine persistierbare, zufällige Recovery-ID erstellen.
2. Bereits übertragene Inhalte durch kryptografische Chunk-Hashes oder eine gleichwertige Inhaltsprüfung an die erneut ausgewählte Datei binden.
3. Persistente Servermanifeste authentisieren und gegen Pfadmanipulation absichern.
4. Absturzreste nur dann wiederverwenden oder löschen, wenn ihre DMDC-Eigentümerschaft eindeutig nachweisbar ist.
5. Wiederaufnahme nach Reload, Sitzungswechsel und optional Dienstneustart getrennt spezifizieren.
6. Manipulations-, Kollisions-, Stromausfall- und Versionsmigrationstests ergänzen.

Abnahme:

- Eine andere Datei mit identischen Metadaten kann niemals eine Teilübertragung übernehmen.
- Ein verlorener Abschluss publiziert weiterhin höchstens eine Zieldatei.
- Nicht zuordenbare Crashreste bleiben erhalten und werden nicht automatisch gelöscht.

### R6.2 Sammel- und Ordnertransfers

Design-Spike mit drei Varianten:

- Flache Mehrfachauswahl verbessern.
- Ordnerauswahl mit sicher normalisierten relativen Pfaden.
- Mehrfachdownload als begrenztes, gestreamtes ZIP.

Vor Umsetzung festlegen:

- Maximale Dateien, Gesamtsumme, Pfadtiefe und Archivgröße.
- Verhalten bei Teilfehlern und Namenskollisionen.
- Ob Uploads flach bleiben oder ein neuer serverseitiger Batchordner angelegt wird.
- CPU-, Speicher-, Dateihandle- und Laufzeitlimits für Archive.

### R6.3 Optionale Transportverschlüsselung als v2-Spike

- Nur als eigener Architektur- und UX-Spike behandeln.
- Zertifikatsbereitstellung, Gerätevertrauen, QR-/Code-Fluss und Browserkompatibilität gemeinsam lösen.
- Kein „optionales HTTPS“ veröffentlichen, das regelmäßig Zertifikatswarnungen erzeugt oder Nutzer zum Wegklicken erzieht.
- Erst nach aktualisiertem Threat Model und realer Geräteabnahme implementieren.

## 12. Phase 7 – Release-Härtung

### R7.1 Automatisierte Releasepipeline

- Reproduzierbaren Windows-Releasebuild und NSIS-Artefakt erzeugen.
- Versionsnummern in Cargo, npm-Workspaces, Tauri und Changelog konsistent halten.
- Prüfsummen und Buildprotokoll veröffentlichen.
- Dependency-Audits für npm und Rust in einer freigegebenen Netzwerkumgebung ausführen und Befunde triagieren.

### R7.2 Signierung und Updates

- Code-Signing für Anwendung und Installer einführen, bevor eine breite öffentliche Verteilung beginnt.
- Erst danach einen signaturgeprüften Updatekanal entwerfen.
- Update während aktivem Dienst beziehungsweise laufender Übertragung ausdrücklich blockieren.
- Rollback- und Konfigurationsmigrationsstrategie dokumentieren.

### R7.3 End-to-End- und reale Abnahme

- Browser-E2E für Login, Navigation, Suche, Uploadqueue und Sessionverlust ergänzen.
- Windows 10 und 11: Installation, UAC-Abbruch, Firewallregel, Update und Deinstallation mit Sentinel-Nutzdaten testen.
- Aktuelles iOS/Safari und Android/Chrome im Offline-WLAN prüfen.
- Große Datei, viele kleine Dateien, Range-Download, langsamer Datenträger, voller Datenträger und Netzwerkwechsel testen.
- Bedienbarkeit mit Tastatur, Screenreader-Basics, Reduced Motion, 200-%-Skalierung und schmalen Displays prüfen.

### R7.4 Veröffentlichungsgrundlagen

- Lizenz- und Drittanbieterhinweise klären.
- Changelog, Supportweg, Sicherheitskontakt und bekannte Grenzen veröffentlichen.
- Datenschutzbeschreibung für lokale Logs, Verlauf und optionale Gerätenamen ergänzen.

## 13. Priorisierte Ausführungsreihenfolge

Die konkrete Abarbeitung erfolgt in dieser Reihenfolge:

1. R0.1 Git-Baseline.
2. R0.2 Dokumentations- und QA-Korrektur.
3. R0.3 CI-Grundlage.
4. R1.1 Mobile Queue und Fehlerbehandlung.
5. R1.2 blockierendes Upload-I/O.
6. R1.3 Einstellungs-Migration.
7. R1.4 Desktop-Validierung und Dirty-State.
8. R2.1/R2.2 generierte Verträge und strukturierte Fehler.
9. R2.3 Modulzerlegung.
10. R2.4 stabiler Statuspfad.
11. R3 Transferkomfort.
12. R4 Vertrauens- und Geräteverwaltung.
13. R5 Profile; gleichzeitige Mehrfachfreigaben nur nach Nutzungsentscheidung.
14. R6 Wiederaufnahme und Sammeltransfer jeweils nach eigenem Design-Gate.
15. R7 vollständige Release-Härtung.

R3 und R4 können nach Abschluss von Phase 2 teilweise parallel geplant werden, sollen aber in getrennten Commits und Abnahmezyklen bleiben.

## 14. Entscheidungspunkte vor größeren Investitionen

Vor der jeweiligen Phase ist eine bewusste Produktentscheidung nötig:

1. Soll der Transferverlauf einen Neustart überleben oder bewusst flüchtig bleiben?
2. Genügen schnell wechselbare Profile oder werden mehrere gleichzeitig sichtbare Freigaben benötigt?
3. Muss Wiederaufnahme nur einen Seitenreload, auch eine neue Sitzung oder sogar einen Dienstneustart überstehen?
4. Werden Ordnerstrukturen übertragen oder genügt ein begrenztes ZIP-Verfahren?
5. Bleibt DMDC ein privates Windows-Werkzeug oder ist eine öffentliche, signierte Verteilung geplant?
6. Ist Transportverschlüsselung ein tatsächliches v2-Ziel oder bleibt das bestätigte LAN die dauerhafte Produktgrenze?

Diese Entscheidungen ändern nicht die frühen Phasen 0 bis 2. Sie bestimmen erst Umfang und Reihenfolge der Phasen 3 bis 7.
