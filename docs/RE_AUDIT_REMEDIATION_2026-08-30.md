# Behebung und Verifikation des Re-Audits vom 30. August 2026

> **Historischer Zwischenstand:** Dieser Bericht hält die zweite Behebungsrunde fest. Eine spätere Härtungsrunde hat insbesondere Restgrößenreservierung, Code-Abkühlung und Absturz-Partials erneut verändert. Maßgeblich für den aktuellen Stand ist `qa/security-fix-report-2026-09-02.md` zusammen mit der aktuellen API-, Architektur-, Sicherheits- und Testdokumentation.

**Bezugsbericht:** `RE_AUDIT_REPORT_2026-08-30.md`  
**Scope:** RA-SEC-09 bis RA-SEC-16, RA-ERR-06 bis RA-ERR-09 und RA-DOC-01 sowie die beim vorgeschriebenen unabhängigen Bypass-Gegencheck zusätzlich bestätigten Pfade im selben lokalen Windows/LAN-Modell.  
**Vorgehen:** gezielte Pfadprüfung, Implementierung, Regressionstests und eine unabhängige read-only Bypass-Prüfung; kein Deep Scan und keine kosmetische Neugestaltung.

## Ergebnis

Alle im Re-Audit und im anschließenden Gegencheck bestätigten Befunde wurden an ihrer frühesten gemeinsamen Sicherheits- beziehungsweise Zustandsgrenze behoben. Die bestehenden Host-, Origin-, Subnetz-, Sitzungs-IP-, CSRF-, Rollen-, Pfad-, No-Replace- und Write-only-Eigenschaften wurden nicht abgeschwächt.

## Security-Fixes

| ID | Ursprünglicher Weg | Geschlossene Grenze und Umsetzung | Gezielter Nachweis |
| --- | --- | --- | --- |
| RA-SEC-09 | Angemeldeter Client sendet bis zu 8 MiB an eine fehlende oder fremde Upload-ID; beim ersten Fix konnten mehrere gleichzeitige PATCH-Anfragen derselben gültigen ID noch vor der serialisierten Offsetprüfung ihre Bodies puffern. | `request_guard()` authentifiziert PATCH vor dem Body und prüft ID, Existenz, Besitz und `finalizing` direkt aus URI und Uploadregistry. Noch vor Body-Polling erwirbt er außerdem genau ein Permit pro Upload-ID und eines von 8 globalen Chunk-Permits; konkurrierende Bodies erhalten `UPLOAD_CHUNK_BUSY`. Der Handler prüft Besitz, Session, Lifecycle, Offset und Größe unmittelbar vor dem Schreiben erneut. | `rejects_unknown_owned_patch_before_polling_its_authenticated_body`, `rejects_foreign_patch_before_polling_its_authenticated_body`, `rejects_parallel_patch_before_polling_a_second_body_for_the_same_upload`, unauthentifizierter Panik-Body-Test und normaler Uploadtest. |
| RA-SEC-10 | Request-Timeout/Abort ließ das Listing-Permit fallen, während `spawn_blocking` weiterlief. | Open- und Page-Permit werden jeweils in den tatsächlichen Blocking-Closure verschoben und erst nach dessen Ende freigegeben. | `dropped_listing_waiters_do_not_release_running_blocking_work`, `limits_parallel_directory_work`, Listing-/Cursortests. |
| RA-SEC-11 | Vier Sitzungen einer IP belegten mit je drei Downloads alle zwölf globalen Slots; Fortschrittsbytes verlängerten die Dauer unbegrenzt. Der erste Stream-Fix deckte einen blockierten Dateiread ab, aber nicht einen langsam zurückgestauten Hyper-Socketwrite bereits gelieferter Frames. | `begin_download()` zählt unter demselben Registry-Lock zusätzlich höchstens vier Downloads pro IP. Jede Lease endet absolut nach sechs Stunden. Zusätzlich beendet die Transport-I/O-Schicht jede Verbindung nach spätestens sechs Stunden unabhängig von Read-/Write-Fortschritt und weckt auch einen blockierten Socketwrite; damit kann die Response den Slot nicht über die Frist hinaus halten. | `one_address_cannot_consume_all_global_download_slots`, `download_lease_has_an_absolute_deadline`, `connection_lifetime_is_not_extended_by_io_progress`, Range-, Drop- und Revoke-Tests. |
| RA-SEC-12 | Vier fast leere Uploads konnten große deklarierte Restgrößen reservieren und durch minimale Chunks unbegrenzt frisch bleiben. | Am serialisierten Create-Punkt gelten Restgrößen-Budgets pro IP und Sitzung zusätzlich zu Anzahl, globaler Reservierung und 1-GiB-Reserve. Die IP erhält höchstens die Hälfte des nutzbaren freien Speichers, wobei eine konfigurativ zulässige Einzeldatei möglich bleibt; eine Sitzung erhält höchstens eine solche Datei beziehungsweise bei unbegrenzter Einzelgröße die Hälfte des IP-Budgets. Uploads enden nach 30 Minuten Idle oder 24 Stunden absolut. | `upload_reservation_budget_preserves_one_allowed_file_but_not_the_whole_disk`, `expires_upload_after_absolute_lifetime_despite_recent_progress`, Idle- und Clientanzahltests. |
| RA-SEC-13 | Ein nach 120 Sekunden verworfener Complete-Handler gab Locks frei, während `MoveFileExW` später noch veröffentlichen konnte. | Complete markiert unter einem owned Filesystem-Lock `finalizing` und startet einen dienstbesessenen Async-Commit, der Blocking-Ergebnis, Registry-Entfernung und Transferstatus selbst verarbeitet. Das Ablegen des HTTP-Waiters beendet diesen Task nicht. Complete ist vom allgemeinen Request-Timeout ausgenommen; Cancel, Revoke, Ablauf und Stop warten am selben Filesystem-Lock. Commit ist der linearisierte Gewinner, sobald `finalizing` gesetzt wurde. | `upload_commit_remains_service_owned_after_the_waiter_is_dropped`, No-Replace-, Kollisions-, Zero-Byte- und normaler Uploadtest. |
| RA-SEC-14 | Eine Sitzung konnte alle 64 Cursor halten und durch Benutzung frisch halten. | Einfügen prüft unter dem Cursor-Map-Lock zusätzlich ein Limit von vier Cursorn pro Sitzung; andere Sitzungen behalten globale Kapazität. | `limits_directory_cursors_per_session_without_blocking_other_sessions`, Bindungs-, TTL- und Page-Budgettests. |
| RA-SEC-15 | Ein Byte vor jedem 30-Sekunden-Idle-Limit hielt unvollständige HTTP-Header unbegrenzt offen. | Der Server verwendet einen explizit konfigurierten Hyper-HTTP/1-Parser mit `TokioTimer` und absoluter 15-Sekunden-Headerfrist für jeden Keep-alive-Request. Das bestehende 30-Sekunden-I/O-Idle-Limit und die Connection-Caps bleiben bestehen; Body- und Responsezeiten werden nicht mit der Headerfrist verwechselt. | `absolute_header_timeout_is_not_extended_by_drip_fed_bytes`, `idle_connection_io_times_out`, Connection-Cap-Test. |
| RA-SEC-16 | Der dienstweite Block wurde vor Codevergleich ausgewertet und sperrte daher auch den neu rotierten korrekten Code. | Ein IP-spezifischer Block bleibt vorrangig. Danach wird konstantzeitlich der aktuelle Code geprüft; der korrekte Code wird angenommen. Die globale Abkühlung blockiert nur weitere falsche Versuche. Schwellenwert, sofortige Rotation und bestehende Sitzungen bleiben unverändert. | `correct_rotated_code_recovers_during_global_wrong_attempt_cooldown` sowie der erweiterte verteilte 49+1-API-Test. |

## Funktionale und Dokumentations-Fixes

| ID | Umsetzung | Gezielter Nachweis |
| --- | --- | --- |
| RA-ERR-06 | Ein `lifecycle_transition`-Mutex serialisiert Start, Stop/Force-Stop, Quit, Status-Reaping, Settings-Save und Firewallkonfiguration über ihre vollständigen Await-Grenzen. Ein Stop kann keinen noch nicht veröffentlichten Start mehr überholen; ein alter Reap kann keinen neueren Startzustand zurücksetzen. | `service_stop_waits_for_the_active_lifecycle_transition`; bestehende Status-, Stop- und Buildpfade. |
| RA-ERR-07 | Die Mobile-App besitzt eine ref-basierte sequenzielle Queue als Ausführungsautorität. Nur `resumeUpload()` entfernt Pause. Statusabgleich verwendet AbortController, Chunks verwenden XHR-Abbruch und vor/nach jedem Await, Retry und Chunk wird Pause/Cancel geprüft. Eine bereits gesendete Create-Anfrage bleibt absichtlich unabgebrochen: Ihre Antwort-ID wird zuerst im Queuezustand gespeichert; Pause stoppt danach, Abbruch löscht die nun bekannte Server-ID best-effort. Dadurch kann ein serverseitig bereits angelegter Upload weder unsichtbar verloren gehen noch beim Fortsetzen dupliziert werden. Bereits terminale Queueeinträge werden nicht erneut verarbeitet, auch wenn ein sofortiges Resume noch einen Eintrag vorgemerkt hatte. `finalizing` ist ein eigener nicht pausierbarer/nicht abbrechbarer UI-Zustand passend zum Backend-Commit. | Mobile-Tests für aktiven XHR, Resume mit derselben Server-ID, ID-Übernahme bei Pause während Create, sofortiges Resume während Create, nachträgliches DELETE bei Abbruch während Create, Retry-Backoff, pausiertes Queueelement und Finalisierung. |
| RA-ERR-08 | Der NSIS-Hook enthält keine rekursive AppData-Löschung mehr. Konfiguration, Logs und mögliche Nutzdaten bleiben beim Uninstall erhalten; die privilegierte Firewallregel-Entfernung bleibt unverändert. | `uninstaller_never_recursively_deletes_application_data`; erfolgreicher NSIS-Build. Ein realer Sentinel-Uninstall bleibt Bestandteil der manuellen Windows-10/11-Freigabeprüfung. |
| RA-ERR-09 | Jede weitere Dateiauswahl wird auch während einer laufenden Übertragung atomar an die bestehende Queue angehängt und sofort sichtbar. Der Picker verwirft keine Auswahl mehr still. | Mobile-Test `nimmt weitere Dateien während eines laufenden Uploads sichtbar in die Warteschlange auf`. |
| RA-DOC-01 | Der Desktoptext nennt jetzt die tatsächliche Backendregel: Download- und Uploadfreigabe dürfen weder gleich sein noch ineinander liegen. Architektur, API, Testplan und README beschreiben ebenfalls die neuen Laufzeitgrenzen. | Desktop-Suite und Quelltextabgleich mit `prepare_roots()`. |

## Ergebnis des unabhängigen Bypass-Gegenchecks

Nach der ersten Implementierung wurde genau eine frische, read-only Gegenprüfung gegen die ursprünglichen Angriffs- und Fehlerwege ausgeführt. Sie bestätigte vier noch umgehbare beziehungsweise unvollständige Grenzen und eine Dokumentationslücke. Diese Punkte wurden anschließend behoben und mit gezielten Regressionen sowie der vollständigen Matrix verifiziert; es wurde kein zweiter Review-Zyklus und kein Deep Scan gestartet.

| Zuordnung | Schweregrad | Reproduktionsweg und Evidenz | Betroffene Grenze | Umgesetzte priorisierte Empfehlung |
| --- | --- | --- | --- | --- |
| RA-SEC-09, paralleler gültiger PATCH | Mittel | Mehrere authentisierte PATCH-Anfragen derselben eigenen ID und desselben Offsets passierten die frühe Besitzprüfung gleichzeitig. Die Bodies konnten vor der späteren `UploadRecord`-Sperre gepuffert werden, obwohl nur ein Handler schreiben durfte. | `request_guard()`, `upload_chunk()`, `TransferServiceState::begin_upload_chunk()` in `src-tauri/src/service/api.rs` und `state.rs`. | P1 umgesetzt: pro Upload-ID genau ein In-flight-Permit plus global 8 Chunk-Permits im Middlewarepfad vor Body-Polling; Panik-Body-Regression beweist die frühe Ablehnung. |
| RA-SEC-11, langsamer Response-Write | Mittel | Nach einem erfolgreich gelesenen Dateiframe konnte Hyper beim Schreiben an einen extrem langsamen Client hängen. Die Lease-Prüfung im Body-Stream wurde ohne erneutes Polling nicht ausgeführt. | `AcceptedIo::poll_write()`/`poll_read()` in `src-tauri/src/service/mod.rs` und Downloadlease in `state.rs`. | P1 umgesetzt: absoluter, selbst weckender Transport-Deadline von 6 Stunden zusätzlich zur Downloadlease; fortlaufende Bytes verlängern ihn nicht. |
| RA-ERR-07, Pause/Abbruch während Create | Niedrig | Der Browser konnte `POST /uploads` abbrechen, nachdem der Server die ID angelegt, aber bevor die Antwort den Client erreicht hatte. Pause/Resume erzeugte dann eine zweite ID; Cancel hinterließ eine unsichtbare Reservierung. | `getOrCreateUpload()`, `processUpload()`, `cancelUpload()` in `apps/mobile/src/App.tsx`. | P1 umgesetzt: Create nicht abbrechen, Antwort-ID synchron übernehmen, dann Pause beachten beziehungsweise bei bereits gesetztem Cancel best-effort DELETE senden. Zwei neue UI-Regressionen decken beide Übergänge ab. |
| NEW-SEC-17, aktive Listing-Fairness | Mittel | Eine Sitzung beziehungsweise die maximalen Sitzungen derselben IP konnten alle 4 Blocking-Listing-Permits belegen und damit andere LAN-Geräte bis zum Ende der Jobs verdrängen. | `TransferServiceState::begin_listing()` und `list_downloads()` in `src-tauri/src/service/state.rs` und `api.rs`. | P1 umgesetzt: aktive Jobs global 4, pro IP 2 und pro Sitzung 1; Permits bleiben weiterhin im Blocking-Closure. `one_session_or_address_cannot_consume_all_active_listing_slots` belegt die Fairness. |
| NEW-DOC-02, historischer Status ohne aktuellen Verweis | Niedrig | Der Re-Audit-Bericht beschrieb offene Befunde ohne gut sichtbaren Link auf deren Behebung; das historische Threat Model verwies nur auf die erste Remediation-Runde. | `docs/RE_AUDIT_REPORT_2026-08-30.md`, `docs/THREAT_MODEL_2026-08-30.md`, dieser Bericht. | P2 umgesetzt: historische Dokumente klar gekennzeichnet und auf diesen datierten Abschlussbericht verlinkt; historische Evidenz wurde nicht umgeschrieben. |

## Geänderte Bereiche

- Desktop-Lifecycle und Uninstall-Invariante: `src-tauri/src/lib.rs`, `src-tauri/windows/hooks.nsh`.
- HTTP-Server und Parserfrist: `src-tauri/src/service/mod.rs`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`.
- Auth-, Cursor-, Download-, Upload- und Commitzustand: `src-tauri/src/service/state.rs`, `src-tauri/src/service/api.rs`.
- Mobile Queue und Regressionen: `apps/mobile/src/App.tsx`, `apps/mobile/src/App.test.tsx`, `apps/mobile/src/i18n.ts`.
- Desktopvertragstext: `apps/desktop/src/i18n.ts`.
- Vertragsdokumentation: `README.md`, `docs/ARCHITECTURE.md`, `docs/API.md`, `docs/TESTPLAN.md` und dieser Bericht.

## Verifikation

| Prüfung | Ergebnis |
| --- | --- |
| `pnpm typecheck` | bestanden; Desktop und Mobile |
| `pnpm test` | bestanden; Desktop 7/7, Mobile 11/11 |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/test-rust.ps1` | bestanden; 70/70 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` | bestanden |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings` | bestanden; keine Quellwarnung |
| `pnpm build:web` | bestanden; Mobile 30 Module, Desktop 35 Module |
| `pnpm build` | bestanden; Release-Binärdatei und NSIS-Installer |

Erzeugter Installer:

- Pfad: `src-tauri/target/release/bundle/nsis/DMDC_0.1.3_x64-setup.exe`
- Größe: 3.266.057 Bytes
- SHA-256: `A6090ECEB8D61B09D939D47432F53CADD36713554F18DEF99BE037031A123556`

Die isolierte Vitest-Ausführung konnte zunächst den esbuild-Elternpfad nicht lesen; derselbe Befehl lief mit dem bereitgestellten Workspace-Node außerhalb dieser Dateisandbox vollständig erfolgreich. Ein direkter `cargo test`-Start außerhalb des projektspezifischen Wrappers traf den bekannten Windows-Test-DLL-Ladefehler; `scripts/test-rust.ps1` initialisierte die dokumentierte MSVC-/Manifest-Umgebung und führte alle 70 Tests erfolgreich aus. Der Release-Linker meldete nur die informative Erzeugung von `.dll.lib` und `.dll.exp`; Release und NSIS-Bundle wurden erfolgreich abgeschlossen.

## Verbleibende Release-Prüfung

Keine bestätigte Code-Lücke aus dem Re-Audit bleibt offen. Vor einer öffentlichen Freigabe bleibt der bereits im Testplan vorgesehene manuelle Windows-10/11-Installations- und Deinstallationstest mit Sentinel-Nutzdaten sinnvoll; er prüft das Verhalten des erzeugten Installers in einer realen Benutzerumgebung, während die automatisierte Invariante bereits jede rekursive AppData-Löschung im Hook verbietet.
