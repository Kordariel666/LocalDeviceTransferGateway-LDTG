# P2-Sicherheits-, Datenschutz- und Supportnachweis

Stand: 3. September 2026  
Basisrevision: `950e4301a61bbef79d4ecca3ed81b646baa356ca`  
Paketstatus: abgeschlossen; `PB-04` vom Owner akzeptiert

Dieser Nachweis schließt keine Veröffentlichung, Lizenzaktivierung, Anmeldung,
kostenpflichtige Maßnahme oder Freigabe von R5.2 beziehungsweise Phase 6 ein.
Der Arbeitsbaum und das GitHub-Repository bleiben privat.

## Sicherheitsscan

Ein vollständiger Standardscan wurde gegen exakt die Basisrevision ausgeführt
und einmal versiegelt:

- Scan-ID: `b7ffcb5e-35a2-4be4-9c7e-cb299c94a59c`
- Umfang: vollständiges Repository, keine Diff-Begrenzung
- Ergebnis: drei validierte Findings, jeweils niedrige Schwere und hohe
  Konfidenz; kein bestätigter mittlerer, hoher oder kritischer Befund
- erzeugte Artefaktarten: Bericht, Findings, Coverage, Manifest und SARIF; die
  temporären lokalen Toolpfade werden nicht als veröffentlichungsfähige
  Repositorydaten übernommen

| Finding/Occurrence | Verletzte Grenze | Sichere Behebung | Regression |
|---|---|---|---|
| `authorization.persisted-network-category` / `occ_4dce2c3871ba4f9aea3fcead` | Eine gespeicherte Netz-ID galt trotz geänderter bestätigter Windows-Profilkategorie weiter als vertraut. | Reiner Match-Prädikat verlangt aufgelöstes Profil, identische ID und identische nicht-`Unbekannt`-Kategorie; Name bleibt nur Anzeige. | `persisted_trust_requires_the_approved_known_category` |
| `lifecycle.firewall-rule-incomplete-cleanup` / `occ_5681f2566ccaf3984f843181` | Der Uninstaller entfernte nur den historischen statt auch den aktuellen Regelnamen und ignorierte Fehler. | Erhöhter Kindprozess entfernt aktuelle und historische Regel im `PersistentStore`, prüft Restregeln und propagiert UAC-/PowerShell-/Prüffehler in Retry/Cancel und `Abort`. AppData wird weiterhin nicht rekursiv gelöscht. | `uninstaller_strictly_removes_current_and_legacy_firewall_rules`, `uninstaller_never_recursively_deletes_application_data` |
| `authentication.global-lockout-peer-aliases` / `occ_c2520143e0dba3154160c7e1` | Ein physischer LAN-Peer konnte mit fünf IPv4-Aliasen allein den globalen 50er-Fehlversuchshaushalt verbrauchen. | Der beim Accept für die Verbindungsfairness ermittelte Peer-Schlüssel wird bis zum Auth-Handler getragen und begrenzt auch die zehn lokalen Fehlversuche. Ohne auflösbaren Nachbareintrag gilt weiterhin die IP. | `one_physical_peer_cannot_consume_the_global_authentication_budget_with_aliases`; bestehende Alias-/Fallbacktests der Verbindungsgrenze |

Das unabhängige Vorab-Grenzreview verlangte zusätzlich fail-closed Verhalten beim
Uninstall, einmalige Neubestätigung alter/geänderter Netzwerkdatensätze und den
Erhalt der dienstweiten Authgrenze. Diese Bedingungen wurden umgesetzt. Das nach
den Tests geforderte frische Fix-Review wird im Abschnitt „Verifikation“
protokolliert.

## Datenschutzabgleich

[`docs/PRIVACY.md`](../../docs/PRIVACY.md) bildet Einstellungen,
Recovery-Kopien, Logs, Sitzungen, IP-/Peer-Metadaten, Gerätenamen,
User-Agent-Ableitung, Transferverlauf, Browserqueue, Cursor,
Abschlussquittungen, Nutzdateien, Partials, Diagnoseexport und Firewallzustand
jeweils mit Ort, Empfänger, Aufbewahrung und Löschweg ab.

P2 reduzierte den Diagnoseexport auf aggregierte Netzwerk-/Firewallwerte. Ein
Sentineltest belegt, dass Adapter-ID/-name, Profilname, IP, Netzwerk-GUID,
Programmpfad und rohes Firewalldetail nicht serialisiert werden. Logs rotieren
täglich und sind nun auf 14 Dateien begrenzt. Der Uninstaller erhält
Konfiguration, Logs und Nutzer-/Freigabedaten weiterhin absichtlich.

Die Quellprüfung fand in Produktquellen nur die lokale Dienst-URL, die lokale
Tauri-IPC-CSP, die lokale Entwicklungs-URL und die nicht zur Laufzeit geladene
Tauri-Schema-URL. Es gibt keine direkte Telemetrieabhängigkeit und keine
produktseitig fest verdrahtete öffentliche API-, CDN-, Font-, WebSocket- oder
Updateadresse. Die nach beiden Webbuilds geprüften Produktionsassets enthalten
neben lokalen Assets nur W3C-Namespacekonstanten und von React eingebettete
`react.dev/errors/...`-Textlinks. Diese sind inerte Fehlerhilfe-Strings und werden
weder als Ressource geladen noch von LDTG aufgerufen.

## Supportbasis und vertrauliche Meldung

[`SUPPORT.md`](../../SUPPORT.md) schlägt bewusst den kleinsten belastbaren
Rahmen vor: neueste Beta, Windows 11 25H2, nur in P4 tatsächlich bestandene
aktuelle iOS-/Safari- und Android-/Chrome-Kombinationen, kein SLA, monatliche
Best-effort-Abhängigkeits-/Sicherheitsprüfung und ein sichtbarer
Archivierungsweg.

Die zeitabhängigen Grundlagen wurden am 3. September 2026 in Primärquellen
geprüft:

- Windows 10 Home/Pro Supportende:
  <https://learn.microsoft.com/en-us/lifecycle/products/windows-10-home-and-pro>
- Windows 11 Home/Pro Lifecycle:
  <https://learn.microsoft.com/en-us/lifecycle/products/windows-11-home-and-pro>
- Windows 10 ESU-Voraussetzungen:
  <https://learn.microsoft.com/en-us/windows/whats-new/enable-extended-security-updates>
- Chrome auf Android und Mindeststand:
  <https://support.google.com/chrome/answer/95414?co=GENIE.Platform%3DAndroid&hl=en>
- aktuelle Apple-Sicherheitsreleases:
  <https://support.apple.com/en-mide/100100>

GitHub Private Vulnerability Reporting ist laut GitHub-Dokumentation für ein
öffentliches Repository zu aktivieren. Das Repository bleibt jetzt privat;
deshalb wird keine externe Einstellung vorweggenommen. Nach einem späteren `GO`
muss der Kanal vor dem Sichtbarkeitswechsel aktiviert und praktisch getestet
werden:

- <https://docs.github.com/en/code-security/how-tos/report-and-fix-vulnerabilities/configure-vulnerability-reporting/configure-for-a-repository?learn=security_advisories&learnProduct=code-security>
- <https://docs.github.com/en/code-security/how-tos/report-and-fix-vulnerabilities/report-privately?learn=security_advisories&learnProduct=code-security>

## Verifikation

Bereits erfolgreich:

- `pnpm check`: generierte Verträge, TypeScript, ESLint, Coverage,
  Produktions-Webbuilds, Rust, Formatierung und Clippy vollständig grün
- Frontend-Coverage: 36 Desktop- und 39 Mobile-Tests bestanden
- `pnpm --filter @ldtg/mobile build`
- `pnpm test:rust`: 122 bestanden, 0 fehlgeschlagen
- `cargo fmt --manifest-path src-tauri/Cargo.toml --check`
- `git diff --check`
- JSON-Parseprüfung von `qa/public-beta/blockers.json`
- lokaler `pnpm build`: Release-EXE und x64-NSIS-Installer erfolgreich erzeugt;
  damit wurde der tatsächliche Uninstall-Hook von `makensis` akzeptiert
- Produktionsasset-URL-Inventur: ausschließlich W3C-Namespacekonstanten und
  React-Fehlerhilfe, keine externe LDTG-Ressource oder Telemetrieadresse
- lokale Markdown-Linkprüfung ohne fehlendes Ziel

Der direkte Aufruf von `cargo test` ist auf Windows absichtlich nicht der
Projekt-Testweg: Er umgeht das Common-Controls-v6-Manifest des vorhandenen
Wrappers und kann deshalb vor dem Test-Harness mit fehlendem
`TaskDialogIndirect` abbrechen. `pnpm test:rust` beziehungsweise
`scripts/test-rust.ps1` bindet das Manifest ein und ist der maßgebliche Pfad.

Das frische unabhängige Read-only-Fixreview meldete für alle drei Findings
ausdrücklich **keine Fix-Grenzverletzung gefunden**. Es bestätigte ID- und
Kategoriebindung samt Altwert-Neubestätigung, current+legacy Firewallcleanup mit
fail-closed NSIS-Verhalten sowie die durchgängige Übergabe des Peer-Schlüssels
vom Accept bis zur Authgrenze. Als einzige Claim-Präzisierung wurden
Erstellungszeit, Plattform, Dienstzustand und konstanter Datenschutzhinweis des
Diagnoseexports in Datenschutz- und Architekturdokument ergänzt. Die dort
genannten Ausschlüsse sensitiver Kennungen und die 14-Dateien-Konfiguration
wurden bestätigt. Die Desktopanzeige wertet gespeichertes Vertrauen ebenfalls
nur bei identischer ID und Kategorie als aktuell und kennzeichnet einen
Kategoriewechsel als erneut bestätigungspflichtig.

## Verbleibende Grenzen

- Reale UAC-, Firewall-, Installations-/Deinstallations-, Upgrade-,
  Netzwechsel-, iOS-/Safari- und Android-/Chrome-Prüfungen gehören zu P4 und
  werden hier nicht als bestanden dargestellt.
- Die Windows-Nachbartabelle ist Best effort; bei fehlendem oder veraltetem
  Eintrag gilt die IP als Auth-/Verbindungs-Fallback.
- Transport bleibt bewusst unverschlüsseltes HTTP im bestätigten LAN; MITM,
  Internetexposition und nicht vertrauenswürdige Netze sind nicht zugesichert.
- `PB-04` wurde am 3. September 2026 akzeptiert. Der Rahmen wird erst nach einem
  späteren `GO` wirksam; die exakten mobilen Kombinationen bleiben vom realen
  P4-Test abhängig.
