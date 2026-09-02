# Security-Fix-Bericht vom 1. September 2026

## Ergebnis

Status: **fixed**

Scan-ID: `76b0cb9a-c8e4-4448-9e9e-7ad83230308c`

Die vier validierten Repository-Befunde und drei angrenzenden Funktionsfehler sind behoben. Eine frische Nur-Lese-Grenzprüfung vor der Änderung und eine unabhängige Bypass-/Regressionsprüfung nach den fokussierten Tests wurden durchgeführt. Die zweite Prüfung fand zwei zusätzliche Race-Conditions in Gesamtwiderruf/Cursoreinfügung und eine Shutdown-Prioritätslücke; auch diese sind behoben und mit Konkurrenztests abgesichert.

## Behobene Pfade und wiederhergestellte Invarianten

1. **Download-Dateisystemarbeit:** GET und HEAD verwenden denselben begrenzten Blocking-Pfad. `safe_existing`, kanonische Pfadprüfung und Metadatenzugriff laufen unter 4 globalen und 2 IP-bezogenen Slots. Das Permit gehört dem Blocking-Job und wird bei HTTP-Abbruch nicht vorzeitig freigegeben. Die Upload-Speicherprüfung nutzt dieselbe Grenze.
2. **Sitzungserschöpfung:** Sitzungen besitzen monotone Fristen von 6 Stunden 15 Minuten Inaktivität und 24 Stunden absolut. Ablaufprüfung, Entfernung und neue Kapazitätsaufnahme erfolgen atomar. Zugehörige Downloads, Uploads und Cursor werden anschließend sitzungsgenau bereinigt; frische Sitzungen werden nicht verdrängt.
3. **Cursor-Fairness und Retry:** Aktive Cursor sind auf 64 global, 8 pro Client-IP und 4 pro Sitzung begrenzt. Abgewiesene Seitenarbeit verlängert keine TTL. Eine gecachte Seite plus Sequenz macht verlorene Zwischen- und Finalantworten wiederholbar, ohne den Iterator erneut vorzuschieben.
4. **Mobile Filterbindung:** Eine Folgeseite verwendet den vom Server bestätigten angewendeten Filter und die zugehörige Seitensequenz, nicht einen inzwischen bearbeiteten Suchentwurf. Spätere 401-Antworten führen zurück zur Anmeldung.
5. **Upload-Namensorakel:** Jeder veröffentlichte Zielname erhält unabhängig vom vorhandenen Inbox-Inhalt einen kryptografisch zufälligen zwölfstelligen Hex-Zusatz. UTF-16-Komponenten- und Gesamtpfadbudgets sowie die atomare No-Replace-Veröffentlichung bleiben erhalten.
6. **Netzwerkmonitor und Stop:** Netzwerkprüfung und Startauswahl laufen außerhalb der Async-Worker, höchstens eine Monitorprüfung gleichzeitig. Accept und Shutdown bleiben reaktionsfähig; Shutdown gewinnt bei gleichzeitig fertigem Fehlergebnis. PowerShell-Prozesse besitzen ein hartes Zeitlimit von 15 Sekunden und werden danach beendet.
7. **Widerrufsrennen:** Gesamtwiderruf bereinigt nur die beim linearen Widerruf erfassten Sitzungs-IDs. Cursoreinfügung ist mit der Sessionmap geordnet und wird nach blockierender Seitenarbeit erneut auf aktive Sitzung geprüft.

## Wesentliche Dateien

- `src-tauri/src/service/state.rs`: Sitzungsfristen, atomare Bereinigung, Dateisystem- und Cursorlimits, Replayzustand, Konkurrenztests.
- `src-tauri/src/service/api.rs`: gemeinsame GET/HEAD-Prüfung, begrenzte Speicherprüfung, Sequenz-/Replay-Protokoll, Post-Work-Sitzungsprüfung.
- `src-tauri/src/service/mod.rs`: nichtblockierender Netzwerkmonitor, Shutdown-Priorität und begrenzter Stop.
- `src-tauri/src/platform/mod.rs`: hartes PowerShell-Zeitlimit.
- `src-tauri/src/domain/shares.rs`: immer opake Uploadzielnamen bei atomarem No-Replace.
- `src-tauri/src/lib.rs`: ausgelagerte Netzwerk-, Freigabe- und Diagnosedateisystemarbeit.
- `packages/shared/src/index.ts`, `apps/mobile/src/App.tsx`, `apps/mobile/src/App.test.tsx`: Cursorvertrag, angewendeter Filter und Mobile-Regressionstest.
- `README.md`, `docs/API.md`, `docs/ARCHITECTURE.md`, `docs/TESTPLAN.md`: aktualisierte Laufzeit- und Sicherheitsverträge.

## Verifikation in Ausführungsreihenfolge

1. `pnpm typecheck` — erfolgreich.
2. `pnpm test` — erfolgreich: Desktop 7/7, Mobile 12/12.
3. `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/test-rust.ps1` — erfolgreich: Rust 82/82.
4. `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` — erfolgreich.
5. `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features --offline --locked -- -D warnings` — erfolgreich.
6. `pnpm build:web` — erfolgreich.
7. `pnpm build` — erfolgreich; Release-EXE und NSIS-Installer wurden erzeugt.

Der Release-Linker meldete ausschließlich die normale Erzeugung der Windows-Importbibliothek als `linker_messages`-Warnung. Es gab keinen Compiler-, Test-, Lint- oder Bundlefehler.

## Nachweis der Nicht-Reproduzierbarkeit

- Abgebrochene Blocking-Waiter behalten Dateisystem- und Listing-Permits bis zum tatsächlichen Jobende.
- Abgelaufene Sitzungskapazität wird während neuer Anmeldung zurückgewonnen; zugehörige Downloads erhalten ein Abbruchsignal.
- Gesamtwiderruf entfernt keinen danach neu angelegten Cursor.
- Gleichzeitiger Einzelwiderruf und Cursoreinfügung hinterlassen keinen Cursor.
- Dieselbe Cursor-/Seitensequenz liefert sowohl auf einer Zwischen- als auch auf der letzten Seite identische Daten erneut.
- Mobile Pagination sendet nach Bearbeitung des Suchfelds weiterhin den zuvor angewendeten Filter.
- Freie und bereits belegte Basisnamen führen beide ausschließlich zu opaken Zielnamen; ein vorhandenes Ziel wird nicht überschrieben.
- Ein künstlich blockierter Monitorjob verzögert den Dienststopp nicht; Shutdown gewinnt gegenüber einem gleichzeitig fertigen negativen Netzwerkresultat.
- Ein künstlich schlafender PowerShell-Prozess wird durch das Zeitlimit beendet.

## Bewusst unverändertes legitimes Verhalten

- Downloadfreigaben bleiben read-only, inklusive Range- und Attachment-Semantik.
- Uploads bleiben add-only; vorhandene Dateien werden weder aufgelistet noch überschrieben.
- Frische Sitzungen werden bei Kapazitätsdruck weiterhin nicht per LRU verdrängt.
- Die HTTP-/LAN-Vertrauensgrenze aus `SECURITY.md` bleibt unverändert; Transportverschlüsselung ist nicht Bestandteil von v1.

## Restunsicherheit

Die automatisierte Matrix belegt Codepfade, Konkurrenzinvarianten und einen echten Windows-Installer-Build. Physische Langzeittests mit absichtlich stockenden UNC-Freigaben, realem WLAN-Profilwechsel sowie iOS-/Android-Browsern bleiben Bestandteil der manuellen Abnahme in `docs/TESTPLAN.md`. Es wurde bewusst kein netzwerkabhängiger Paket-Audit ausgeführt; Abhängigkeitsmetadaten außerhalb des Repositories waren nicht Teil dieses Fixauftrags.
