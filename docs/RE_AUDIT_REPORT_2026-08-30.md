# Re-Audit nach Remediation vom 30. August 2026

> **Historischer Befundstand vor dieser Behebung:** Der umgesetzte Status, die zusätzliche Bypass-Prüfung und die erneute vollständige Verifikation stehen in `RE_AUDIT_REMEDIATION_2026-08-30.md`.

## Ergebnis

Der aktuelle Checkout wurde nach der dokumentierten Audit-Behebung erneut unabhängig geprüft. Der zuvor auf Nutzerwunsch abgebrochene Deep Scan wurde nicht als Beleg übernommen. Stattdessen erfolgten genau ein Standard-Sicherheitsdurchlauf, eine gezielte manuelle Verifikation der historischen Befunde und die vollständige lokale Test- und Build-Matrix.

Die ursprünglichen Angriffs- und Fehlerwege von **SEC-01 bis SEC-08** und **ERR-01 bis ERR-05** sind im aktuellen Quellstand geschlossen. Es wurden jedoch acht neue Sicherheits-/Verfügbarkeitsbefunde und fünf funktionale beziehungsweise Dokumentationsbefunde bestätigt:

- 0 kritisch
- 0 hoch
- 8 mittel
- 5 niedrig

Von den acht mittleren Befunden sind fünf sicherheitsrelevant und drei funktional. Von den fünf niedrigen Befunden sind drei sicherheitsrelevant, ein funktionaler Bedienfehler und eine veraltete UI-Erklärung.

Am Produktcode wurde nichts geändert. Diese Datei ist die einzige absichtliche Quellbaumänderung des Re-Audits; Tests und Builds haben ausschließlich generierte Artefakte unter `dist` und `target` aktualisiert.

## Prüfumfang und Methode

Vollständig gelesen und gegen den aktuellen Quellstand abgeglichen wurden:

- `SECURITY.md`
- `README.md`
- `docs/ARCHITECTURE.md`
- `docs/API.md`
- `docs/TESTPLAN.md`
- `docs/CODE_AND_SECURITY_AUDIT_2026-08-30.md`
- `docs/THREAT_MODEL_2026-08-30.md`
- `docs/FIX_PLAN_2026-08-30.md`
- `docs/REMEDIATION_REPORT_2026-08-30.md`
- Desktop- und Mobile-Quellen sowie Tests unter `apps`
- Shared-Paket unter `packages/shared`
- Rust-Backend, Domänenlogik, Windows-Plattformcode, Tauri-Konfiguration, Capability und NSIS-Hooks unter `src-tauri`
- Root-Manifeste, Lockfiles und `scripts/test-rust.ps1`

Die Bewertung respektiert die Grenzen aus `SECURITY.md`: Trusted-LAN-HTTP ist beabsichtigt; LAN-MITM, Internetexposition, Portweiterleitung, UPnP und der Betrieb in einem nicht vertrauenswürdigen Netzwerk wurden nicht als Produktlücken bewertet. `node_modules`, `.pnpm-store`, `dist`, `target`, Git-Metadaten und binäre Bildressourcen wurden nicht als eigenständige Produktquellen auditiert. Eine aktuelle Online-CVE-/Advisory-Abfrage war nicht Bestandteil dieser Prüfung.

Der abgeschlossene Standard-Sicherheitslauf trägt die ID `572a7323-cf9d-442b-8749-49cece137518`. Seine kanonische Auswertung enthält acht Sicherheitsbefunde, davon fünf mittel und drei niedrig. Der Scanner meldete am Ende eine Snapshot-Änderung, weil während des Laufs die verlangten Web-/Rust-/Installer-Builds `dist` und `target` aktualisierten. Eine anschließende Zeitstempelprüfung fand keine nach Scanbeginn veränderte Produktquelldatei außerhalb dieser generierten Verzeichnisse.

## Erneute Prüfung der früheren Befunde

| ID | Ergebnis | Konkrete Evidenz |
| --- | --- | --- |
| SEC-01 | geschlossen | `request_guard()` authentifiziert PATCH inklusive CSRF vor dem Body; `rejects_patch_before_polling_an_unauthenticated_body` besteht. Der neue authentifizierte Vorpuffer-Fall steht separat als RA-SEC-09. |
| SEC-02 | geschlossen | Achtstelliger Code, konstante Prüfung, IP- und globale Schwellen sowie Code-Rotation sind aktiv. Der ursprüngliche verteilte Rate-Bypass ist geschlossen; die verbleibende Admission-DoS-Eigenschaft steht separat als RA-SEC-16. |
| SEC-03 | geschlossen | `create_session()` weist bei 128 globalen beziehungsweise vier Sitzungen pro IP ab, ohne bestehende Sitzungen zu verdrängen; der Regressionstest besteht. |
| SEC-04 | geschlossen | `same_network_identity()` vergleicht Adapter-ID, Netzmaske und vollständige Netzwerk-ID; ein Profilwechsel bei gleicher Adresse wird erkannt. |
| SEC-05 | geschlossen | Kanonische Benutzer- und systemweite Windows-Autostartpfade einschließlich Unterordnern werden als Uploadziel abgewiesen; der Regressionstest besteht. |
| SEC-06 | geschlossen | `prepare_roots()` vergleicht kanonische Wurzeln auf Gleichheit und beide Verschachtelungsrichtungen; Alias-/Nested-Tests bestehen. |
| SEC-07 | geschlossen | Eine Seite untersucht höchstens 256 Roh- und liefert höchstens 200 sichtbare Einträge; Cursor, TTL und vier Listing-Slots begrenzen den ursprünglichen Vollscanpfad. Neue Fairness-/Abbruchprobleme stehen als RA-SEC-10 und RA-SEC-14. |
| SEC-08 | geschlossen | Auth-Versuchsdatensätze besitzen TTL und eine feste Kapazität von 1.024; der Regressionstest besteht. |
| ERR-01 | geschlossen | Upload-ID und `File` bleiben an dasselbe In-Memory-`UploadItem` gebunden; es existiert kein `localStorage`-Resume-Mapping mehr. |
| ERR-02 | geschlossen | Navigation verwendet `AbortController` und eine monotone Request-ID; der Test mit vertauschter Antwortreihenfolge besteht. |
| ERR-03 | geschlossen | Serve- und Join-Fehler setzen einen sichtbaren Stopgrund; beide Rust-Tests bestehen. Der neue Start-/Stop-Übergangsrace steht separat als RA-ERR-06. |
| ERR-04 | geschlossen | Dateinamen- und Gesamtpfadbudget werden in UTF-16-Einheiten begrenzt; Kollisionssuffixe bleiben im Budget; die Tests bestehen. |
| ERR-05 | geschlossen | Nur `NotFound` lädt still Defaults. Lese-/Parsefehler bleiben erhalten, werden sichtbar gemeldet und vor Ersatz gesichert; Rust- und Desktoptests bestehen. |

## Bestätigte neue Sicherheitsbefunde

### RA-SEC-09 — Authentifizierter PATCH-Body vor Upload-Besitzprüfung

**Schweregrad:** mittel  
**Priorität:** P1  
**Betroffen:** `src-tauri/src/service/api.rs:107`, `210`, `1272-1320`; `request_guard()`, `upload_chunk()`, `owned_upload()`

**Evidenz:** `request_guard()` schließt SEC-01 für unangemeldete Clients, prüft bei PATCH aber nur Sitzung und CSRF. Im Handler wird anschließend ein `Bytes`-Body von bis zu `CHUNK_SIZE + 1024` vollständig extrahiert, bevor `owned_upload()` Existenz und Besitzer der ID prüft. Die gleichzeitigen Requestgrenzen erlauben acht Anfragen pro IP und 64 global. Damit können vor einer billigen Ablehnung ungefähr 64 MiB pro IP beziehungsweise rund 512 MiB global gepuffert werden.

**Reproduktion:** Mit gültiger Sitzung und CSRF acht parallele 8-MiB-PATCH-Anfragen derselben IP an erfundene Upload-IDs senden. Die Bodies werden gelesen, bevor `UPLOAD_NOT_FOUND` oder `UPLOAD_OWNER_MISMATCH` entsteht. Mit mehreren Quelladressen lässt sich das globale Limit ausnutzen.

**Empfehlung:** Upload-ID, Besitzer, aktiven Sitzungszustand und möglichst den Offset in einer Parts-/Middleware-Prüfung vor Body-Extraktion validieren und unmittelbar vor dem Schreiben sicher erneut bestätigen. Ein Regressionstest muss für eine angemeldete fremde/unbekannte ID beweisen, dass der Body nicht gepollt wird.

### RA-SEC-10 — Blocking-Ordnerarbeit überlebt Timeout und Listing-Limit

**Schweregrad:** mittel  
**Priorität:** P1  
**Betroffen:** `src-tauri/src/service/api.rs:215`, `626-701`; `request_guard()`, `list_downloads()`

**Evidenz:** Ordneröffnung und Seitenscan laufen in `spawn_blocking`. Das Permit für maximal vier aktive Listings liegt im abbrechbaren Handler-Future. Nach 120 Sekunden verwirft `request_guard()` diesen Future und gibt das Permit frei; ein gestarteter `spawn_blocking`-Task wird durch Drop seines JoinHandles jedoch nicht abgebrochen. Auf einer langsamen oder hängenden, aber von `validate_root()` nicht ausgeschlossenen UNC-/Netzfreigabe können wiederholte Anfragen daher mehr als vier reale Blocking-Arbeiten akkumulieren.

**Reproduktion:** Eine zulässige Freigabe verwenden, deren `read_dir`/Metadatenzugriffe länger als 120 Sekunden blockieren. Listing anfordern, HTTP-Timeout abwarten und wiederholen. Jeder Request gibt seinen äußeren Slot frei, während die frühere Blocking-Arbeit weiterlaufen kann.

**Empfehlung:** Das Permit in die tatsächliche Blocking-Arbeit verschieben oder eine dauerhaft begrenzte Worker-Queue verwenden, deren Kapazität auch nach Request-Abbruch belegt bleibt. Tests sollen blockierte Arbeit simulieren und beweisen, dass nie mehr als vier Aufgaben laufen.

### RA-SEC-11 — Eine IP kann alle Download-Slots monopolisieren

**Schweregrad:** mittel  
**Priorität:** P1  
**Betroffen:** `src-tauri/src/service/state.rs:29-33`, `577-609`, `690-725`; `src-tauri/src/service/mod.rs:96-140`

**Evidenz:** LDTG erlaubt vier Sitzungen pro IP, drei Downloads pro Sitzung und zwölf Downloads global. Eine Per-IP-Downloadquote fehlt. Damit kann eine einzelne angemeldete Adresse exakt alle zwölf globalen Slots belegen. `AcceptedIo` erneuert die Schreib-Idlefrist bei jedem Fortschritt; eine absolute Transferlease oder Mindestdurchsatzgrenze fehlt.

**Reproduktion:** Von derselben IP vier getrennte Cookies/Sitzungen anlegen, pro Sitzung drei große Dateien herunterladen und jeweils nur so viel lesen, dass die 30-Sekunden-Idlefrist erneuert wird. Andere Geräte erhalten danach das globale Downloadlimit.

**Empfehlung:** Eine Per-IP-Downloadquote unterhalb des globalen Limits ergänzen und zusätzlich eine großzügige absolute Lease oder Mindestdurchsatzregel für Streams einführen.

### RA-SEC-12 — Deklarierte Uploadgrößen können freien Speicher ohne Daten reservieren

**Schweregrad:** mittel  
**Priorität:** P1  
**Betroffen:** `src-tauri/src/domain/settings.rs:9`; `src-tauri/src/service/state.rs:28-31`; `src-tauri/src/service/api.rs:1108-1198`; `create_upload()`

**Evidenz:** Das Standardlimit beträgt 20 GiB pro Datei. `create_upload()` summiert die deklarierte Restgröße aller aktiven Uploads als Reservierung, legt selbst aber zunächst nur eine leere `.part`-Datei an. Eine IP darf vier Uploads halten und kann damit bis zu 80 GiB rechnerisch reservieren. Ein gelegentlich erfolgreich gespeichertes Byte erneuert `last_activity` und verhindert den 30-Minuten-Ablauf.

**Reproduktion:** Mit einer Uploadsitzung vier Uploads mit großen `size`-Werten anlegen, keine oder nur selten minimale Chunks senden und danach von einem zweiten Gerät einen legitimen Upload anlegen. Dieser kann mit `DISK_FULL` scheitern, obwohl die Angreiferdateien fast keinen Platz belegen.

**Empfehlung:** Per-IP-/Per-Session-Bytebudgets für reservierte Restgröße einführen und Reservierungen an absoluten Lease-Ablauf beziehungsweise Mindestfortschritt binden.

### RA-SEC-13 — Upload-Publikation kann nach Timeout/Abbruch weiterlaufen

**Schweregrad:** mittel  
**Priorität:** P1  
**Konfidenz:** mittel; benötigt eine ungewöhnlich lange blockierende Dateisystemoperation  
**Betroffen:** `src-tauri/src/service/api.rs:215`, `1395-1535`; `src-tauri/src/service/state.rs:655-681`; `src-tauri/src/service/mod.rs:216-233`

**Evidenz:** `complete_upload()` hält `upload_fs_lock` und den Uploadrecord, wartet aber auf `spawn_blocking(publish_new)`. Nach dem 120-Sekunden-Timeout wird der Handler verworfen; der Blocking-Task läuft weiter, während Sperren freigegeben werden. Widerruf, Abbruch oder Dienststopp können danach bereinigen und Erfolg melden, obwohl die abgekoppelte `MoveFileExW`-Operation später noch eine fertige Datei veröffentlicht.

**Reproduktion:** Einen vollständigen Upload auf einer zulässigen, gestörten Netz-/Remotefreigabe finalisieren, `publish_new()` länger als 120 Sekunden blockieren lassen, Timeout abwarten und Sitzung widerrufen, Upload abbrechen oder Dienst stoppen. Kehrt die Operation danach erfolgreich zurück, kann die Datei trotz sichtbarem Abbruch erscheinen.

**Empfehlung:** Finalisierung in einen dienstbesessenen, journalisierten Commit-Zustand überführen. Timeout, Stop und Widerruf müssen einen Commit entweder vor Beginn verhindern oder seinen Ausgang abwarten und konsistent melden.

### RA-SEC-14 — Eine Sitzung kann alle 64 Verzeichniscursor halten

**Schweregrad:** niedrig  
**Priorität:** P2  
**Betroffen:** `src-tauri/src/service/state.rs:36`, `477-545`; `src-tauri/src/service/api.rs:637-698`

**Evidenz:** Die Cursor-Map ist nur global auf 64 begrenzt. `owner_session` wird beim Einfügen nicht quotiert, und jeder gültige Zugriff erneuert `last_activity`. Eine Sitzung kann daher alle Cursor anlegen und dauerhaft frisch halten.

**Reproduktion:** In einem Verzeichnis mit genügend Einträgen 64 frische Listing-Anfragen unter derselben Sitzung senden und alle gelieferten Cursor jeweils vor Ablauf erneut verwenden. Neue Listing-Anfragen anderer Sitzungen erhalten `DIRECTORY_CURSOR_LIMIT`.

**Empfehlung:** Ein kleines Per-Session-Cursorlimit am gesperrten Einfügepunkt ergänzen; bei Überschreitung den ältesten inaktiven Cursor derselben Sitzung entfernen oder die Neuanlage ablehnen.

### RA-SEC-15 — Langsame Teil-Header halten TCP-Slots unbegrenzt

**Schweregrad:** niedrig  
**Priorität:** P2  
**Betroffen:** `src-tauri/src/service/mod.rs:31-33`, `76-143`; `src-tauri/src/service/api.rs:215`

**Evidenz:** `AcceptedIo` setzt die 30-Sekunden-Lese- und Schreibfrist nach jedem übertragenen Byte zurück. Der 120-Sekunden-Request-Timeout greift erst nach einem vollständig geparsten Request. Unvollständige Header können mit einem Byte kurz vor Ablauf unbegrenzt offen bleiben. Das Per-IP-Limit von zwölf verhindert den vollständigen DoS durch nur eine Quelladresse; acht Adressen/IP-Aliase können jedoch alle 96 Slots halten.

**Reproduktion:** Verbindungen öffnen, unvollständige HTTP-Header senden und vor jeder 30-Sekunden-Grenze ein weiteres Byte übertragen. Mit genügend Quelladressen bleiben alle globalen Connection-Permits belegt.

**Empfehlung:** Zusätzlich eine absolute Header-/Handshake-Deadline einführen, die durch Fortschritt nicht verlängert wird.

### RA-SEC-16 — Globaler Anti-Bruteforce-Block sperrt korrekte Neuanmeldungen

**Schweregrad:** niedrig  
**Priorität:** P2  
**Betroffen:** `src-tauri/src/service/state.rs:43-45`, `370-445`; `src-tauri/src/service/api.rs:329-403`, Test ab `1828`

**Evidenz:** Nach 50 Fehlversuchen in fünf Minuten setzt LDTG einen globalen Fünf-Minuten-Block und rotiert den Code. Der Block wird vor dem Vergleich des gelieferten Codes geprüft. Der vorhandene Test `distributed_failures_rotate_and_block_the_service_code` bestätigt diesen Zustand ausdrücklich.

**Reproduktion:** Fünf Quelladressen mit jeweils zehn Fehlversuchen verwenden. Während `global_blocked_until` erhält auch ein Gerät mit dem neuen korrekten Code `SERVICE_CODE_BLOCKED`. Der Zyklus lässt sich wiederholen; bestehende Sitzungen bleiben allerdings aktiv.

**Empfehlung:** Global verteiltes Raten durch Verzögerung und Rotation begrenzen, ohne alle korrekten Neuanmeldungen hart zu sperren, oder eine operatorbestätigte sofortige Wiederfreigabe vorsehen.

## Bestätigte funktionale und Dokumentationsbefunde

### RA-ERR-06 — Start/Stop/Status sind nicht als atomarer Dienstübergang serialisiert

**Schweregrad:** mittel  
**Priorität:** P1  
**Betroffen:** `src-tauri/src/lib.rs:123-138`, `177-203`, `255-314`; `stop_runtime()`, `current_service_status()`, `start_service()`

**Evidenz:** `start_service()` prüft `runtime.service.is_none()`, gibt den Mutex frei und führt Validierung, Persistierung, Bind und `service::start()` aus. Erst danach wird der Handle gespeichert. Ein paralleles `stop_runtime()` sieht in diesem Fenster `None`, meldet Erfolg und setzt `running=false`; anschließend kann der Start den Dienst trotzdem veröffentlichen und `running=true` setzen. Tray- und UI-Ereignisse liefern zwei reale Aufrufer. Ein ähnliches Fenster besteht beim Einsammeln eines beendeten Handles: Während `finish().await` ist der Slot leer, ein neuer Start kann erfolgen und der alte Statuspfad danach das atomare `running`-Flag wieder auf `false` setzen. Auch `save_settings()` kann während eines noch nicht eingetragenen Starts alte/neue Persistenzstände überkreuzen.

**Reproduktion:** `start_service` und kurz danach `stop_service` beziehungsweise den Tray-Stop parallel auslösen, bevor der Listenerhandle in `runtime.service` eingetragen ist. Stop kann erfolgreich zurückkehren, obwohl Start danach den Dienst aktiviert. Für den zweiten Pfad einen beendeten Handle über `get_service_status` einsammeln und während `finish().await` erneut starten.

**Empfehlung:** Einen expliziten Übergangszustand (`Stopped`, `Starting`, `Running`, `Stopping`) beziehungsweise einen separaten Transition-Mutex einführen. Start, Stop, Quit, Save und Status-Reaping müssen denselben linearisierbaren Zustandsautomaten verwenden. Konkurrenztests mit kontrollierten Barrieren ergänzen.

### RA-ERR-07 — Mobile Pause ist nicht für alle Uploadzustände verbindlich

**Schweregrad:** mittel  
**Priorität:** P1  
**Betroffen:** `apps/mobile/src/App.tsx:151-213`; `processUpload()`, `queueFiles()`, `pauseUpload()`

**Evidenz:** `processUpload()` löscht zu Beginn immer `paused.current`, auch wenn ein in der Warteschlange pausiertes Element später automatisch an die Reihe kommt. Nach `getOrCreateUpload()`, während Retry-Backoff und unmittelbar vor `complete` wird Pause nicht erneut geprüft. Nur ein gerade aktiver XHR wird zuverlässig durch `abort()` unterbrochen. Der vorhandene Test prüft ausschließlich diesen aktiven-XHR-Fall.

**Reproduktion:** Zwei Dateien auswählen, während die erste läuft bei der zweiten `Pausieren` drücken und die erste abschließen lassen. Die Queue ruft `processUpload()` für die zweite auf, entfernt den Pausemarker und startet ohne `Fortsetzen`. Alternativ während Uploadanlage, Retry-Wartezeit oder Finalisierungs-POST pausieren; die Operation kann weiterlaufen oder als vollständig enden.

**Empfehlung:** Einen einzigen expliziten Uploadzustandsautomaten verwenden. Vor und nach jedem `await`, vor jedem Retry/Chunk und vor Finalisierung Pause/Cancel prüfen. Automatische Queueverarbeitung darf pausierte Elemente überspringen und nur eine explizite Resume-Aktion darf sie reaktivieren. Tests für queued, creating, backoff und finalizing ergänzen.

### RA-ERR-08 — Uninstaller kann einen konfigurierten Freigabeordner rekursiv löschen

**Schweregrad:** mittel  
**Priorität:** P1  
**Betroffen:** `src-tauri/windows/hooks.nsh:5-7`; `src-tauri/src/domain/shares.rs:40-160`; `validate_root()`

**Evidenz:** Der NSIS-Post-Uninstall-Hook löscht rekursiv und ohne Besitzprüfung `$APPDATA\de.ldtg.desktop` und `$LOCALAPPDATA\de.ldtg.desktop`. `validate_root()` verbietet Windows-, Programm-, PATH-, PowerShell-Modul- und Autostartpfade, nicht aber diese beiden AppData-Bäume. Ein Operator kann daher einen Download- oder Uploadordner darin auswählen und später durch Deinstallation alle dortigen Nutzdaten verlieren. Die historische Dokumentation bezeichnet diese Verzeichnisse pauschal als app-spezifisch und sicher löschbar.

**Reproduktion:** Unter einem der beiden Hook-Ziele einen Ordner mit Nutzdaten anlegen, ihn als Freigabe konfigurieren und die Anwendung deinstallieren. Der Hook entfernt den gesamten übergeordneten Baum rekursiv.

**Empfehlung:** Keine Freigabe innerhalb der später rekursiv gelöschten AppData-Ziele zulassen und beim Uninstall ausschließlich eindeutig LDTG-eigene Konfigurations-/Logdateien entfernen. Alternativ Daten standardmäßig erhalten und eine explizite, verständliche Löschoption anbieten.

### RA-ERR-09 — Weitere Dateiauswahl während Upload wird still verworfen

**Schweregrad:** niedrig  
**Priorität:** P2  
**Betroffen:** `apps/mobile/src/App.tsx:191-198`, `276`; `queueFiles()` und Dateiauswahl

**Evidenz:** Der Dateipicker bleibt während `uploading=true` aktiv. `queueFiles()` kehrt in diesem Zustand ohne Meldung zurück; der `onChange`-Handler leert anschließend trotzdem den Inputwert. Die ausgewählten Dateien erscheinen nicht in der Queue und der Nutzer erhält keinen Fehler.

**Reproduktion:** Einen längeren Upload starten und währenddessen erneut Dateien über den weiterhin aktiven Picker auswählen. Die Auswahl verschwindet ohne Queueeintrag oder Rückmeldung.

**Empfehlung:** Entweder den Picker während aktiver Batch-Verarbeitung deaktivieren oder neue Elemente atomar an eine fortlaufende Queue anhängen. Das stille `return` durch sichtbares Verhalten ersetzen und testen.

### RA-DOC-01 — Same-Folder-Hinweis beschreibt nicht mehr das Backendverhalten

**Schweregrad:** niedrig  
**Priorität:** P2  
**Betroffen:** `apps/desktop/src/i18n.ts:86`; Anzeige in `apps/desktop/src/DesktopApp.tsx:460-562`

**Evidenz:** Der Hinweis behauptet, bei demselben Ordner würden neue Uploads im Downloadbereich sichtbar. Das Backend weist gleiche und verschachtelte kanonische Wurzeln inzwischen vollständig ab. Die Meldung erklärt daher einen Zustand, der nicht mehr gestartet werden kann, statt die tatsächliche Validierungsregel zu nennen.

**Reproduktion:** In beiden Rollen denselben rohen Pfad wählen. Die UI zeigt die alte Sichtbarkeitswarnung; Start endet anschließend mit der Backend-Ablehnung.

**Empfehlung:** Text auf „gleiche oder verschachtelte Freigaben sind nicht zulässig“ aktualisieren und optional bereits vor Start blockieren. Keine kosmetische Neugestaltung erforderlich.

## Test- und Buildnachweise

| Prüfung | Ergebnis | Evidenz / Untersuchung auffälliger Ausgaben |
| --- | --- | --- |
| `pnpm typecheck` | bestanden | Desktop und Mobile jeweils `tsc -b --pretty false`. Der erste Lauf fand `node` nicht, weil die isolierte Runner-PATH den gebündelten Node-Pfad nicht enthielt; mit explizitem Workspace-Node lief er fehlerfrei. Kein Produktfehler. |
| `pnpm test` | bestanden | Desktop: 1 Datei, 7/7 Tests. Mobile: 1 Datei, 5/5 Tests. Der erste Lauf scheiterte nur an der Dateisandbox beim Laden der Vite-Konfiguration; außerhalb dieser Einschränkung bestanden beide Suiten. |
| `pnpm test:rust` / Wrapper | bestanden | 53/53 Rust-Tests, 0 fehlgeschlagen, 0 ignoriert. |
| `cargo fmt --all -- --check` | bestanden | Exitcode 0. Die Runner-Warnung, das Benutzerprofil nicht kanonisieren zu können, stammt aus der Sandbox und zeigt keinen Formatfehler. |
| `cargo clippy --all-targets --all-features -- -D warnings` | bestanden | Exitcode 0, keine Clippy-/Quellwarnung. Dieselbe Sandbox-Kanonisierungswarnung wie oben. |
| `pnpm build:web` | bestanden | Mobile: 30 Module; Desktop: 35 Module; beide Vite-Produktionsbuilds erfolgreich. |
| `pnpm build` | bestanden | Release-Binary gebaut und NSIS erfolgreich ausgeführt. Die MSVC-Linkerausgabe zur Erstellung von `.dll.lib` und `.dll.exp` wurde von Cargo als `linker_messages`-Warnung weitergereicht, enthielt aber keinen Code- oder Linkfehler. |
| Installer | bestanden | `src-tauri/target/release/bundle/nsis/LDTG_0.1.3_x64-setup.exe`, 3.252.941 Bytes, SHA-256 `33FB2BA4CE9DEDB5E98DCE964A14A7AC317E45AF90FE8C01B8AAE6E1382022AD`. |

## Ausdrücklich ohne weiteren Befund

- Kein bestätigter Traversal-, ADS-, Symlink-/Reparse-, Hidden-/Managed- oder kanonischer Escape-Pfad bei Downloads.
- Kein bestätigter LAN-Weg zum Überschreiben, Löschen, Umbenennen, Verschieben oder Ausführen bestehender Uploadinhalte.
- Kein bestätigter Bypass der vollständigen kanonischen Trennung von Download- und Uploadwurzel.
- Kein bestätigter Bypass der Windows-Autostart-, PATH- oder PowerShell-Modul-Ausschlüsse für Uploadziele.
- Keine HTTP-Route zu Desktopsteuerung, Einstellungen, Firewallkonfiguration oder Diagnoseexport.
- Keine Ausgabe von Zugangscode, Sitzungs-/CSRF-Token, Dateilisten oder Dateiinhalten in der untersuchten Diagnose-/Logginglogik.
- Kein bestätigter Host-, Origin-, Subnetz-, Sitzungs-IP-, CSRF-, Rollen- oder Uploadbesitz-Bypass.
- Kein bestätigter PowerShell-Pfad- oder Argument-Injection-Pfad; System-PowerShell, EncodedCommand und Readback bleiben wirksam.
- Keine weitere Desktop-/Mobile-Abweichung mit Sicherheitswirkung gefunden.
- Keine zusätzliche Accessibility-Barriere mit funktionaler Auswirkung in den untersuchten Hauptflüssen gefunden; Formlabels, Buttons und Fortschrittsanzeigen besitzen nutzbare semantische Zuordnung.
- Beide Produktions-Webbuilds, Rust-Release und Installer sind reproduzierbar erfolgreich gebaut worden.

## Priorisierte Empfehlung

1. RA-ERR-06 zuerst beheben: Ein atomarer Dienstzustandsautomat verhindert unerwartet weiterlaufende Listener und Folgeraces bei Stop, Quit, Save und Status.
2. RA-ERR-08 absichern, bevor ein Installer breit verteilt wird: Nutzdaten dürfen niemals von einem pauschalen Uninstall-`RMDir /r` erfasst werden.
3. RA-SEC-09 und RA-SEC-12 gemeinsam härten: frühe Uploadbesitzprüfung sowie faire Byte-Reservierungsquoten.
4. RA-SEC-10 und RA-SEC-13 mit einer cancellation-sicheren Ownership von Blocking-Dateisystemarbeit beheben.
5. RA-SEC-11 um eine Per-IP-Downloadquote und absolute Transferlease ergänzen.
6. RA-ERR-07 durch einen expliziten Mobile-Uploadzustandsautomaten korrigieren.
7. Danach die niedrigen Admission-/Cursor-/Headergrenzen RA-SEC-14 bis RA-SEC-16 sowie RA-ERR-09 und RA-DOC-01 schließen.

Nach jeder Behebung sollten die hier geforderten Konkurrenz-, Pause-, Ressourcen-, Timeout- und Uninstaller-Regressionstests ergänzt und anschließend dieselbe lokale Test-/Build-Matrix erneut ausgeführt werden.
