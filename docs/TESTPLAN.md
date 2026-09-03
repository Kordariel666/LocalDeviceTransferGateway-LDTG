# Abnahmeplan für DMDC v1

## Automatisiert

- `pnpm check`: vollständige lokale Prüfkette aus generiertem Vertragsvergleich, Typprüfung, ESLint, Frontendtests mit Coverage, Rust-Tests, Formatierung, Clippy und beiden Produktions-Webbuilds; derselbe Befehl läuft in der Windows-CI.
- `pnpm contracts:generate`: TypeScript-Verträge aus den serialisierbaren Rust-DTOs aktualisieren.
- `pnpm contracts:check`: read-only nachweisen, dass der eingecheckte Export exakt dem Rust-Modell entspricht; veraltete Verträge führen zu einem Fehlercode.
- `pnpm typecheck`: beide React-Anwendungen und gemeinsame Verträge.
- `pnpm test`: Frontendtests ohne reale Benutzerordner.
- `pnpm test:coverage`: Frontendtests mit V8-Coverageberichten je Anwendung.
- `pnpm test:rust`: Pfad-, Subnetz-, Zustands-, Sitzungs-, Upload- und Downloadtests mit temporären Ordnern; der Wrapper ergänzt nur für den Windows-Test-Runner das von Tauri benötigte Common-Controls-v6-Manifest.
- `pnpm build:web`: reproduzierbare Produktionsassets für Desktop und Handy.
- `pnpm build`: Windows-Binärdatei und per-user NSIS-Installer.

Profilregressionen prüfen die verlustfreie Schema-4-Migration der bisherigen
Einzel-Freigaben in ein Standardprofil, eindeutige Profilidentitäten und -namen,
geerbte und explizit überschriebene Netzwerk-/Port-/Limitwerte sowie
Duplizieren, Umbenennen und Löschen.

Gezielte Regressionen decken zusätzlich ab: Authentisierung einschließlich begrenzter sitzungsbezogener Gerätenamen und lokaler User-Agent-Klassifizierung; unveränderte IP-bezogene Codeversuchs- und Sitzungslimits bei wechselnden Gerätenamen; manuelle Coderotation mit zurückgesetzter Drosselung, aber ohne stillen Sitzungswiderruf; Existenz, Besitz und ein exklusives Upload-ID-Permit vor PATCH-Bodyverbrauch; globale Uploadblock-, Verbindungs-, Request-, Auth-, Download-, Dateisystem-, Cursor-, Uploadanzahl-, Inbox-Objekt-/Byte- und Sitzungslimits; globale und IP-bezogene Fairness des Upload-I/O-Pools; IP-/Sitzungsfairness aktiver und persistenter Ordnerarbeit; monotone Idle-/Absolutfristen von Sitzungen sowie absolute Header-, Verbindungs-, Download- und Uploadfristen; atomare Rückgewinnung abgelaufener Sitzungskapazität; Konkurrenz zwischen Einzel-/Gesamtwiderruf und neuer Sitzung beziehungsweise Cursoreinfügung; cancellation-sichere Blocking-Permits, dienstbesessene Upload-Chunks und Upload-Commits; reaktionsfähige Dienstbereinigung bei blockierter Upload-I/O; konsistente Offsets, Bytebudgets und Transferzustände nach Schreibfehlern; Wiederholung verlorener Zwischen- und Finalseiten; Bindung der mobilen Folgeseite an den angewendeten statt bearbeiteten Filter; serialisierte Desktop-Lifecycle-Übergänge; reaktionsfähiger Stop bei blockierter Umgebungsprüfung, Shutdown-Priorität gegenüber gleichzeitigem Fehlergebnis und Beendigung eines stockenden PowerShell-Hilfsprozesses; TTL/Kapazität der Fehlversuchs-Map, unveränderter Code und die Sperre korrekter wie falscher Codes während der globalen Abkühlung; vollständige Netzwerkprofilidentität; Windows-Autostartpfade; kanonisch überlappende Freigaben; begrenzte serverseitige Ordnerarbeit; reducer-basierte Queue-Invarianten und alle drei abbrechbaren Backoff-Stufen; atomare Batchaktionen für Pause, Fortsetzung und Retry in stabiler Reihenfolge; Einzelentfernung ausschließlich wartender Queue-Einträge und Sammelbereinigung terminaler Einträge; größenbewerteten Summenfortschritt sowie eine nur bei betroffener Queue sichtbare Erklärung des Sitzungsverlusts; unmittelbaren Start der nächsten Datei nach Pause oder Abbruch; Pause/Resume in der Queue und während der Erstellung, jedoch nur vor Beginn der Finalisierung; Übernahme beziehungsweise nachträgliches Löschen einer während Pause/Abbruch oder Entfernung erstellten Server-ID sowie sofortiges Resume vor der Create-Antwort; Erhalt strukturierter PATCH-Fehlercodes und sichtbaren Retry von `failed` nach `uploading`; Schutz gegen doppeltes Login und lokalen Logout bei nicht erreichbarem Dienst; weitere Dateiauswahl während eines Uploads; getrennte Dateiauswahl-IDs beim Resume; veraltete Navigationsantworten; Serve-/Join-Fehler; UTF-16-Namen, immer opake Uploadzielnamen und atomare No-Replace-Veröffentlichung; fehlende, schrittweise migrierte, zukünftige, semantisch ungültige, typbeschädigte und unlesbare Einstellungen einschließlich Recovery-Backup; vom Konfigurationsschema unabhängige Buildversionsanzeige; Dirty-State und Speichersperre ohne tatsächliche Änderung; feldbezogene Port-, Größen-, Datei- und Freigabefehler; kanonische Backend-Prüfung vor dem Start; Entwurfserhalt über Seitenwechsel und Hintergrundereignisse; Warnung vor Verwerfen über Browser-, Fenster- und Tray-Quit-Pfade; stabile JSON-Codes und diskriminierte Kontexte aller Tauri-Steuerdialoge einschließlich `|` im Freigabepfad; Abschirmung roher interner Fehlerdetails sowie das Verbot rekursiver AppData-Löschung im Uninstaller.

Zeit- und ETA-Regressionen prüfen zusätzlich monotone Backendmessung,
exponentielle Glättung, Neustart bei rückläufigem Offset, mobile
Pause-/Retry-Baselines, exakte Bytefortschritte, gemeinsame Dauer-/Ratenformate
und die Umschaltung zwischen unbekannter, instabiler, stabiler und veralteter
Restzeitschätzung. Die dabei verwendeten Modelle erhalten keine Pfade oder
Dateiinhalte.

## Manuell vor Freigabe

- Installation und Deinstallation auf Windows 10 und 11, jeweils mit angenommener und abgebrochener UAC-Abfrage.
- Nach der Deinstallation prüfen, dass Konfiguration, Logs und Sentinel-Nutzdaten unter beiden DMDC-AppData-Bäumen erhalten bleiben, während die Firewallregel entfernt ist.
- Regelprüfung nach Portänderung; Nutzung in einem von Windows als „Öffentlich“ markierten, aber in DMDC bestätigten Netz.
- Offline-WLAN ohne Internet mit Safari auf einem aktuellen iPhone und Chrome auf einem aktuellen Android-Gerät.
- Zwei parallele Handysitzungen sowie gezieltes und vollständiges Trennen.
- Unicode-/Nicht-BMP-Namen, Namenskollisionen, viele kleine Dateien, eine große Datei nahe dem Limit, Pause, Verbindungsabbruch und Fortsetzung innerhalb derselben offenen Dateiauswahl. Nach Seitenreload muss die Datei bewusst neu gewählt und als neuer Upload begonnen werden.
- Range-Fortsetzung eines großen Downloads.
- Fenster schließen, Tray öffnen, Dienst stoppen und Beenden bei aktiver Übertragung.
- Negativtests für Traversal, doppelte URL-Kodierung, ADS, Symlink/Reparse-Point, versteckte Dateien, belegten Port, vollen Datenträger und Netzwerkwechsel.
- Nachweis, dass gleiche/verschachtelte Freigabewurzeln und Windows-Startup/Common-Startup als Uploadziel abgewiesen werden, der Upload-Eingang nicht aufgelistet und keine Datei über das Handy gelöscht, überschrieben, umbenannt oder verschoben werden kann.
