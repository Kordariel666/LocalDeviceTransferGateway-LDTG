# DMDC – Sicherheits- und Fehlerbehebungsbericht

Datum: 2026-09-02  
Scan-ID: `86fe353e-43db-4564-99a6-3eddfc5b4844`  
Ausgangsscan: `unversioned_20260902T110711Z_s3_lqggp`

## Ausgeführter Plan

1. Upload-Lebenszyklus und dauerhafte Kapazitätsgrenzen vereinheitlichen.
2. Authentifizierung, Verbindungsannahme und Netzwerkfreigaben gegen Identitäts- und Verfügbarkeitsangriffe härten.
3. Windows-Dateisystemgrenzen mit stabilen Handles, Root-Identitäten und Autoload-Ausschlüssen absichern.
4. Sitzungswiderruf, Abschlusswiederholung und nebenläufige Abbruchpfade deterministisch machen.
5. Desktop- und Mobile-Wiederherstellung sowie täuschungssichere Darstellung korrigieren.
6. Alle Änderungen durch gezielte Regressionstests, vollständige Tests, Lints, Typprüfung, Builds und einen unabhängigen Read-only-Patchreview verifizieren.

## Ergebnis der 21 Scanbefunde

| Nr. | Finding-ID | Ergebnis | Wesentliche Korrektur |
|---:|---|---|---|
| 1 | `csf_0bcc636b6d793450acdf7da0` | behoben | Kanonisch aufgelöste Windows-Pfade werden erneut vollständig gegen Dotfile-, System- und Managed-Path-Regeln geprüft; echter NTFS-8.3-Test ergänzt. |
| 2 | `csf_7abbf4ad84e07e8caf3e392d` | behoben | Konfigurierbare, endliche Inbox-Grenzen zählen aktive und alle bereits abgeschlossenen Dateien/Bytes im Upload-Eingang und werden bei jedem neuen Upload neu mit dem Dateisystem abgeglichen. |
| 3 | `csf_7b7e107a0bf0d09ea465ae4b` | behoben | Interne Accept-Wiederholung und priorisierte Auswahl entfernt; fertige Verbindungstasks werden vor jeder Auswahl vollständig abgeholt. |
| 4 | `csf_6e2e8deac77007c38c490099` | behoben | Jeder nicht abschließende Uploadblock muss exakt 8 MiB groß sein; nur der letzte Block darf kleiner sein. |
| 5 | `csf_edfbce755b6efbbbc9991baf` | behoben | Effektive `Startup`- und `CommonStartup`-Known-Folder werden über Windows aufgelöst; entfernte und unbekannte Upload-Laufwerke werden abgewiesen. |
| 6 | `csf_8887f3268d0400fe1a053e96` | behoben | Verteilte Fehlversuche lösen nur noch eine globale Abkühlung aus und können den Zugangscode nicht mehr rotieren. |
| 7 | `csf_85929a721b40ad080d227c74` | behoben | Uploaddatei, Quota-Reservierung und Uploadrecord werden ohne Abbruchpunkt gemeinsam angelegt; danach bleibt jeder Zustand per Client-Token auffindbar oder wird durch Ablauf bereinigt. |
| 8 | `csf_474219d17d56542125026d3d` | gehärtet | Anonyme und angemeldete Kapazität sind getrennt. Windows fasst IP-Aliase aus derselben Nachbartabellen-/MAC-Identität zu einem Peerlimit zusammen; außerhalb des Subnetzes wird vor Admission verworfen. |
| 9 | `csf_038fd3716c96aa8a3cc3ada8` | behoben | Es gibt keine logische Vorreservierung der angekündigten Dateigröße mehr. Quota wird nur für tatsächlich geschriebene Bytes belastet; aktive Objekte bleiben separat begrenzt. |
| 10 | `csf_4b346c5130e59920b1785d9b` | behoben | Globale Abkühlung und IP-Sperren werden vor dem konstantzeitnahen Codevergleich geprüft; gültige Codes sind während der globalen Sperre kein ungezähltes Orakel. |
| 11 | `csf_332930c1158527462459df98` | behoben | Die Freigabe ist eine zufällige, kurzlebige Challenge, die an ID, Netzmaske, Profil-GUID und Kategorie des erneut ermittelten Netzwerkprofils gebunden ist. |
| 12 | `csf_71473fed70e2a890b146fcd5` | behoben | Downloaddateien und Verzeichniseinträge werden über geöffnete Handles gegen die verankerte Root-Identität und den endgültigen Pfad geprüft; dieselbe Datei wird anschließend verwendet. |
| 13 | `csf_cda35e00efa86db8de565996` | behoben | Partials werden einmal exklusiv mit Read/Write/Delete-Rechten geöffnet; Chunk-Schreiben, Löschen und Veröffentlichung verwenden dieses stabile Handle. |
| 14 | `csf_302dde9ef378191c807eab05` | behoben | Download-, Upload- und Partial-Roots besitzen stabile Identitäten; Austausch stoppt den Dienst, und jede sensible Operation prüft die Identität erneut. |
| 15 | `csf_55e774050f7cd8d77dc2b39d` | behoben | Langsame Synchronisation/Publikation läuft außerhalb der globalen Upload-Steuersperre in einem service-eigenen Blocking-Job. Abbruch setzt ein atomisches Signal und kann Steuerung/Shutdown fortsetzen. |
| 16 | `csf_72c57d100096b47e7f07b945` | behoben | GET, Zero-Byte-GET und HEAD prüfen die Sitzung nach der blockierenden Pfad-/Metadatenarbeit erneut, bevor Metadaten oder Inhalt zurückgegeben werden. |
| 17 | `csf_229a4574c91b8a361326bea1` | behoben | Bekannte Word-`STARTUP`- und Excel-`XLSTART`-Verzeichnisse für Benutzer und installierte Office-Versionen sind keine zulässigen Upload-Roots mehr. |
| 18 | `csf_6f442a60bc0228370baeead1` | behoben | LAN-Antworten verwenden für fehlende, geschützte, umgeleitete oder falsche Pfadtypen einheitlich `PATH_UNAVAILABLE`. |
| 19 | `csf_ff74d60614b59932b536b00b` | behoben | Bidi- und unsichtbare Formatzeichen werden in Uploadnamen ersetzt; nicht vertrauenswürdige Namen werden in Desktop und Mobile zusätzlich mit `bdi`/`unicode-bidi: isolate` dargestellt. |
| 20 | `csf_b48378a23060d4f8af8bb2dc` | behoben | Duplikat von Nr. 1; dieselbe kanonische Nachprüfung und derselbe reale 8.3-Regressionstest decken den Befund ab. |
| 21 | `csf_c3a56b7fa2e1e3d4e8244bf2` | behoben | Nicht auflösbare Windows-Profilmetadaten gelten als nicht vertrauenswürdig und können nicht ausgewählt oder dauerhaft bestätigt werden. |

## Zusätzlich behobene Funktionsfehler

- Ein verlorener Upload-Abschluss kann dieselbe begrenzte Abschlussquittung wiederholen, ohne eine zweite Datei zu veröffentlichen. Eine zufällige 128-Bit-Client-ID ermöglicht dies auch nach neuer Anmeldung oder IP-Wechsel.
- Bei Sitzungsverlust werden alle aktiven Mobile-Requests beendet, veraltete Server-IDs entfernt und die vollständige Uploadwarteschlange nach erneuter Anmeldung fortgesetzt.
- Die Desktop-App ordnet eine Dienst-URL nur noch über einen exakt geparsten Hostnamen einer Netzwerkschnittstelle zu.
- Ein fehlgeschlagener initialer Desktop-Snapshot zeigt eine Retry-Aktion, versucht begrenzt automatisch erneut und kann bei einem späteren Service-Ereignis vollständig neu geladen werden.
- Die öffentliche `.dmdc`-Markierung wird nicht länger als Eigentumsnachweis für einzelne Dateien behandelt. Nicht sicher zuordenbare Absturzreste werden bewahrt statt gelöscht.
- Der unabhängige Patchreview fand zusätzlich eine mögliche Verzeichnis-Enumeration nach Namespace-Tausch, eine IP-Wechsel-Lücke bei Abschlussquittungen und ein Race beim gleichzeitigen Upgrade einer Verbindung. Diese Punkte wurden mit handle-validierten Einträgen, adressunabhängigen 128-Bit-Recovery-Tokens und atomarer Permit-Belegung korrigiert.

## Verifikation

- Rust-Unit-/Integrationstests: **89 bestanden, 0 fehlgeschlagen**.
- Desktop-Vitest: **9 bestanden, 0 fehlgeschlagen**.
- Mobile-Vitest: **14 bestanden, 0 fehlgeschlagen**.
- TypeScript-Projektbuildprüfung für Desktop und Mobile: bestanden.
- `cargo fmt --check`: bestanden.
- `cargo clippy --all-targets --all-features -- -D warnings`: bestanden.
- Rust-Anwendungsbuild: bestanden.
- Desktop-Produktions-Webbuild: bestanden.
- Mobile-Produktions-Webbuild: bestanden.
- Unabhängiger, frischer Read-only-Patchreview: durchgeführt; vier Hinweise bewertet und die belastbaren Restkorrekturen eingearbeitet.

## Bewusste Restgrenzen

- DMDC bleibt ein HTTP-Dienst für ein bewusst bestätigtes, vertrauenswürdiges lokales Netzwerk. Transportverschlüsselung und Schutz gegen LAN-MITM sind nicht Bestandteil von Version 1.
- Nachbartabellen-/MAC-Gruppierung verhindert gewöhnliche IP-Alias-Umgehungen, ist aber keine kryptografische Geräteidentität. Ein Angreifer, der zusätzlich viele Layer-2-Identitäten fälscht oder das gesamte LAN flutet, kann auf Anwendungsebene nicht vollständig von legitimen neuen, noch nicht angemeldeten Geräten unterschieden werden.
- Echte Windows-Namespace-Races, umgeleitete Known Folders und NTFS-8.3-Verhalten werden im Code beziehungsweise soweit lokal verfügbar in Tests geprüft; eine zusätzliche adversariale Mehrprozess-/Hardware-Testumgebung bleibt sinnvoll.
- Ein aktueller Online-`pnpm audit` wurde nicht ausgeführt, weil dafür die Produktions-Abhängigkeitsliste an die npm-Registry übertragen werden müsste und diese Datenweitergabe nicht freigegeben war. `cargo-audit` ist in der lokalen Toolchain nicht installiert. Lokale Compiler-, Lockfile-, Test- und Buildprüfungen waren erfolgreich.
- Absturzreste im `.dmdc`-Ordner werden absichtlich nicht automatisch gelöscht, wenn ihre Eigentümerschaft nicht mehr über ein lebendes Dateihandle bewiesen werden kann.
