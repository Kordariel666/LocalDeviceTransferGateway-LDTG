# Vollständiger Code- und Sicherheits-Audit

**Projekt:** Local Device Transfer Gateway (LDTG) v0.1.3
**Stichtag:** 30. August 2026  
**Status:** Analyse abgeschlossen; keine Behebung durchgeführt  
**Zweck:** Belastbare Befundbasis für einen späteren, gesonderten Fix-Plan

> **Historischer Ausgangsstand:** Die in diesem Audit beschriebenen Befunde wurden anschließend umgesetzt und verifiziert. Aktueller Status, Prüfkommandos und Nachweise stehen in `REMEDIATION_REPORT_2026-08-30.md`; der Audittext bleibt als unveränderte Befundbasis erhalten.

## 1. Ergebnis in Kürze

Der überprüfte Quellstand baut und testet erfolgreich, enthält aber acht bestätigte Sicherheitsprobleme und fünf bestätigte Funktions- beziehungsweise Zuverlässigkeitsfehler, die von den vorhandenen Tests nicht erfasst werden.

| Kategorie | Kritisch | Hoch | Mittel | Niedrig |
|---|---:|---:|---:|---:|
| Sicherheitsbefunde | 0 | 0 | 3 | 5 |
| Funktions-/Zuverlässigkeitsfehler | 0 | 1 | 3 | 1 |

Die Sicherheitsbewertung berücksichtigt nicht nur den maximalen Schaden, sondern auch die im Quellcode belegten Voraussetzungen. Darum sind beispielsweise die mögliche Ausführung eines Uploads aus einem Windows-Autostartordner und der Wechsel in ein anderes Netzwerk trotz hohen möglichen Schadens als **niedrig** eingestuft: Beide Wege benötigen eine besondere Laufzeitkonfiguration beziehungsweise einen engen Netzwerkwechsel.

Alle Produktdateien blieben unverändert. Neu angelegt wurden ausschließlich diese Audit-Unterlagen unter `docs`.

## 2. Umfang und Methode

Geprüft wurden die aktuellen, nicht versionierten Dateien des Arbeitsverzeichnisses:

- React-/TypeScript-Code der Desktop- und Mobiloberfläche einschließlich Tests und Styles;
- gemeinsames TypeScript-Paket;
- Rust-/Tauri-/Axum-Backend einschließlich Plattform-, Netzwerk-, Datei- und Sitzungslogik;
- Tauri-Fähigkeiten, Build-, Installer- und Manifestkonfiguration;
- Root-Manifeste, Lockfiles, Skripte, Richtlinien und vorhandene technische Dokumentation;
- sicherheitsrelevante Datenflüsse von LAN-Anfragen bis zu Authentifizierung, Dateisystem, Sitzungen, Windows-Netzwerkprofilen und Firewallbefehlen.

Nicht als Produktquellcode geprüft wurden generierte Build-Ausgaben (`dist`, `src-tauri/target`), vendete Abhängigkeitsbäume (`node_modules`), generierte Tauri-Schemata sowie binäre Icons und QA-Screenshots. Manifeste und Lockfiles wurden als Kontext ausgewertet.

Die Sicherheitsanalyse erfolgte als vollständiger Standard-Scan mit:

- unabhängiger Baseline-Prüfung;
- unabhängiger Architektur- und Bedrohungsmodellierung;
- fokussierter Gegenprüfung der Freigaben, Authentifizierung, Netzwerkidentität und Ressourcenbegrenzung;
- anschließender, einmaliger Validierung jedes eindeutigen Befunds am tatsächlichen Kontroll- und Datenfluss;
- expliziter Prüfung von Gegenmaßnahmen und Gegenbelegen, damit bloße Suchtreffer nicht als Schwachstellen erscheinen.

Wichtig zum Repository-Zustand: Git meldet derzeit **keine verfolgten Dateien**; alle Projektdateien erscheinen als untracked. Dieser Audit beschreibt deshalb exakt den vorliegenden Verzeichnis-Snapshot, nicht einen Commit oder eine Branch-Differenz.

## 3. Ausgeführte Prüfungen

| Prüfung | Ergebnis |
|---|---|
| Desktop-Typecheck | bestanden |
| Mobile-Typecheck | bestanden |
| Desktop-Vitest | 6/6 bestanden |
| Mobile-Vitest | 3/3 bestanden |
| Rust-Tests über `scripts/test-rust.ps1` | 28/28 bestanden |
| `cargo fmt --all -- --check` | bestanden |
| `cargo clippy --all-targets --offline --locked -- -D warnings` | bestanden |
| Produktions-Build Mobile | bestanden |
| Produktions-Build Desktop | bestanden |

Die anfängliche Ausführung über die normale Shell fand Node.js nicht im `PATH`. Mit der gebündelten Workspace-Laufzeit liefen Typechecks, Tests und Builds erfolgreich. Die Frontendtests mussten außerhalb der eingeschränkten Dateisystem-Sandbox gestartet werden, weil `esbuild` einen Elternprozess außerhalb des Sandboxpfads benötigt. Beides sind Ausführungsumgebungs-Hinweise und keine LDTG-Codefehler.

Es wurde kein Online-Advisory-Dienst verwendet. Aussagen zu aktuell veröffentlichten CVEs in npm- oder Cargo-Abhängigkeiten sind deshalb ausdrücklich **nicht** Teil dieses Audits.

## 4. Bestätigte Sicherheitsbefunde

### SEC-01 – Uploadblöcke werden vor der Authentifizierung gepuffert

**Einstufung:** Mittel · **CWE-770** · **Konfidenz:** Hoch · **Status:** Nicht behoben

Der PATCH-Handler verwendet einen `Bytes`-Body-Extractor. Axum liest damit den vollständigen, ungefähr auf 8 MiB begrenzten Body, bevor der Handler `authorized()` und die CSRF-Prüfung ausführt. Der gemeinsame Request-Guard prüft vorher Subnetz, Host und Origin, aber keine Sitzung. Gleichzeitig wird `axum::serve` ohne globale Verbindungs-/Requestbegrenzung, Load-Shedding oder Lese-/Idle-Timeout gestartet.

**Belege:**

- `src-tauri/src/service/api.rs:81-104` – nur Body-Limit und Request-Guard;
- `src-tauri/src/service/api.rs:1171-1192` – `Bytes`-Extraktion vor `authorized()`;
- `src-tauri/src/service/mod.rs:86-94` – unmittelbarer `axum::serve`-Start ohne zusätzliche Ressourcenlayer.

**Angriffsweg:** Ein nicht angemeldeter nativer Client im ausgewählten Subnetz sendet viele parallele PATCH-Anfragen mit korrektem Host/Origin und ungültiger Sitzung. Jeder Request kann Speicher, Task- und Socketkapazität binden, bevor er abgewiesen wird.

**Auswirkung:** Speicher- und Pagingdruck bis hin zum Prozessabbruch; Störung aller Übertragungen.

**Wirksame Gegenkontrollen:** ungefähr 8 MiB pro Request; Subnetz-, Host- und Origin-Prüfung. Die späteren Uploadlimits von 64 global beziehungsweise 4 pro IP greifen hier noch nicht.

**Späteres Abnahmekriterium:** Eine nicht authentifizierte große PATCH-Anfrage muss abgewiesen werden, bevor ihr Body gelesen wird; parallele und langsame Requests müssen durch globale und IP-bezogene Grenzen sowie Timeouts begrenzt sein.

### SEC-02 – Verteiltes Raten des sechsstelligen Zugangscodes

**Einstufung:** Mittel · **CWE-307** · **Konfidenz:** Hoch · **Status:** Nicht behoben

Der Code hat genau eine Million mögliche Werte. Falsche Versuche werden ausschließlich pro Quell-IP gezählt; jede IP erhält zehn Versuche und nach fünf Minuten erneut zehn. Es existieren weder ein dienstweiter Fehlversuchshaushalt noch eine progressive globale Verzögerung oder automatische Codeablaufzeit.

**Belege:**

- `src-tauri/src/service/state.rs:87-108` – sechsstelliger Code und IP-basierte Map;
- `src-tauri/src/service/api.rs:304-375` – zehn Versuche und fünf Minuten Block nur für die jeweilige IP.

**Angriffsweg:** Ein nicht angemeldeter LAN-Client verwendet mehrere Geräte oder gültige IPv4-Aliase innerhalb des gewählten Subnetzes. Auf einem /24 wären theoretisch bis zu 30.480 Versuche pro Stunde gegen denselben, standardmäßig langlebigen Code möglich.

**Auswirkung:** Ein Treffer erstellt sofort eine vollständige Sitzung mit allen aktivierten Download- und Uploadrollen.

**Wirksame Gegenkontrollen:** `OsRng`, konstante Vergleichszeit, Serialisierung pro IP, Subnetz-/Host-/Origin-Grenzen. Der Weg benötigt viele gültige Quelladressen oder eine lange Dienstlaufzeit.

**Späteres Abnahmekriterium:** Verteilte Fehlversuche müssen einen gemeinsamen Dienstgrenzwert auslösen; der Code muss ausreichend entropiereich sein und nach Schwellenwert, Zeit oder Kopplung erneuert werden.

### SEC-03 – Ein Client kann den Sitzungspool verdrängen

**Einstufung:** Mittel · **CWE-770** · **Konfidenz:** Hoch · **Status:** Nicht behoben

Jede erfolgreiche Codeanmeldung erzeugt auch von derselben IP eine neue Sitzung. Erfolgreiche Anmeldungen sind nicht begrenzt. Ab 128 Sitzungen entfernt `create_session()` die am längsten inaktive Sitzung und bricht deren Downloads und unvollständige Uploads ab.

**Belege:**

- `src-tauri/src/service/api.rs:304-395` – unbegrenzt wiederholbare erfolgreiche Anmeldung;
- `src-tauri/src/service/state.rs:295-328` – globale LRU-Verdrängung;
- `src-tauri/src/service/state.rs:372-405` – Abbruch der Übertragungen der verdrängten Sitzung.

**Angriffsweg:** Ein bereits codeberechtigter Client führt 128 oder mehr erfolgreiche Anmeldungen aus und wiederholt dies bei Bedarf.

**Auswirkung:** Andere Geräte werden abgemeldet; Downloads werden beendet und unvollständiger Uploadfortschritt wird entfernt.

**Wirksame Gegenkontrollen:** Der globale Grenzwert verhindert unbegrenztes Map-Wachstum; Tokens bleiben zufällig und IP-gebunden. Der Angreifer gewinnt keine fremden Datenrechte, sondern verursacht Verfügbarkeitsverlust.

**Späteres Abnahmekriterium:** Ein Client darf den globalen Pool nicht füllen; Kapazitätsüberschreitung darf bestehende fremde Sitzungen oder Übertragungen nicht automatisch zerstören.

### SEC-04 – Netzwerkprofilwechsel wird nicht vollständig überwacht

**Einstufung:** Niedrig · **CWE-284** · **Konfidenz:** Hoch · **Status:** Nicht behoben

Beim Start wird eine `network_id` bestätigt, die Schnittstelle, Windows-Profilname und Netzadresse/Präfix enthält. Der 15-Sekunden-Monitor vergleicht später jedoch nur die `id` aus Schnittstellenname und IPv4-Adresse sowie die Netzmaske.

**Belege:**

- `src-tauri/src/domain/network.rs:121-135` – unterschiedliche Definitionen von `id` und `network_id`;
- `src-tauri/src/lib.rs:257-285` – Startfreigabe anhand der vollständigen `network_id`;
- `src-tauri/src/service/mod.rs:67-81` – Laufzeitprüfung nur anhand `id` und Netzmaske.

**Angriffsweg:** Derselbe Adapter wechselt zwischen zwei Windows-Netzwerkprofilen, erhält aber zwischen zwei Prüfpunkten dieselbe IPv4-Adresse und Netzmaske. Listener, Code und Sitzungen bleiben bestehen, obwohl das neue Profil nie bestätigt wurde.

**Auswirkung:** LDTG kann auf einem unbestätigten Netzwerk weiter erreichbar sein.

**Wirksame Gegenkontrollen:** Geänderte Adresse/Netzmaske oder eine beobachtete Trennung stoppen den Dienst; Request-Subnetz, Host und Origin bleiben aktiv. Die identische Wiedervergabe und das verpasste Trennungsfenster sind enge Voraussetzungen.

**Späteres Abnahmekriterium:** Ein Profilwechsel mit unverändertem Schnittstellennamen, IPv4 und Präfix muss den Dienst stoppen und alle Sitzungen verwerfen.

### SEC-05 – Uploadziel kann ein Windows-Autostartordner sein

**Einstufung:** Niedrig · **CWE-434** · **Konfidenz:** Hoch · **Status:** Nicht behoben

Die Uploadwurzel wird über einen freien Ordnerdialog gewählt. Die Backendprüfung sperrt Windows-/Programmverzeichnisse, exakte Programm-/Arbeits-/PATH-Pfade und PowerShell-Modulbäume, aber nicht den beschreibbaren benutzerbezogenen Startup-Ordner unter `%APPDATA%`. Dateiendungen bleiben erhalten; fertige Dateien werden direkt in die konfigurierte Wurzel veröffentlicht.

**Belege:**

- `apps/desktop/src/DesktopApp.tsx:142-170` – freie Verzeichnisauswahl;
- `src-tauri/src/domain/shares.rs:46-90` – endliche Sperrliste;
- `src-tauri/src/domain/shares.rs:208-244` – Dateinamenbereinigung behält Endungen;
- `src-tauri/src/service/api.rs:1364-1401` – direkte Veröffentlichung im Zielordner.

**Angriffsweg:** Der lokale Operator konfiguriert einen Windows-Autostart-/Code-Ladeordner als Uploadziel. Ein angemeldeter LAN-Client lädt dort eine ausführbare Datei, ein Skript oder eine Verknüpfung hoch. Windows verarbeitet den Eintrag bei einer späteren Anmeldung.

**Auswirkung:** Ausführung von Angreifercode im Kontext des angemeldeten Windows-Benutzers und mögliche Persistenz.

**Wirksame Gegenkontrollen:** Authentifizierung, Pfadkanonisierung, keine Traversierung, kein Überschreiben und keine direkte Ausführung durch LDTG. Der unsichere Operatorpfad ist zwingende Voraussetzung.

**Späteres Abnahmekriterium:** Bekannte benutzerbezogene und gemeinsame Windows-Autostart-/Code-Ladepfade sowie kanonische Aliase müssen abgewiesen oder Uploads zunächst in einem anwendungskontrollierten, nicht ausführbaren Eingang isoliert werden.

### SEC-06 – Überlappende Freigaben legen den Upload-Eingang offen

**Einstufung:** Niedrig · **CWE-668** · **Konfidenz:** Hoch · **Status:** Nicht behoben

`prepare_roots()` prüft und kanonisiert Download- und Uploadwurzel unabhängig, vergleicht sie anschließend aber weder auf Gleichheit noch auf Enthaltensein. Liegt die Uploadwurzel gleich oder unterhalb der Downloadwurzel, akzeptiert die Downloadauflösung die vorhandenen Inboxdateien als normalen Downloadinhalt.

**Belege:**

- `src-tauri/src/domain/shares.rs:36-43` – keine Beziehungskontrolle der kanonischen Wurzeln;
- `src-tauri/src/service/api.rs:523-606` und `751-906` – Auflisten und Lesen jedes erlaubten Downloadnachfahren;
- `apps/desktop/src/DesktopApp.tsx:460-463` – nur roher Textvergleich;
- `apps/desktop/src/DesktopApp.tsx:562` – rein informative, nicht blockierende Warnung.

**Angriffsweg:** Beide Rollen sind aktiv und die Uploadwurzel ist gleich der Downloadwurzel oder ein sichtbarer Unterordner davon. Jeder angemeldete Client mit Downloadrolle kann vorhandene Inboxdateien anderer Clients auflisten und lesen. Unterschiedliche Schreibweisen desselben kanonischen Pfades und verschachtelte Pfade umgehen die UI-Warnung.

**Auswirkung:** Vertraulichkeitsverlust für bestehende Upload-Inhalte; Bruch der in `SECURITY.md` und `README.md` zugesagten Write-only-Rolle.

**Wirksame Gegenkontrollen:** `.ldtg`-Teildateien bleiben verborgen; bei disjunkten Wurzeln existiert keine Uploadlistenroute; exakte rohe Gleichheit wird sichtbar gewarnt.

**Späteres Abnahmekriterium:** Gleiche oder überlappende kanonische Wurzeln müssen im Backend abgewiesen werden; Tests müssen Gleichheit, beide Verschachtelungsrichtungen und Pfadaliasfälle abdecken.

### SEC-07 – Verzeichnispaginierung begrenzt die Serverarbeit nicht

**Einstufung:** Niedrig · **CWE-400** · **Konfidenz:** Hoch · **Status:** Nicht behoben

`list_downloads()` führt in einem asynchronen Handler synchrone `std::fs`-Operationen für jeden Eintrag aus, sammelt alle Treffer in einem `Vec`, sortiert die gesamte Menge und wendet erst danach `skip(cursor).take(200)` an. Jede Folgeseite wiederholt die gesamte Arbeit.

**Beleg:** `src-tauri/src/service/api.rs:523-606`.

**Angriffsweg:** Ein angemeldeter Downloadclient fragt wiederholt oder parallel ein sehr großes beziehungsweise langsames Verzeichnis ab.

**Auswirkung:** Blockierte Tokio-Worker, CPU-/Speicherdruck und verlangsamte Authentifizierungs- und Übertragungsoperationen.

**Wirksame Gegenkontrollen:** Anmeldung erforderlich; nur 200 Einträge werden serialisiert. Die Operatorfreigabe muss sehr groß sein, damit der Schaden wesentlich wird.

**Späteres Abnahmekriterium:** Kosten einer Seite müssen unabhängig von der Gesamtzahl begrenzt sein; blockierende Dateisystemarbeit und parallele Listings benötigen eigene Grenzen.

### SEC-08 – Fehlversuchs-Map besitzt weder Ablauf noch Kapazitätsgrenze

**Einstufung:** Niedrig · **CWE-400** · **Konfidenz:** Hoch · **Status:** Nicht behoben

Jede IP mit einem einzigen falschen Code erzeugt einen `AttemptRecord`. Eintragungen unterhalb des Sperrwerts laufen nie ab. Auch abgelaufene Sperren werden nur bei einer weiteren Anfrage derselben IP zurückgesetzt; der zurückgesetzte Datensatz bleibt bestehen. Der periodische Monitor bereinigt nur Uploads.

**Belege:**

- `src-tauri/src/service/state.rs:81-99` – unbegrenzte `HashMap<IpAddr, AttemptRecord>`;
- `src-tauri/src/service/api.rs:334-376` – Einfügen, Zurücksetzen und einzige Entfernung bei erfolgreicher Anmeldung derselben IP;
- `src-tauri/src/service/mod.rs:62-82` – keine Bereinigung der Fehlversuche.

**Angriffsweg:** Viele gültige Quelladressen eines großen privaten Präfixes senden jeweils einen falschen Versuch und halten den Dienst lange aktiv.

**Auswirkung:** Lebenslanges Speicherwachstum des Dienstes; auf typischen /24-Netzen gering, bei großen Präfixen und vielen kontrollierten Adressen potenziell störend.

**Späteres Abnahmekriterium:** Alle Datensätze benötigen TTL und eine harte Kapazitätsgrenze; eine Überlastreaktion darf den dienstweiten Authentifizierungsschutz nicht abschwächen.

## 5. Bestätigte Funktions- und Zuverlässigkeitsfehler

### ERR-01 – Resume kann zwei verschiedene Dateien still zusammensetzen

**Einstufung:** Hoch · **Konfidenz:** Hoch · **Status:** Nicht behoben

Der Browser merkt sich einen Upload nur über `serviceId`, Dateiname, Größe und `lastModified`. Zwei verschiedene Dateien mit identischen Metadaten erhalten denselben Schlüssel. `getOrCreateUpload()` verwendet dann ohne Inhaltsidentität die alte Upload-ID und setzt ab dem vorhandenen Offset fort.

**Belege:**

- `apps/mobile/src/App.tsx:33-35` – kollisionsfähiger `localStorage`-Schlüssel;
- `apps/mobile/src/App.tsx:132-145` – blindes Fortsetzen der gespeicherten Upload-ID;
- `apps/mobile/src/App.tsx:148-177` – neue Dateibytes werden ab altem Offset angehängt;
- `src-tauri/src/service/api.rs:1151-1168` – Status enthält Offset/Größe/Zeit, aber keine Inhaltsidentität.

**Fehlerszenario:** Datei A wird teilweise hochgeladen. Danach wählt der Benutzer Datei B mit gleichem Namen, gleicher Größe und gleichem Änderungszeitpunkt, aber anderem Inhalt. Die fertige Zieldatei besteht aus dem Anfang von A und dem Rest von B und kann trotzdem erfolgreich abgeschlossen gemeldet werden.

**Auswirkung:** Stille Datenkorruption mit falscher Erfolgsmeldung.

**Späteres Abnahmekriterium:** Resume muss an eine belastbare Datei-/Inhaltsidentität gebunden sein; eine Metadatenkollision darf niemals vorhandene Teilbytes übernehmen.

### ERR-02 – Ältere Verzeichnisantwort kann neuere Navigation überschreiben

**Einstufung:** Mittel · **Konfidenz:** Hoch · **Status:** Nicht behoben

`loadDirectory()` besitzt weder Request-ID noch Abbruchcontroller. Mehrere parallele Navigations-, Such- oder Paginierungsanfragen dürfen in beliebiger Reihenfolge enden; jede erfolgreiche Antwort setzt `path` und `directory`.

**Beleg:** `apps/mobile/src/App.tsx:71-84` sowie Aufrufe in `apps/mobile/src/App.tsx:254-267`.

**Fehlerszenario:** Der Benutzer öffnet schnell Ordner A und danach B. Antwort B kommt zuerst, anschließend überschreibt die langsamere Antwort A wieder Pfad und Dateiliste.

**Auswirkung:** Anzeige und Breadcrumbs springen auf veraltete Inhalte; Folgeaktionen können sich auf einen unerwarteten Ordner beziehen.

**Späteres Abnahmekriterium:** Nur die neueste Navigation darf den sichtbaren Zustand ändern; ältere Requests müssen abgebrochen oder anhand einer monotonen Request-ID verworfen werden.

### ERR-03 – Laufzeitfehler des HTTP-Servers erscheinen als sauberes Stoppen

**Einstufung:** Mittel · **Konfidenz:** Hoch · **Status:** Nicht behoben

Ein Fehler von `axum::serve` wird nur protokolliert. Es wird kein `stop_reason` gesetzt. Wenn die UI später den beendeten Join-Handle erkennt, übernimmt sie `None` als `last_error` und erzeugt einen normalen `stopped`-Status.

**Belege:**

- `src-tauri/src/service/mod.rs:86-94` – Fehler nur via `tracing::error!`;
- `src-tauri/src/lib.rs:175-202` – fehlender Grund wird als sauber gestoppt dargestellt.

**Auswirkung:** Der Operator erhält keine sichtbare Fehlerursache; Diagnose und Wiederherstellung werden erschwert.

**Späteres Abnahmekriterium:** Jeder unerwartete Serve-/Join-Fehler muss atomar als Fehlerstatus mit sicherer, sichtbarer Ursache übernommen werden.

### ERR-04 – Windows-Dateinamenlimit wird nach Unicode-Zeichen statt UTF-16 geprüft

**Einstufung:** Mittel · **Konfidenz:** Hoch · **Status:** Nicht behoben

`safe_file_name()` kürzt auf 240 Unicode-Skalare. Windows begrenzt eine Pfadkomponente jedoch nach UTF-16-Codeeinheiten; viele Zeichen außerhalb der BMP benötigen zwei Einheiten. Zusätzlich hängt `unique_target()` bei Kollisionen ` (n)` an einen bereits maximal langen Namen an.

**Belege:** `src-tauri/src/domain/shares.rs:208-268` und abschließende Veröffentlichung in `src-tauri/src/service/api.rs:1364-1388`.

**Fehlerszenario:** Ein Name mit vielen Emoji oder ein 240-Zeichen-Name mit Zielkollision wird vollständig übertragen, scheitert aber erst beim finalen Windows-Move. Wiederholen kann den Namen nicht erfolgreich machen.

**Auswirkung:** Vollständig übertragene Dateien können nicht veröffentlicht werden; unnötiger Datentransfer und irreführende Retry-Situation.

**Späteres Abnahmekriterium:** Der endgültige Name einschließlich Kollisionssuffix muss vor Übertragung beziehungsweise Veröffentlichung innerhalb der effektiven Windows-Komponenten- und Gesamtpfadgrenze liegen.

### ERR-05 – Beschädigte Einstellungen werden still durch Standardwerte ersetzt

**Einstufung:** Niedrig · **Konfidenz:** Hoch · **Status:** Nicht behoben

`settings::load()` behandelt eine fehlende, unlesbare und syntaktisch beschädigte `settings.json` identisch und gibt kommentarlos `AppSettings::default()` zurück.

**Beleg:** `src-tauri/src/domain/settings.rs:74-79`.

**Auswirkung:** Konfigurationsbeschädigung bleibt unsichtbar; Freigaben, Port und vertraute Netzwerke wirken plötzlich zurückgesetzt. Die Defaults deaktivieren Freigaben und sind daher fail-safe, aber die Ursache geht verloren.

**Späteres Abnahmekriterium:** „Datei fehlt“ darf Defaults verwenden; Lese- oder Parsefehler müssen sichtbar diagnostiziert und die beschädigte Datei für eine Wiederherstellung erhalten werden.

## 6. Dokumentationsabweichungen

1. `docs/ARCHITECTURE.md:21` sagt, Sitzungen endeten nur durch Widerruf oder Dienststopp. Tatsächlich verdrängt die 129. Sitzung die am längsten inaktive Sitzung und beendet deren Übertragungen (`src-tauri/src/service/state.rs:295-328`).
2. `docs/ARCHITECTURE.md:26` beschreibt streng aufsteigende 8-MiB-Blöcke. Der Server fordert nur den exakten aktuellen Offset und akzeptiert jeden nicht leeren Block bis maximal 8 MiB (`src-tauri/src/service/api.rs:1171-1240`). `docs/API.md:20` beschreibt dies korrekt als Maximum.
3. `README.md:12-16` garantiert, dass der Upload-Eingang nicht aufgelistet wird. Diese Aussage gilt bei gleicher oder unterhalb der Downloadwurzel liegender Uploadwurzel nicht; siehe SEC-06.

Diese Abweichungen wurden dokumentiert, aber noch nicht in den ursprünglichen Dateien korrigiert.

## 7. Verifizierte Sicherheitskontrollen ohne Befund

Folgende Bereiche wurden bis zum sensiblen Sink verfolgt; dabei wurde kein belegbarer Umgehungsweg gefunden:

- Downloadpfade weisen Parent-/Root-/Prefix-Komponenten, Alternate Data Streams, Symlinks, Reparse-Points, versteckte/Systemeinträge und kanonisches Entkommen zurück.
- Uploadteildateien erhalten serverseitige UUID-Namen in einem markierten `.ldtg`-Ordner; Upload-IDs sind an die Sitzung gebunden.
- Fertige Uploads überschreiben keine vorhandenen Dateien; Veröffentlichung verwendet einen freien Namen und No-Replace-Semantik.
- Sitzungstokens sind zufällig, HttpOnly, SameSite=Strict und an die Client-IP gebunden; Schreiboperationen verlangen zusätzlich CSRF.
- Host, Origin und Quellsubnetz werden zentral geprüft.
- Die LAN-API enthält keine Tauri-/Desktop-Steuerbefehle.
- Tauri-Fähigkeiten erteilen weder generische Shell- noch Dateisystemrechte.
- React rendert kontrollierte Namen als Text; kein `dangerouslySetInnerHTML`, `eval` oder belegbarer XSS-Sink wurde gefunden.
- PowerShell wird aus dem kanonischen Windows-Systemverzeichnis gestartet; dynamische Werte werden für den verwendeten PowerShell-Kontext maskiert und das Ergebnis wird zurückgelesen.
- Diagnostik exportiert keine Dateiliste, Dateiinhalte, Zugangscodes, Tokens, User-Agents oder Sitzungsadressen.
- HTTP ohne `Secure`-Cookie ist laut `SECURITY.md` eine bewusste v1-Grenze für ein bestätigtes vertrauenswürdiges LAN; MITM, Internetfreigabe und untrusted LAN sind ausdrücklich nicht zugesichert und wurden daher nicht als Schwachstelle gemeldet.

## 8. Fehlende Regressionstests

Die vorhandenen 37 automatisierten Tests sind grün, decken die bestätigten Randfälle aber nicht ab. Für eine spätere Behebung fehlen mindestens Tests für:

- Authentifizierung vor Verbrauch eines großen PATCH-Bodys sowie globale Request-/Timeoutgrenzen;
- dienstweite Rate-Limits über mehrere Quell-IPs und Ablauf/Kapazität der Attempt-Map;
- wiederholte erfolgreiche Anmeldung, Poolkapazität und Schutz fremder Sitzungen;
- Netzwerkprofilwechsel bei identischer Adresse und Netzmaske;
- Windows Startup/Common Startup und andere Autoload-Ziele;
- gleiche, verschachtelte und kanonisch aliasierte Freigabewurzeln;
- große Verzeichnisse und begrenzte Arbeit pro Seite;
- Resume-Kollision zweier Dateien mit gleichen Metadaten;
- veraltete parallele Navigationsantworten;
- unerwartetes Ende von `axum::serve`;
- lange UTF-16-Dateinamen und Kollisionssuffixe;
- beschädigte beziehungsweise unlesbare Einstellungen.

Die Pause-/Fortsetzen-Logik besitzt ebenfalls keinen gezielten Race-Test. Aus dem statischen Ablauf allein wurde jedoch kein sicher reproduzierbarer zusätzlicher Fehler bewiesen; sie bleibt deshalb eine Testlücke und kein bestätigter Befund.

## 9. Grenzen und offene Nachweise

- Ein statischer Audit kann nicht garantieren, jeden Fehler zu finden. Die dokumentierten Befunde sind quellgestützt und reproduzierbar ableitbar, aber keine Vollständigkeitsgarantie.
- Windows-Netzwerkprofilwechsel, tatsächliche App-Konfigurations-ACLs und reale Operatorpfade waren im Snapshot nicht materialisiert; die Findings nennen diese Voraussetzungen ausdrücklich.
- Es gab keinen Online-Abgleich gegen aktuelle npm-/Cargo-Advisory-Datenbanken.
- Code Signing, Auto-Update und öffentliche Veröffentlichung sind laut Projektumfang v1 nicht implementiert und wurden nicht als vorhandene Schutzflächen bewertet.
- Die Scan-Vorabprüfung hatte drei statt der empfohlenen sechs nutzbaren Worker-Slots. Baseline, Architekturprüfung und fokussierte Gegenprüfung wurden dennoch vollständig abgeschlossen.
- Die Zugriffskontrolle des Security-Workbench meldete den TAC-Status `unknown` und keine Grants. Geschützte Workbench-Artefakte können deshalb möglicherweise nicht direkt angezeigt werden; diese lokale Dokumentation enthält die vollständigen nutzerrelevanten Ergebnisse.

## 10. Übergabe für einen späteren Fix-Plan

Dieses Dokument ist absichtlich noch **kein** Fix-Plan. Es enthält dafür die benötigten stabilen Eingaben:

- eindeutige Befund-IDs;
- Quellorte und Root Causes;
- Angreifer, Voraussetzungen, Auswirkungen und Gegenkontrollen;
- Abnahmekriterien und fehlende Regressionstests;
- dokumentierte Scope-Grenzen und offene Laufzeitnachweise.

Ein späterer Plan sollte diese IDs verwenden, Abhängigkeiten zwischen den Befunden berücksichtigen und Änderungen erst nach expliziter Beauftragung umsetzen.
