# DMDC

**Desktop Mobile Data Center** ist eine lokale Desktopanwendung für kontrollierte Dateiübertragungen zwischen einem PC und Mobilgeräten im selben Netzwerk. Die Desktop-App startet den Dienst und legt die zwei möglichen Freigaben fest; Uploads und Downloads werden ausschließlich im Browser des Handys ausgelöst.

## Sicherheitsmodell

DMDC v1 verwendet bewusst gehärtetes **HTTP im vertrauenswürdigen LAN**. Es gibt keine Cloud, kein Konto, keine öffentliche Webseite, keine Portweiterleitung und keine externen Web-Ressourcen. HTTP ist jedoch keine Ende-zu-Ende-Verschlüsselung: Andere Teilnehmer oder Administratoren des lokalen Netzes könnten Verkehr grundsätzlich mitlesen oder manipulieren. DMDC darf deshalb nur in einem bewusst bestätigten Netzwerk eingesetzt werden.

- Der achtstellige Code steht nie in URL oder QR-Code. Verteilte Fehlversuche besitzen zusätzlich einen dienstweiten Grenzwert. Eine aktive Abkühlphase wird vor dem Codevergleich geprüft und rotiert den Code nicht, damit fremde Geräte weder einen Codewechsel erzwingen noch den gültigen Code als Prüf-Orakel verwenden können.
- Sitzungen sind an Dienstinstanz und Client-IP gebunden, laufen nach 6 Stunden 15 Minuten Inaktivität beziehungsweise nach 24 Stunden absolut ab und enden spätestens beim Dienststopp.
- Die Downloadfreigabe ist ausschließlich lesbar.
- Der Upload-Eingang erlaubt nur neue Dateien und zeigt seinen vorhandenen Inhalt nicht an. Das Backend weist gleiche oder verschachtelte Download-/Uploadwurzeln ab, damit diese Zusage auch bei abweichenden Pfadschreibweisen gilt.
- Gleichzeitig sind höchstens 12 Downloads insgesamt, 4 pro Client-IP und 3 pro Handysitzung aktiv; pro Client-IP sind höchstens 4 unvollständige Uploads reserviert. Zusätzlich begrenzen standardmäßig 100 GiB und 10.000 Dateien den gesamten Upload-Eingang einschließlich bereits abgeschlossener Dateien. Diese beiden Werte sind in den Sicherheitseinstellungen anpassbar.
- Verbindungen, gleichzeitig bearbeitete HTTP-Anfragen, Dateisystemprüfungen, Ordnerlistings und Sitzungen besitzen globale, klassenspezifische, Geräte-/IP- und soweit passend sitzungsbezogene Kapazitätsgrenzen sowie Zeitlimits. Windows gruppiert mehrere IP-Aliase nach Möglichkeit anhand der lokalen Nachbartabelle zu einem physischen Peer; fehlt diese Information, gilt die IP als konservativer Ersatzschlüssel. Nicht angemeldete Verbindungen haben eine eigene kleine Kapazität und enden nach spätestens 30 Sekunden; angemeldete Verbindungen können diese Klasse nicht verdrängen. Neue Sitzungen verdrängen niemals frische Geräte oder deren Übertragungen.
- Unvollständige Uploads laufen nach 30 Minuten ohne übertragenen Block oder spätestens nach 24 Stunden ab. Live-Uploads werden ausschließlich über bereits geöffnete, stabile Dateihandles verwaltet und gelöscht. Nach einem Absturz bleiben nicht mehr zweifelsfrei zuordenbare `.part`-Dateien zur manuellen Prüfung erhalten.
- Pro Upload-ID wird höchstens ein Datenblock gleichzeitig angenommen; insgesamt werden höchstens 8 Uploadblöcke vor dem Body-Puffern zugelassen. Uploadanlage, Inbox-Prüfung, Schreiben, Synchronisation und Abschluss laufen zusätzlich in einem fairen Blocking-Pool mit 4 globalen und 2 Slots pro Client-IP. Jeder nicht abschließende Block ist exakt 8 MiB groß, damit ein Client keine beliebige Zahl winziger dauerhafter Schreib- und Fortschrittsvorgänge auslösen kann. Ein neuer Offset wird erst nach erfolgreichem `sync_data` bestätigt; verliert der Client die Antwort, schließt der Dienst die konsistente Buchhaltung trotzdem ab.
- Mobile Uploads verwenden eine kryptografisch zufällige 128-Bit-Wiederaufnahme-ID. Eine begrenzte, ablaufende Abschlussquittung verhindert auch nach verlorener Antwort, neuer Anmeldung oder IP-Wechsel eine zweite Veröffentlichung derselben Datei.
- Es existiert keine LAN-API zum Starten, Stoppen oder Umkonfigurieren des Dienstes.
- Löschen, Überschreiben, Umbenennen, Verschieben und Ausführen von Dateien sind nicht implementiert.
- Eingehende Dateien erhalten immer einen unvorhersagbaren serverseitigen Namenszusatz. Die atomare No-Replace-Übernahme bleibt erhalten, ohne durch den Antwortnamen die Existenz gleichnamiger Inbox-Dateien offenzulegen.

Weitere Details stehen in [SECURITY.md](SECURITY.md) und [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## Entwicklung

Voraussetzungen unter Windows 10/11:

- Node.js LTS und pnpm
- Rust stable mit MSVC-Ziel
- Visual Studio 2022 Build Tools mit „Desktopentwicklung mit C++“
- WebView2 Runtime

```powershell
pnpm install
pnpm check
pnpm dev
```

`pnpm check` prüft zuerst, dass die aus Rust erzeugten TypeScript-Verträge aktuell
sind, und führt danach Typprüfung, ESLint mit React-Hooks-Regeln, Frontendtests samt
Coverage-Bericht, Rust-Tests, Formatierung, Clippy und beide Webbuilds aus. Die
Einzelbefehle wie `pnpm test`, `pnpm test:coverage`, `pnpm test:rust` und
`pnpm build:web` bleiben für gezielte lokale Prüfungen verfügbar. Dasselbe
Qualitätsgate läuft auf GitHub Actions unter Windows; HTML-Coverage-Berichte werden
dort 14 Tage als Buildartefakt aufbewahrt.

Rust-DTOs unter `src-tauri/src/domain` sind die maßgebliche Quelle der gemeinsam
genutzten Datenverträge. Nach einer DTO-Änderung aktualisiert
`pnpm contracts:generate` das Paket `@dmdc/shared`; `pnpm contracts:check` meldet
vergessene Exporte, ohne Dateien zu verändern.

Fallible Tauri-Aufrufe lehnen Promises mit einem typisierten `CommandError` ab.
Stabile Codes und diskriminierte Kontextobjekte steuern Bestätigungsdialoge;
interne System- oder Dateifehler werden weder als Delimiter-Strings ausgewertet
noch ungefiltert in der Oberfläche angezeigt.

Der NSIS-Installer wird mit `pnpm build` erzeugt. Code-Signing, Auto-Updates und öffentliche Veröffentlichung sind nicht Bestandteil von v1.

Der Uninstaller entfernt die Firewallregel, bewahrt aber Konfiguration, Logs und mögliche Nutzdaten in den DMDC-AppData-Verzeichnissen. Er löscht diese Verzeichnisse nicht rekursiv.

Die Konfiguration besitzt ein eigenes versioniertes Schema. Ältere
`settings.json`-Dateien werden beim Laden schrittweise migriert; zukünftige,
beschädigte oder semantisch ungültige Dateien bleiben unverändert und führen zu
sicheren Standardwerten mit sichtbarer Warnung. Vor einem bewussten Ersetzen legt
DMDC eine nummerierte Recovery-Kopie an. Die App-Buildversion wird nicht als
Benutzereinstellung gespeichert.

Die Desktopoberfläche markiert ungespeicherte Änderungen, ordnet
Validierungsfehler direkt den betroffenen Feldern zu und prüft Freigabeordner
kanonisch im Backend, bevor Konfiguration oder Dienststart fortfahren. Entwürfe
bleiben beim Seitenwechsel erhalten; Fensterschließen und Tray-Beenden verlangen
vor dem Verwerfen eine ausdrückliche Bestätigung.

`pnpm test:rust` bettet ausschließlich in den Windows-Test-Runner das Common-Controls-v6-Manifest ein, das Tauri beim normalen App-Build ohnehin erhält. Dadurch laufen die Rust-Unit- und Integrationstests ohne den Windows-Ladefehler `TaskDialogIndirect`; Produktions- und Installer-Manifeste werden nicht verändert.

Auf Rechnern mit einer strikten Windows-Anwendungssteuerungsrichtlinie müssen lokal von Cargo erzeugte Build-Helfer für Entwicklungsbuilds zugelassen sein. Diese Einschränkung betrifft nur die Entwicklung, nicht die Architektur von DMDC.

## Projektstruktur

```text
apps/desktop       Tauri-Desktopoberfläche (React/Vite)
apps/mobile        eingebettete responsive Handyoberfläche (React/Vite)
packages/shared    aus Rust generierte TypeScript-Verträge
src-tauri/domain   Einstellungen, Netzwerk- und Dateisystemgrenzen
src-tauri/service  Axum-Server, Sitzungen und Übertragungsprotokoll
src-tauri/platform plattformspezifische Firewallintegration
```

Die beiden Weboberflächen sind getrennte Builds. Dateiinhalte passieren niemals Tauri-IPC, sondern werden vom Rust-Server direkt gestreamt.

## Dokumentation und Projektstatus

- [Entwicklungsroadmap](docs/ROADMAP.md)
- [Changelog](CHANGELOG.md)
- [Aktuelle HTTP-API](docs/API.md)
- [Aktuelle Architektur](docs/ARCHITECTURE.md)
- [Sicherheitsrichtlinie](SECURITY.md)
- [Abnahmeplan](docs/TESTPLAN.md)
- [Geprüfter Git-Ausgangsstand](docs/BASELINE_2026-09-02.md)
- [Abhängigkeitsstrategie](docs/DEPENDENCIES.md)
- [Letzter Sicherheits- und Fehlerbehebungsbericht](qa/security-fix-report-2026-09-02.md)

## Bedienung

1. Downloadordner und/oder Upload-Eingang wählen.
2. Netzwerk, Port und Grenzen prüfen und die Firewallregel einmalig einrichten.
3. Nur in einem vertrauten Netz den Dienst starten.
4. Die angezeigte URL oder den QR-Code am Handy öffnen und den separat angezeigten Code eingeben.
5. Nach der Übertragung den Dienst manuell stoppen. Solange er läuft, minimiert das Schließen des Fensters DMDC in den System-Tray.

DMDC führt empfangene Dateien nicht aus und enthält keinen Virenscanner. Empfangene Dateien sollten wie jeder andere externe Inhalt behandelt werden.
Windows-Autostartverzeichnisse dürfen nicht als Upload-Eingang verwendet werden. Download- und Uploadwurzel müssen vollständig getrennt sein.
DMDC akzeptiert Upload-Eingänge nur auf lokalen festen, entfernbaren oder RAM-Laufwerken. Effektive Windows-Startordner, bekannte Office-Autoload-Verzeichnisse und nachträglich umgebogene beziehungsweise ausgetauschte Freigabewurzeln werden abgewiesen. Nicht auflösbare Windows-Netzwerkprofile gelten als nicht vertrauenswürdig und benötigen nach jeder Identitätsänderung eine neue Bestätigung.
