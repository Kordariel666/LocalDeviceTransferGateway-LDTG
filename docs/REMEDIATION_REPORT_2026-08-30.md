# Remediation- und Verifikationsbericht vom 30. August 2026

**Ergebnis:** `fixed` für SEC-01 bis SEC-08 und ERR-01 bis ERR-05. Die drei dokumentierten Vertragsabweichungen wurden korrigiert. Kein bestätigter Befund blieb technisch offen.

**Ausgangsbasis:** `CODE_AND_SECURITY_AUDIT_2026-08-30.md`, `THREAT_MODEL_2026-08-30.md` und `SECURITY.md`. Das Audit und das zugehörige Threat Model bleiben als historische Beschreibung des verwundbaren Ausgangszustands erhalten. Der umgesetzte, abhängigkeitssensitive Plan steht in `FIX_PLAN_2026-08-30.md`.

## Umgesetzte Security-Fixes

| ID | Geschlossene Grenze | Umsetzung | Gezielter Nachweis |
|---|---|---|---|
| SEC-01 | Nicht angemeldeter LAN-Client → PATCH-Body/Socket-/Taskressourcen | PATCH-Sitzung und CSRF werden in Middleware vor Bodyextraktion geprüft. Zusätzlich gelten 96 Verbindungen global/12 pro IP, 64 bearbeitete Requests global/8 pro IP, 30 s I/O-Idle- und 120 s Requestlimit. | `rejects_patch_before_polling_an_unauthenticated_body`, globale/IP-Requesttests, `connection_limiter_enforces_global_and_address_caps`, `idle_connection_io_times_out`; der normale Uploadtest bleibt grün. |
| SEC-02 | Verteilte Quell-IPs → langlebiger kleiner Zugangscode → vollständige Sitzung | Achtstelliger `OsRng`-Dezimalcode, zehn Fehlversuche pro IP, 50 dienstweite Fehlversuche pro Fünf-Minuten-Fenster, globaler Block und sofortiger Codewechsel am Schwellenwert. | `distributed_failures_rotate_and_block_the_service_code`, `access_codes_have_eight_decimal_digits`, paralleler IP-Lock-Test und erfolgreicher Cookie-Login. |
| SEC-03 | Wiederholte erfolgreiche Logins → LRU-Verdrängung fremder Sitzungen/Transfers | Maximal 128 Sitzungen global und 4 pro IP. Eine weitere Anmeldung wird abgewiesen; es gibt keine LRU-Entfernung und keinen Transferabbruch fremder Sitzungen. | `session_limits_reject_new_sessions_without_evicting_existing_ones` hält die erste Sitzung und ihren Download trotz voller Kapazität aktiv. |
| SEC-04 | Gleichbleibende Adapteradresse/-maske bei neuem Windows-Profil → unbestätigtes Netz | Der Monitor vergleicht `id`, Netzmaske und vollständige bestätigte `network_id`; jede Abweichung stoppt den Dienst und verwirft danach den flüchtigen Dienstzustand. | `profile_change_is_not_the_same_confirmed_network` reproduziert gleiche IP/Maske mit anderer Profilidentität. |
| SEC-05 | Operator wählt Windows-Autoloadziel → Client veröffentlicht ausführbaren Inhalt | Kanonische `%APPDATA%`-Startup- und `%PROGRAMDATA%`-Common-Startup-Bäume werden neben PATH-/Arbeits-/Programmpfad- und PowerShell-Modulzielen als Uploadwurzel gesperrt. | `startup_roots_and_descendants_are_blocked_upload_targets`; normale disjunkte Uploadziele und Veröffentlichungen bleiben grün. |
| SEC-06 | Gleiche/verschachtelte/aliasierte Wurzeln → Upload-Eingang über Download lesbar | `prepare_roots` vergleicht beide kanonischen Wurzeln und weist Gleichheit sowie beide Verschachtelungsrichtungen ab. | `rejects_equal_and_nested_share_roots`, `canonical_aliases_are_treated_as_the_same_share`. |
| SEC-07 | Angemeldeter Client → wiederholter Vollscan eines großen Verzeichnisses → Tokio-/CPU-/Speicherdruck | Ein opaker, sitzungs-/pfad-/filtergebundener Servercursor hält `ReadDir` zwischen Seiten. Pro Seite werden höchstens 256 Roh- und 200 sichtbare Einträge bearbeitet; maximal 4 blockierende Listings laufen parallel, Cursor haben TTL und Kapazitätslimit. | `directory_page_stops_after_a_bounded_number_of_hidden_entries`, `directory_cursor_is_bound_to_its_session`, `limits_parallel_directory_work`. |
| SEC-08 | Viele Quelladressen → lebenslang wachsende Attempt-Map | Jeder Record erhält `last_seen` und zehn Minuten TTL; die Map ist auf 1.024 Records begrenzt. Neue Adressen oberhalb der Kapazität erhöhen weiterhin den dienstweiten Fehlversuchszähler und schwächen SEC-02 daher nicht. | `auth_attempt_records_expire_and_remain_capacity_bounded`. |

## Umgesetzte Fehlerbehebungen

| ID | Umsetzung | Gezielter Nachweis |
|---|---|---|
| ERR-01 | Das metadatenbasierte `localStorage`-Resume wurde entfernt. Fortsetzung ist nur innerhalb derselben offenen Auswahl an dasselbe in-memory `File`-Objekt und dessen Upload-ID gebunden; jede neue Auswahl legt einen neuen Upload an. | Mobile-Test `übernimmt bei gleichen Metadaten niemals eine Upload-ID einer anderen Dateiauswahl`. |
| ERR-02 | Navigation verwendet monotone Request-IDs und `AbortController`; nur die jüngste Antwort darf Pfad, Einträge oder Fehler ändern. | Mobile-Test `verwirft eine ältere Ordnerantwort nach neuerer Navigation`. |
| ERR-03 | Serve-Fehler setzen vor Taskende einen sicheren Stopgrund; beim Einsammeln wird auch ein Join-/Panicfehler in den sichtbaren Fehlerstatus übernommen. | `serve_error_sets_a_visible_stop_reason`, `join_failure_sets_a_visible_stop_reason`. |
| ERR-04 | Namen werden nach UTF-16-Einheiten und effektivem erweiterten Windows-Gesamtpfadbudget begrenzt. Kollisionssuffix und Erweiterung werden gemeinsam in die Komponentenbegrenzung eingepasst; die Prüfung erfolgt bei Uploadanlage. | `limits_non_bmp_names_by_utf16_units`, `collision_suffix_stays_inside_utf16_component_limit`, `rejects_root_without_safe_total_path_budget`. |
| ERR-05 | Nur eine fehlende Datei lädt Defaults ohne Warnung. Read-/Parsefehler liefern fail-safe Defaults und eine persistente Desktopwarnung. Vor dem ersten Ersatz wird create-new eine `settings.recovery-N.json` angelegt; scheitert die Sicherung, wird nicht überschrieben. | Settings-Tests für missing/corrupt/unreadable/backup sowie Desktop-Test `zeigt beschädigte Einstellungen als persistente Warnung`. |

## Korrigierte Dokumentationsabweichungen

1. `ARCHITECTURE.md` beschreibt jetzt feste globale/IP-Sitzungskapazitäten und explizite Ablehnung statt LRU-Verdrängung.
2. `ARCHITECTURE.md` beschreibt nicht leere Blöcke bis 8 MiB am exakt bestätigten Offset statt zwingender 8-MiB-Schritte.
3. `README.md` bindet die Write-only-Zusage ausdrücklich an die neue Backend-Sperre für gleiche/verschachtelte kanonische Wurzeln.

Zusätzlich dokumentieren `README.md`, `ARCHITECTURE.md`, `API.md` und `TESTPLAN.md` achtstelligen Code, dienstweiten Authschutz, Verbindungs-/Request-/Listinggrenzen, Cursorbindung, Startup-Sperren, UTF-16-Namen, in-memory Resume und die neuen Regressionen.

## Geänderte Dateien

- Security-/Servicegrenzen: `src-tauri/src/service/api.rs`, `src-tauri/src/service/state.rs`, `src-tauri/src/service/mod.rs`.
- Domänen-/Laufzeitgrenzen: `src-tauri/src/domain/network.rs`, `src-tauri/src/domain/shares.rs`, `src-tauri/src/domain/settings.rs`, `src-tauri/src/domain/types.rs`, `src-tauri/src/lib.rs`.
- Mobile: `apps/mobile/src/App.tsx`, `apps/mobile/src/App.test.tsx`, `apps/mobile/src/i18n.ts`.
- Desktop/Verträge: `apps/desktop/src/DesktopApp.tsx`, `apps/desktop/src/App.test.tsx`, `packages/shared/src/index.ts`.
- Dokumentation: `README.md`, `docs/ARCHITECTURE.md`, `docs/API.md`, `docs/TESTPLAN.md`, `docs/FIX_PLAN_2026-08-30.md`, dieser Bericht sowie historische Verweise in Audit/Threat Model.

## Vollständige Abnahme

| Prüfung | Ergebnis |
|---|---|
| `pnpm typecheck` | bestanden; Desktop und Mobile |
| `pnpm test` | bestanden; Desktop 7/7, Mobile 5/5 |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/test-rust.ps1` | bestanden; 53/53 |
| `cargo fmt --all -- --check` | bestanden |
| `cargo clippy --all-targets --offline --locked -- -D warnings` | bestanden |
| `pnpm build:web` | bestanden; Mobile- und Desktop-Produktionsassets |
| `pnpm build` | bestanden; Release-Binary und per-user NSIS-Installer |

Die ersten Vitest-/Vite-Läufe innerhalb der eingeschränkten Dateisystem-Sandbox konnten wie bereits im Audit den esbuild-Elternprozess nicht lesen. Dieselben unveränderten Kommandos liefen außerhalb dieser Einschränkung erfolgreich; dies ist kein Produktfehler. Cargo gab ausschließlich den bekannten Hinweis aus, den Benutzerprofilpfad nicht kanonisieren zu können. Der MSVC-Linker meldete beim Release-Build informativ die Erzeugung der Importbibliothek; der Build und das Bundle waren erfolgreich.

Erzeugte Hauptartefakte:

- `apps/mobile/dist/index.html`
- `apps/desktop/dist/index.html`
- `src-tauri/target/release/dmdc.exe`
- `src-tauri/target/release/bundle/nsis/DMDC_0.1.3_x64-setup.exe`

## Read-only Security-Verifikation gegen die ursprünglichen Angriffswege

```json
{
  "results": [
    { "id": "SEC-01", "status": "fixed", "evidence": "PATCH wird vor Bodypoll authentifiziert; globale/IP-Verbindungs- und Requestlimits sowie Idle-/Requesttimeouts sind aktiv. Panik-Body-, Kapazitäts-, Timeout- und legitimer Uploadtest bestanden." },
    { "id": "SEC-02", "status": "fixed", "evidence": "Achtstelliger OsRng-Code plus dienstweiter 50er-Fehlversuchshaushalt; verteilter 49+1-Test löst Block und Codewechsel aus, erfolgreicher Login bleibt möglich." },
    { "id": "SEC-03", "status": "fixed", "evidence": "128 global/4 pro IP; Überkapazität liefert Fehler statt LRU. Test beweist, dass erste Sitzung und Download aktiv bleiben." },
    { "id": "SEC-04", "status": "fixed", "evidence": "Monitor verwendet vollständige network_id; Test mit identischer Adresse/Maske und anderem Profil wird als Wechsel erkannt." },
    { "id": "SEC-05", "status": "fixed", "evidence": "Kanonische Benutzer-/Common-Startupbäume sowie bestehende Code-Ladepfade werden in der gemeinsamen Uploadwurzelprüfung abgewiesen; Containment-Test bestanden." },
    { "id": "SEC-06", "status": "fixed", "evidence": "Gemeinsame Backendprüfung weist gleiche und beide verschachtelten kanonischen Wurzeln ab; Gleichheits-, Verschachtelungs-, Alias- und disjunkter Kontrolltest bestanden." },
    { "id": "SEC-07", "status": "fixed", "evidence": "Persistenter gebundener Cursor begrenzt jede Seite auf 256 Prüfungen/200 Ergebnisse und vier parallele Blocking-Jobs; Bounded-Work- und Fremdsitzungstests bestanden." },
    { "id": "SEC-08", "status": "fixed", "evidence": "Attempt-Records besitzen TTL und 1024er-Cap; Kapazitätstest beweist außerdem fortgesetzte Erhöhung des globalen Schutzbudgets." }
  ]
}
```

## Legitimes Verhalten und verbleibende Release-Checks

Erfolgreiche Anmeldung, HttpOnly-/SameSite-Cookie, CSRF, Host/Origin/Subnetz, Range-Downloads, Pause/Retry innerhalb derselben Dateiauswahl, exakte Uploadoffsets, Zero-Byte-Uploads, No-Replace-Publikation, disjunkte Freigaben und normale versionierte Einstellungen bleiben durch Tests belegt. Keine bestehende Authentifizierungs-, Pfad-, Rollen-, Diagnose- oder Desktop/LAN-Grenze wurde gelockert.

Es bleibt kein offener Codebefund. Die in `TESTPLAN.md` vorgesehenen realen Windows-10/11-Installations-, UAC-, Firewall-, Browser- und physischen Netzwerkwechseltests bleiben reguläre manuelle Release-Abnahme; für die hier behobenen Logikpfade wurden deterministische fokussierte Tests verwendet.
