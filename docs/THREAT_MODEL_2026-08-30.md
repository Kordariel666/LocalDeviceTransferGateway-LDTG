# Bedrohungsmodell zum Audit vom 30. August 2026

**Status:** Quellgestützt erfasst; keine Behebung durchgeführt

> **Historischer Ausgangsstand:** Dieses Bedrohungsmodell beschreibt den Auditzeitpunkt vor den beiden Behebungsrunden. Die aktuellen Kontrollen und die Verifikation der ursprünglichen SEC-Befunde stehen in `REMEDIATION_REPORT_2026-08-30.md`; die anschließenden Re-Audit-Fixes und ihre Verifikation stehen in `RE_AUDIT_REMEDIATION_2026-08-30.md`.

## Systemzusammenfassung

DMDC v1 ist eine Windows-10/11-Tauri-Anwendung. Ein vertrauenswürdiger lokaler Operator wählt Download- und/oder Uploadordner, private IPv4-Schnittstelle und Port, bestätigt ein Netzwerk und startet einen Axum-HTTP-Dienst für Mobilbrowser im LAN. Die lokale React-WebView steuert Einstellungen, Dienstlebenszyklus, Sitzungen, Firewall und Diagnose ausschließlich über registrierte Tauri-Befehle; diese Desktopfunktionen sind nicht Teil des LAN-Routers (`src-tauri/src/lib.rs:205-438`, `554-566`, `src-tauri/src/service/api.rs:81-104`, `docs/API.md:5-18`).

Beim Start werden Einstellungen geprüft, die Schnittstelle aufgelöst, Freigabepfade kanonisiert und der Listener direkt an `<ausgewählte IPv4>:<Port>` gebunden (`src-tauri/src/lib.rs:243-301`, `src-tauri/src/service/mod.rs:44-59`). Der Mobile-Build ist im Rust-Binary eingebettet. Nach der Anmeldung überträgt Rust Dateien direkt zwischen HTTP und Dateisystem; Dateiinhalte laufen nicht über Tauri-IPC (`src-tauri/src/service/api.rs:38-40`, `1434-1463`, `README.md:55`).

## Schutzgüter

- Dateien und Metadaten unter der kanonischen Downloadwurzel; LAN-Zugriff muss dort lesend und enthalten bleiben.
- Bestehende Inhalte und Integrität des Upload-Eingangs; Clients dürfen ihn nicht auflisten und abgeschlossene Uploads dürfen nichts ersetzen.
- Unvollständige Uploaddaten unter `<Uploadwurzel>/.dmdc/<UUID>.part` sowie Uploadzahl, Speicherreservierung und 1-GiB-Reserve.
- Flüchtiger sechsstelliger Zugangscode, Sitzungs- und CSRF-Tokens, Service-ID, Upload-Eigentümer und IP-Bindungen.
- Persistierte Freigabepfade, Adapter, Port, Upload-/Idle-Limits und bestätigte Netzwerkidentitäten.
- Lokale Desktopbefugnisse: Start/Stop, Rotation, Widerruf, Einstellungen, Diagnose, Beenden und erhöhte Firewallkonfiguration.
- Windows-Firewallregel `DMDC Local Transfer` mit Programm, TCP-Port, `LocalSubnet`, allen Profilen und gesperrtem Edge Traversal.
- Vertraulichkeit der Diagnose und Logs.
- Verfügbarkeit von Dienst und Host.

## Vertrauensgrenzen

1. **Lokaler Operator → Desktop-WebView → Tauri-Backend.** Der Hauptfensterkontext besitzt Core-/Dialogrechte und registrierte DMDC-Befehle, aber keine generische Shell- oder Dateisystemfähigkeit (`src-tauri/capabilities/default.json:3-10`).
2. **Desktop-Backend → Windows-Admin/UAC.** Die Firewallroutine startet kanonisches System32-PowerShell über `runas`, ersetzt die benannte Regel und liest das Ergebnis zurück (`src-tauri/src/platform/mod.rs:32-67`, `108-157`, `170-262`).
3. **LAN-Client → HTTP-Listener.** Statische Assets sind vor Anmeldung erreichbar; jede Anfrage benötigt passendes Quellsubnetz und exakten Host, Schreibmethoden zusätzlich exakten Origin. Es existieren nur Datei-/Sitzungs-APIs (`src-tauri/src/service/api.rs:81-194`).
4. **Nicht angemeldet → angemeldete Sitzung.** `/auth` besitzt ein enges JSON-Limit, konstanten Codevergleich und IP-bezogene Fehlversuchsbegrenzung. Erfolg erstellt zufällige Tokens und ein HttpOnly-/SameSite-Strict-Cookie (`src-tauri/src/service/api.rs:304-395`).
5. **Sitzung → autorisierte Operation.** Cookie-Token und Client-IP müssen zur Sitzung passen; Schreiboperationen benötigen CSRF; Upload-IDs gehören einer konkreten Sitzung (`src-tauri/src/service/api.rs:233-282`, `1114-1143`).
6. **HTTP-Dienst → Downloaddateisystem.** Kanonische Wurzel, Komponentenprüfung, ADS-/Symlink-/Reparse-/Hidden-Prüfung und kanonisches Enthaltensein schützen die Leseroute (`src-tauri/src/domain/shares.rs:128-175`).
7. **HTTP-Dienst → Uploaddateisystem.** Server-UUIDs, Create-new, Eigentümer, exakter Offset, Größenprüfung, Synchronisierung und No-Replace-Veröffentlichung schützen die Schreibrouten (`src-tauri/src/domain/shares.rs:208-318`, `src-tauri/src/service/api.rs:984-1401`).
8. **Mobile Origin → Browserzustand.** Cookie, React-CSRF und `localStorage`-Resume-Mapping liegen im Browser. Eine Upload-ID allein genügt ohne Sitzung, gleiche IP, Eigentümer und CSRF nicht.
9. **Installer/Uninstaller → Hostzustand.** NSIS installiert current-user; die Deinstallation entfernt erhöht die Firewallregel und löscht nur die app-spezifischen Roaming-/Local-AppData-Verzeichnisse (`src-tauri/windows/hooks.nsh:1-7`).

## Angreiferfähigkeiten

- Der primäre In-Scope-Angreifer ist ein nativer LAN-Client im gewählten Schnittstellensubnetz. Er kann HTTP-Anfragen/Header, Dateinamen, Uploadbytes, Downloadpfade/-ranges und Codeversuche kontrollieren.
- Anfangs fehlen ihm Sitzung, CSRF, Uploadeigentum, lokale Tauri-/Dateisystemrechte und Windows-Adminrechte.
- Nach gültiger Anmeldung erhält er nur die aktivierten Rollen. Es existiert kein HTTP-Weg für Einstellungen, Dienststeuerung, Coderotation, Sitzungswiderruf, Diagnose oder Firewall.
- Pro Sitzung sind höchstens drei Downloads, global zwölf Downloads, pro IP vier und global 64 unvollständige Uploads erlaubt; ein Block ist höchstens 8 MiB groß.
- Upload-UUIDs verleihen ohne IP-gebundene Sitzung, Eigentum und CSRF keine zusätzliche Fähigkeit.
- Passive/aktive LAN-MITM, Internetexposition, Portweiterleitung, UPnP und Betrieb auf einem untrusted Netzwerk sind laut `SECURITY.md:16-18` außerhalb der v1-Zusage.
- Der lokale Operator und sein Prozesskonto sind vertrauenswürdige Konfigurationsautoritäten.

## Sicherheitsziele

- Kein LAN-Client darf außerhalb konfigurierte Freigaben lesen oder schreiben, destruktive/Move-/Rename-/Execute-Operationen ausführen, den Upload-Eingang auflisten, Desktopsteuerung über HTTP erreichen, Auth-/Sitzungs-/CSRF-/Host-/Origin-/Subnetzkontrollen umgehen oder Geheimnisse/Dateiinhalte aus Diagnosen erhalten (`SECURITY.md:7-14`).
- Der Listener darf nur an die ausgewählte private IPv4 binden und muss die passende Netzwerkgeometrie, Host und Origin durchsetzen.
- Der Zugangscode darf weder URL noch QR-Code enthalten.
- Sitzungen bleiben dienstlokal und IP-gebunden; Widerruf oder Stop müssen ihre Übertragungen abbrechen.
- Downloads bleiben read-only, kanonisch enthalten, Attachment/Octet-stream und nicht cachebar.
- Uploads bleiben add-only, eigentümergebunden, nicht auflistbar und nicht überschreibend.
- Cleanup darf nur reguläre UUID-`.part`-Dateien in einem korrekt markierten, nicht reparse-basierten `.dmdc`-Ordner verändern.
- Einstellungen und privilegierte Firewalloperationen bleiben lokale Desktopabläufe.
- Diagnosen und Logs dürfen keine Dateilisten, Dateiinhalte, Codes oder Sitzungstokens aufnehmen.

## Annahmen und qualifizierende Abweichungen

- Unterstützter Produktionsumfang ist Windows 10/11 mit current-user-NSIS. Nicht-Windows-Fallbacks sind nicht v1-Produktionsumfang.
- Trusted-LAN-HTTP ohne Transportverschlüsselung ist bewusst; Schutz gegen LAN-MITM wird nicht zugesagt.
- Die Firewall ist Defense-in-depth und keine Backend-Startvoraussetzung: Die UI kann nach Warnung explizit trotzdem starten.
- `docs/ARCHITECTURE.md:21` widerspricht der Laufzeit: Ab 128 Sitzungen wird LRU verdrängt und die Übertragung abgebrochen (`src-tauri/src/service/state.rs:295-328`).
- `docs/ARCHITECTURE.md:26` nennt 8-MiB-Blöcke; wirksam ist ein exakter Offset mit beliebiger nicht leerer Blockgröße bis 8 MiB (`src-tauri/src/service/api.rs:1171-1240`).
- Konkrete Freigabepfade, Netzwerke, Ports, Diagnoseziele, Executable-Pfade und Tauri-Appverzeichnisse sind Laufzeitwerte und im Snapshot nicht vorhanden.
- Die effektiven ACLs der Tauri-Konfigurations- und Logverzeichnisse gehören zur Laufzeit-/OS-Umgebung und sind aus diesem Source-Snapshot nicht beweisbar.
- Code Signing, Auto-Update und öffentliche Veröffentlichung sind laut README außerhalb v1.
- Dieses Bedrohungsmodell ist Architekturkontext; die bestätigten Findings und ihre Validierung stehen in `docs/CODE_AND_SECURITY_AUDIT_2026-08-30.md`.
