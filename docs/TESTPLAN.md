# Abnahmeplan für DMDC v1

## Automatisiert

- `pnpm typecheck`: beide React-Anwendungen und gemeinsame Verträge.
- `pnpm test`: Frontendtests ohne reale Benutzerordner.
- `pnpm test:rust`: Pfad-, Subnetz-, Zustands-, Sitzungs-, Upload- und Downloadtests mit temporären Ordnern; der Wrapper ergänzt nur für den Windows-Test-Runner das von Tauri benötigte Common-Controls-v6-Manifest.
- `pnpm build:web`: reproduzierbare Produktionsassets für Desktop und Handy.
- `pnpm build`: Windows-Binärdatei und per-user NSIS-Installer.

Gezielte Regressionen decken zusätzlich ab: Authentisierung, Existenz, Besitz und ein exklusives Upload-ID-Permit vor PATCH-Bodyverbrauch; globale Uploadblock-, Verbindungs-, Request-, Auth-, Download-, Dateisystem-, Cursor-, Uploadreservierungs- und Sitzungslimits; IP-/Sitzungsfairness aktiver und persistenter Ordnerarbeit; monotone Idle-/Absolutfristen von Sitzungen sowie absolute Header-, Verbindungs-, Download- und Uploadfristen; atomare Rückgewinnung abgelaufener Sitzungskapazität; Konkurrenz zwischen Einzel-/Gesamtwiderruf und neuer Sitzung beziehungsweise Cursoreinfügung; cancellation-sichere Blocking-Permits und dienstbesessene Upload-Commits; Wiederholung verlorener Zwischen- und Finalseiten; Bindung der mobilen Folgeseite an den angewendeten statt bearbeiteten Filter; serialisierte Desktop-Lifecycle-Übergänge; reaktionsfähiger Stop bei blockierter Netzwerkprüfung, Shutdown-Priorität gegenüber gleichzeitigem Fehlergebnis und Beendigung eines stockenden PowerShell-Hilfsprozesses; TTL/Kapazität der Fehlversuchs-Map; korrekter neuer Code während globaler Wrong-Attempt-Abkühlung; vollständige Netzwerkprofilidentität; Windows-Autostartpfade; kanonisch überlappende Freigaben; begrenzte serverseitige Ordnerarbeit; Pause/Resume während Queue, Erstellung, Retry-Backoff und vor Finalisierung; Übernahme beziehungsweise nachträgliches Löschen einer während Pause/Abbruch erstellten Server-ID sowie sofortiges Resume vor der Create-Antwort; weitere Dateiauswahl während eines Uploads; getrennte Dateiauswahl-IDs beim Resume; veraltete Navigationsantworten; Serve-/Join-Fehler; UTF-16-Namen, immer opake Uploadzielnamen und atomare No-Replace-Veröffentlichung; fehlende, beschädigte und unlesbare Einstellungen sowie das Verbot rekursiver AppData-Löschung im Uninstaller.

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
