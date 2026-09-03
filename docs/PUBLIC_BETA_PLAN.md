# Plan bis zur Veröffentlichungsentscheidung für LDTG

Stand: 4. September 2026
Status: beschlossenes Entscheidungsprogramm, noch keine Veröffentlichungsfreigabe

## 1. Zweck und verbindliche Grenze

Dieser Plan führt LDTG vom geprüften Stand `0.3.0-rc.1` zu einer belastbaren
Entscheidung darüber, ob das Projekt privat bleibt oder als öffentliche
Open-Source-Beta erscheint. Er autorisiert weder eine Veröffentlichung noch eine
kostenpflichtige Bestellung, Registrierung oder externe Bewerbung.

Bis zum letzten Entscheidungsgate gelten deshalb folgende Regeln:

- Das Repository bleibt privat.
- Die am 4. September 2026 ausdrücklich gewählte `Apache-2.0`-Lizenz ist lokal
  aktiviert. Das ist keine Veröffentlichungsfreigabe.
- Es wird kein GitHub-Repository öffentlich geschaltet und kein öffentliches
  Release erzeugt.
- Es wird kein SignPath-Antrag, Store-Konto, Zertifikat, Domain- oder
  Hostingvertrag beauftragt.
- Wiederkehrende Kosten bleiben bei **0 Euro**. Jede Abweichung benötigt eine
  neue, ausdrückliche Entscheidung.
- R5.2 und Phase 6 bleiben pausiert. Fehlerkorrekturen, Release-Härtung und die
  in diesem Plan definierten Nachweise sind weiterhin zulässig.

Die Veröffentlichung selbst ist ein separater, zustimmungspflichtiger Schritt.
Ohne ausdrückliche Freigabe lautet die Standardentscheidung **HOLD – privat
lassen**.

## 2. Ausgangslage

Der aktuelle Stand besitzt:

- eine Windows-Desktopanwendung und eine im Dienst eingebettete mobile
  Browseroberfläche;
- ein dokumentiertes Sicherheitsmodell für bewusst bestätigte lokale Netze;
- Freigabeprofile mit genau einem aktiven Download- und Uploadbereich;
- typisierte Verträge, automatisierte Tests, CI-Grundlage und einen
  per-user-NSIS-Installer;
- keine Cloud, kein Konto, keinen öffentlichen Dienst und keine Telemetrie;
- die gewählte Projektlizenz `Apache-2.0`, aber noch keine vollständigen
  Drittanbieterhinweise, Codesignierung oder öffentliche Releasepipeline;
- noch keine vollständige reale Abnahmematrix auf Windows 11 25H2 sowie aktuellem
  iOS/Safari und Android/Chrome.

LDTG wird für dieses Programm nicht als fertiges kommerzielles Produkt
behandelt. Der Zielzustand ist höchstens eine klar begrenzte, nachvollziehbar
gebaute **öffentliche Beta**.

## 3. Begriffe, die getrennt bleiben müssen

### 3.1 Open-Source-Lizenz

Die Projektlizenz regelt, wie Dritte Quellcode verwenden, verändern und
weitergeben dürfen. Sie beseitigt keine Windows-Sicherheitswarnung.

### 3.2 Windows-Codesignatur

Die Codesignatur bindet ein Binärartefakt kryptografisch an einen bestätigten
Herausgeber. Ein korrekt lizenziertes Open-Source-Programm kann unsigniert sein;
ein proprietäres Programm kann signiert sein.

### 3.3 Release-Nachweis

Tag, Buildprotokoll, Prüfsumme, SBOM und dokumentierter Buildpfad machen ein
Release nachvollziehbar. Sie ersetzen keine Herausgebersignatur, ermöglichen
aber einen überprüfbaren Bezug zwischen Quellstand und Download.

## 4. Entscheidungen, die am Ende möglich sein müssen

| Entscheidung | Bedeutung | Unmittelbare Folge |
|---|---|---|
| `GO-GPL` | Öffentliche Beta unter `GPL-3.0-only` | Copyleft-Veröffentlichung vorbereiten und SignPath beantragen |
| `GO-APACHE` | Öffentliche Beta unter `Apache-2.0` | permissive Veröffentlichung vorbereiten und SignPath beantragen |
| `HOLD` | Projekt bleibt privat, Entscheidung wird vertagt | keine externe Änderung, private Nutzung bleibt möglich |
| `ARCHIVE` | Entwicklung wird geordnet beendet | privaten Stand dokumentieren und keine Binärdateien veröffentlichen |
| `COMMERCIAL-DISCOVERY` | Erst ein separates Geschäftsmodell prüfen | keine Open-Source-Veröffentlichung und keine Produktinvestition ohne neue Validierung |

Der Owner hat am 4. September 2026 `Apache-2.0` gewählt. Damit sind möglichst
einfache Übernahme, Integration und spätere geschäftliche Flexibilität bewusst
höher gewichtet als ein Copyleft-Zwang für weitergegebene Ableitungen. Der
ursprüngliche LDTG-Code und seine Hinweise bleiben geschützt; größere oder
abgeleitete Werke können jedoch unter anderen Bedingungen verteilt werden.
`GPL-3.0-only` bleibt nur als dokumentierte verworfene Alternative im Dossier.

Eine individuelle Nichtkommerziell-Lizenz oder selbst formulierte Lizenz ist
nicht vorgesehen. Sie wäre keine übliche Open-Source-Lizenz, erschwert
Abhängigkeitsprüfung und Beiträge und würde die SignPath-Eignung gefährden.

## 5. Roadmapübersicht

| Paket | Status | Ziel | Abhängigkeit | Größe | Ergebnis/Gate |
|---|---|---|---|---|---|
| P0 | abgeschlossen | Umfang einfrieren und Entscheidungsregeln festhalten | Phase 5.1 | S | Plan ist verbindlich und verlinkt |
| P1 | abgeschlossen | Repository, Herkunft und Fremdlizenzen auditieren | P0 | M | technische Prüfung und Owner-Bestätigung vollständig |
| P2 | abgeschlossen | Sicherheits-, Datenschutz- und Supportversprechen schärfen | P1 | M | öffentliche Aussagen entsprechen dem Code; Owner akzeptiert Wartungsgrenze |
| P3 | abgeschlossen | Releasepipeline und Herkunftsnachweise härten | P1 | M–L | lokaler/privater Release-Dry-Run reproduzierbar |
| P4 | abgeschlossen | Reale Installations- und Geräteabnahme durchführen | P2–P3 | L | Kernpfade bestanden; nicht ausgeführte Plattform-/Stressfälle ausdrücklich als Beta-Grenzen akzeptiert |
| P5 | abgeschlossen | Open-Source- und SignPath-Unterlagen als Entwurf vorbereiten | P1–P4 | M | Apache-2.0, Pseudonym und Issues-only-Modus bestätigt |
| P6 | offen | Veröffentlichung vollständig trocken durchspielen | P3–P5 | M | unveröffentlichte Beta-Mappe ist vollständig |
| PG | offen | Bewusste Veröffentlichungsentscheidung treffen | P0–P6 | S | genau ein dokumentierter Ausgang |

Die Pakete werden nacheinander abgeschlossen. P2 und P3 dürfen teilweise
parallel vorbereitet werden, ihre Gates bleiben jedoch getrennt. Ein späteres
Paket darf offene Blocker eines früheren Pakets nicht durch bloße Dokumentation
überdecken.

## 6. P0 – Umfang und Kosten einfrieren

Status: abgeschlossen am 3. September 2026.

### Aufgaben

- Den Funktionsumfang von `0.3.0-rc.1` als Basis festhalten.
- R5.2, Wiederaufnahme über Seitenreload/Dienstneustart, Sammeltransfers,
  Transportverschlüsselung, Auto-Update und kommerzielle Funktionen aus dem
  Entscheidungsprogramm ausschließen.
- Zulässige Änderungen auf Fehlerbehebung, Testbarkeit, sichere Installation,
  Releaseherkunft und öffentliche Dokumentation begrenzen.
- Eine Kostenliste mit `0 Euro` als verbindlichem Vorentscheidungslimit führen.
- Jede externe Zustandsänderung als gesondert genehmigungspflichtig markieren.

### Gate P0

- Dieser Plan ist in Roadmap und README verlinkt.
- Arbeitsumfang, Nichtziele, Kostenlimit und Freigabegrenze sind widerspruchsfrei.

## 7. P1 – Repository- und Lizenz-Audit

Status: abgeschlossen am 3. September 2026. Die Historienentscheidung `PB-01`
wurde nach verifiziertem privatem Vollbackup mit einem Rewrite von `main`, dem
Vorbereitungsbranch und den drei Release-Tags umgesetzt. Die Namensentscheidung
`PB-03` ist mit der
Umbenennung in **LDTG – Local Device Transfer Gateway** und der dokumentierten
[Kollisions-Vorprüfung](BRANDING.md) abgeschlossen. Die Herkunft und Rechte vor
dem Wurzelcommit sowie die Freigabefähigkeit der QA-Testdaten wurden mit `PB-02`
vom Repositoryinhaber bestätigt. Es erfolgten keine Veröffentlichung,
Lizenzaktivierung oder Anmeldung.

### 7.1 Herkunft und Rechte

- Festhalten, welche Bestandteile selbst erstellt, generiert, übernommen oder
  durch Werkzeuge unterstützt wurden.
- Icons, Logos, Screenshots, Fonts, Texte, Testdaten und eingebettete Webassets
  einzeln auf Herkunft und erlaubte Weitergabe prüfen.
- Unklare oder nicht benötigte Fremdassets ersetzen oder entfernen.
- Name und Abkürzung `LDTG` wurden auf offensichtliche Konflikte in
  Softwarekatalogen, Paketregistern und Markenregistern geprüft und in
  [`docs/BRANDING.md`](BRANDING.md) dokumentiert; daraus folgt keine formale
  Markenfreigabe.

### 7.2 Git- und Datenschutzprüfung

- Vollständige Git-Historie auf Zugangsdaten, Tokens, private URLs, E-Mail-
  Adressen, Benutzernamen, absolute persönliche Pfade und echte Nutzdaten prüfen.
- Arbeitsbaum, Tags, Branches, große Binärdateien und erzeugte Artefakte erfassen.
- Beispielkonfigurationen, Logs und QA-Screenshots auf personenbezogene oder
  systemidentifizierende Inhalte prüfen.
- Falls eine Historienbereinigung nötig ist, vor jedem Rewrite eine getrennte
  Sicherung und eine ausdrückliche Zustimmung verlangen.

### 7.3 Abhängigkeitslizenzen

- Für npm/pnpm und Cargo einen vollständigen, versionsgenauen Lizenzbericht aus
  den Lockfiles erzeugen.
- Direkte und transitive Abhängigkeiten einschließlich Build-, Test- und
  gebündelter Laufzeitkomponenten unterscheiden.
- Copyleft-, Source-available-, unbekannte und nicht deklarierte Lizenzen
  einzeln prüfen.
- Lizenztexte, Copyright-Hinweise und erforderliche Notices bestimmen.
- Die Vereinbarkeit mit `GPL-3.0-only`, `Apache-2.0` und den SignPath-
  Bedingungen separat dokumentieren.
- Einen maschinenlesbaren SBOM-Entwurf in SPDX oder CycloneDX erzeugen; Format
  und Generator werden festgehalten.

### Ergebnisse

- [Repository-, Herkunfts- und Datenschutz-Audit](../qa/public-beta/repository-audit.md)
- [Abhängigkeits- und Lizenz-Audit](../qa/public-beta/dependency-license-audit.md)
- [maschinenlesbare Lizenzinventur](../qa/public-beta/dependency-licenses.json)
  und [CycloneDX-1.6-SBOM-Entwurf](../qa/public-beta/sbom.cdx.json)
- [maschinenlesbarer Repositorynachweis](../qa/public-beta/repository-evidence.json)
- [Blockerliste](../qa/public-beta/blockers.json) mit Eigentümer, Schweregrad,
  Entscheidungsmöglichkeiten und Erledigungsnachweis
- [bestätigte Herkunftserklärung](../qa/public-beta/provenance-attestation.md)

### Gate P1

- Kein ungeklärtes fremdes oder proprietäres Artefakt würde veröffentlicht.
- Jede ausgelieferte Abhängigkeit besitzt eine bekannte, kompatible Lizenz.
- Erforderliche Drittanbieterhinweise sind vollständig ableitbar.
- Es befinden sich keine Geheimnisse oder privaten Nutzdaten im vorgesehenen
  öffentlichen Git-Stand.

Gatebewertung vom 3. September 2026: **erfüllt**. Der
Abhängigkeitsanteil ist mit 857 inventarisierten Paketversionen, null unbekannten
Lizenzdeklarationen und ableitbaren Notices abgeschlossen. Der bereinigte
Arbeitsbaum besitzt keinen offenen Secret-Befund. `main` und die drei
Release-Tags wurden nach privatem Vollbackup neu geschrieben und anschließend
als exakte Veröffentlichungs-Refs mit null persönlichen Pfad- oder
Identitätstreffern geprüft. Die Historien- und Namensrisiko-Entscheidungen sind
abgeschlossen. Der Repositoryinhaber hat mit `PB-02` außerdem Ursprung,
Veröffentlichungsrechte für die Projektassets und Freigabefähigkeit der
QA-Testdaten bestätigt.

## 8. P2 – Sicherheits-, Datenschutz- und Supportgrenzen

Status: am 3. September 2026 abgeschlossen. Der Owner hat den engen Support-
und Wartungsrahmen mit `PB-04` akzeptiert. Er wird erst durch ein späteres `GO`
für eine öffentliche Beta wirksam. Es erfolgten keine Veröffentlichung,
Lizenzaktivierung, Anmeldung oder kostenpflichtige Maßnahme.

### 8.1 Sicherheitswahrheit

- README, `SECURITY.md`, Threat Model und tatsächlichen Code erneut abgleichen.
- Die Produktgrenze „HTTP nur im bewusst bestätigten vertrauenswürdigen LAN“
  sichtbar und ohne irreführende Verschlüsselungsbehauptung darstellen.
- Nichtziele wie Internetfreigabe, Portweiterleitung, Cloudzugriff,
  Virenscan und Fernadministration klar nennen.
- Sicherheitsrelevante Voreinstellungen, Firewalländerungen, AppData-Erhalt und
  Deinstallationsverhalten dokumentieren.
- Öffentliche GitHub-Meldung für vertrauliche Schwachstellen vorsehen, ohne eine
  private E-Mail-Adresse veröffentlichen zu müssen.

### 8.2 Datenschutz

- Datenfluss für Einstellungen, Logs, Transferverlauf, IP-Adressen,
  Gerätenamen und User-Agent-Ableitung tabellarisch dokumentieren.
- Speicherorte, Aufbewahrung, lokale Löschmöglichkeiten und Nichtübertragung an
  Dritte beschreiben.
- Prüfen, dass Anwendung, Websiteassets und Build keine Telemetrie oder externen
  Laufzeitressourcen nachladen.

### 8.3 Support und Wartung

- Öffentliche Beta ohne SLA und ohne garantierte Antwortzeit definieren.
- Unterstützte Windows- und Browserstände festlegen.
- Eine realistische minimale Wartungsweise zur Entscheidung vorlegen, zum
  Beispiel monatliche Abhängigkeits- und Sicherheitsprüfung ohne Garantie.
- Vorgehen bei Aufgabe des Projekts definieren: deutlicher
  „nicht mehr gepflegt“-Hinweis, Archivierung und keine Darstellung alter
  Binärdateien als weiterhin sicher unterstützte Version.

### Ergebnisse

- [P2-Sicherheits-, Datenschutz- und Supportnachweis](../qa/public-beta/p2-security-privacy-support.md)
- [aktualisiertes Bedrohungsmodell](THREAT_MODEL.md) und
  [Sicherheitsrichtlinie](../SECURITY.md)
- [Dateninventar und Löschwege](PRIVACY.md)
- [akzeptierter Beta-Supportrahmen](../SUPPORT.md)
- drei bestätigte Findings niedriger Schwere behoben: Netzwerkvertrauen bindet
  ID und Kategorie; Uninstall entfernt aktuelle und historische Firewallregel
  fail-closed; Auth-Fehlversuche verwenden denselben physischen Peer-Schlüssel
  wie die Verbindungsgrenze

### Gate P2

- Öffentliche Aussagen sind enger oder gleich eng wie die nachgewiesene
  Implementierung.
- Datenschutz- und Supportumfang erzeugen keine unbeabsichtigte Dauerzusage.
- Kritische Sicherheitsmeldungen könnten vertraulich entgegengenommen werden.

Gatebewertung vom 3. September 2026: **bestanden**. Code, Tests,
Datenschutzinventar und Kanalentwurf sind vorbereitet. Der Owner hat mit
`PB-04` den engen Rahmen „neueste Beta, Windows 11 25H2, nur in P4 bestandene
aktuelle Mobilkombinationen, kein SLA, monatliche Best-effort-Sicherheitsprüfung,
geordnete Archivierung“ akzeptiert. GitHub Private Vulnerability Reporting wird
erst nach einem späteren `GO` und unmittelbar vor Sichtbarkeitswechsel aktiviert
und getestet; solange das Repository privat ist, existiert bewusst kein externer
Meldekanal.

## 9. P3 – Releasepipeline und Herkunftsnachweis

Status: abgeschlossen am 3. September 2026. Der Owner hat mit `PB-05` genau
einen lokalen Commit und den anschließenden kostenfreien Clean-Commit-Dry-Run
autorisiert. Der Lauf war erfolgreich. Es erfolgten keine Veröffentlichung,
Signierung oder kostenpflichtige Maßnahme.

### 9.1 Privater CI-Releasepfad

- Einen Windows-Workflow entwerfen, der ausschließlich aus einem festgelegten
  Commit/Tag und eingecheckten Lockfiles baut.
- Das vollständige Qualitätsgate vor dem Paketieren erzwingen.
- Versionsgleichheit in Cargo, npm-Workspaces, Tauri und Changelog automatisch
  prüfen.
- NSIS-Installer und gegebenenfalls portable ZIP-Datei getrennt erzeugen.
- SHA-256-Prüfsummen, SBOM, Buildprotokoll und eindeutige Commit-ID gemeinsam
  ausgeben.
- Aktionen und Buildwerkzeuge möglichst auf überprüfte Versionen oder vollständige
  Commit-Hashes festlegen.
- Releaseberechtigungen minimal halten und Build- von Freigabeschritten trennen.

### 9.2 Reproduzierbarkeit und Signierbarkeit

- Festhalten, welche Dateianteile reproduzierbar sind und welche Zeitstempel
  oder NSIS-Metadaten deterministische Binärgleichheit verhindern.
- Mindestens nachweisen, dass jedes Artefakt eindeutig aus demselben Quellstand
  und kontrollierten Abhängigkeiten gebaut wurde.
- Eine SignPath-kompatible Trennung zwischen Build, manueller Freigabe und
  Signierung vorsehen.
- Eine `CODE_SIGNING.md` als Entwurf erstellen, aber vor dem finalen Gate keine
  SignPath-Zusage behaupten.

### Gate P3

- Ein privater Dry-Run erzeugt installierbare Artefakte, Prüfsummen, SBOM und
  Buildnachweis ohne manuelle Dateiänderung.
- Ein fehlgeschlagener Test, Versionskonflikt oder unvollständiger Audit stoppt
  das Paketieren.
- Für den Dry-Run werden keine dauerhaften Zugangsdaten oder bezahlten Dienste
  benötigt.

Gatebewertung vom 3. September 2026: **bestanden**. Der lokale Dry-Run aus Commit
`4c48058fc1b438ae1f0d5a76a2b17408a6b4b25e` erzeugte nach dem vollständigen
Qualitätsgate einen frischen unsignierten NSIS-Installer, vier unabhängig
bestätigte SHA-256-Einträge, eine commitgebundene CycloneDX-SBOM mit 857
Komponenten, Buildlog und Manifest. Der Quellbaum blieb sauber. Details und
exakte Hashes stehen im [privaten Releasepfad](PRIVATE_RELEASE.md) und im
[P3-Nachweis](../qa/public-beta/p3-release-pipeline.md).

## 10. P4 – Reale Abnahme

Status: **abgeschlossen am 4. September 2026**; die Releaseblocker `PB-08` und
`PB-09` sind geschlossen.
Installation und grundlegender Funktionslauf waren erfolgreich. `P4-FW-01`
wurde als Testumgebungsartefakt geschlossen: Die erste aus dem paketierten
Codex-Kontext gestartete Installation wurde verworfen, ihre zwei breiten
Windows-Autoregeln wurden entfernt und die Neuinstallation über den unabhängigen
Datei-Explorer wiederholt. Der Firewall-/Dienstfluss zeigte keinen zweiten
Dialog; die erhöhte Prüfung bestätigte genau eine enge LDTG-Regel. Anschließend
quarantänisierte Microsoft Defender den getesteten P3-Build als
`Trojan:Win32/Bearfoos.A!ml`. Der ersetzende AV-Härtungskandidat verwendet für
Firewall- und Netzwerkzugriffe keine PowerShell mehr und paketiert den
Vertragsgenerator nicht mehr. Sein Defender-, Installations-, Firewall-,
Dienst- und bidirektionaler Kleintransfer-Retest blieb ohne neuen AV-Fund. Beim
vollständigen Uninstall entfernte der erhöhte Helfer die Regel, aber der
NSIS-Hook wertete wegen eines nicht unterstützten `ExecShellWait`-Exitcodes
fälschlich einen Fehler aus (`PB-09`). Nach der Korrektur bestanden
Defender-Prüfung, Darüberinstallation, enge Firewallregel und vollständiger
Uninstall mit erhaltenen AppData- und Freigabedaten. Die reale Neuinstallation
erhielt anschließend Freigaben und Einstellungen. Der Owner akzeptierte die
nicht real ausgeführten Upgrade-, iOS-, Chrome-, Großdatei-, Queue-,
Datenträger-, Netzwerkwechsel- und Barrierefreiheitsfälle ausdrücklich als
sichtbare Beta-Grenzen; automatisierte Sicherheits- und Konsistenztests bleiben
grün. P4 ist damit für den engen Beta-Supportumfang abgeschlossen.

### 10.1 Windows

- Windows 11 25H2 mit Installation, abgebrochener UAC-Abfrage,
  Start, Upgrade, Portwechsel, Firewallregel, Dienststopp und Deinstallation
  prüfen.
- Windows 10 und ältere Windows-11-Stände dürfen optional als reine
  Kompatibilitätsbeobachtung erfasst werden, erweitern den Supportumfang aber
  nicht.
- Sentinel-Dateien nach Deinstallation erhalten und Firewallregel entfernt
  nachweisen.
- Verhalten eines unsignierten Downloads einschließlich SmartScreen-Hinweis
  dokumentieren; Nutzer dürfen nicht zum gedankenlosen Wegklicken angeleitet
  werden.
- Tastaturbedienung, Basis-Screenreaderprüfung, Reduced Motion, 200-%-Skalierung
  und schmale Fenster prüfen.

### 10.2 Mobilgeräte und Transferfälle

- Aktuelles iOS/Safari und Android/Chrome in einem WLAN ohne Internet testen.
- Login, Coderotation, zwei Sitzungen, Einzel- und Gesamtwiderruf prüfen.
- Download, Range-Fortsetzung, Upload, Pause/Fortsetzung innerhalb derselben
  Dateiauswahl, Netzwerkverlust und Seitenreload prüfen.
- Große Datei, viele kleine Dateien, Unicode/Nicht-BMP, Namenskollisionen,
  langsamen und vollen Datenträger abdecken.
- Profilwechsel, Vererbung und Overrides praktisch prüfen.

### 10.3 Befundregeln

- P0: Datenverlust, Codeausführung, Authentisierungsumgehung oder Zugriff
  außerhalb einer Freigabe – Veröffentlichung blockiert.
- P1: reproduzierbarer Absturz, unsichere Standardkonfiguration oder kaputter
  Installations-/Deinstallationspfad – Veröffentlichung blockiert.
- P2: erheblicher Bedien- oder Kompatibilitätsfehler – beheben oder im finalen
  Gate ausdrücklich akzeptieren.
- P3: kleinere Darstellung oder Komfortabweichung – darf dokumentiert in die
  Beta übernommen werden.

### Gate P4

- Alle verfügbaren Matrixfelder besitzen Ergebnis, Datum, Plattform und Beleg.
- P0 und P1 sind geschlossen; offene P2 sind einzeln entscheidbar.
- Nicht verfügbare Hardware wird als echte Evidenzlücke ausgewiesen und nicht
  als bestanden markiert.

Gatebewertung vom 4. September 2026: **bestanden mit ausdrücklich akzeptierten
Evidenzlücken**. Es gibt keinen offenen P0-/P1-Befund. Die tatsächlich getestete
Mobilkombination ist Android 16 mit Firefox; andere Mobilplattformen und
Browser werden nicht als getestet zugesichert. Der beobachtete Anzeigehinweis
`P4-UI-01` ist als P3-UX-Klarstellung behoben und ohne Großdatei-Realtest
akzeptiert. Da die Klarstellung den eingebetteten Webbuild ändert, muss P6 ein
frisches finales Artefakt erzeugen und vor jeder Veröffentlichung erneut
hashen und mit Defender prüfen.

## 11. P5 – Lizenz-, Mitwirkungs- und SignPath-Entwürfe

### 11.1 Lizenzdossier

Für `GPL-3.0-only` und `Apache-2.0` wird jeweils festgehalten:

- erlaubte private und kommerzielle Nutzung;
- Pflichten bei unveränderter und veränderter Weitergabe;
- Wirkung auf proprietäre Ableitungen;
- Patent- und Haftungsregelungen;
- Kompatibilität mit allen ausgelieferten Abhängigkeiten;
- Auswirkung fremder Beiträge auf einen späteren Lizenzwechsel;
- Vereinbarkeit mit kostenloser SignPath-Signierung.

Der Eigentümer- und Copyright-Hinweis verwendet den vom Nutzer gewählten realen
Namen, ein geeignetes Pseudonym oder später eine juristische Person. Codex wird
nicht als Urheber, Rechteinhaber oder Herausgeber eingetragen.

### 11.2 Beiträge Dritter

- `CONTRIBUTING.md`, Pull-Request-Vorlage und Verhaltensregeln entwerfen.
- Festlegen, ob zunächst nur Issues oder auch Codebeiträge erwünscht sind.
- Vor dem ersten angenommenen Fremdbeitrag zwischen einfachem Beitrag unter der
  Projektlizenz, Developer Certificate of Origin oder Contributor License
  Agreement entscheiden.
- Keine Lizenzübertragung oder weitreichende Rechteabtretung beiläufig in einer
  allgemeinen Beitragsdatei verstecken.
- Maintainer behalten vollständige Review- und Ablehnungsfreiheit; es besteht
  kein Anspruch auf Merge oder Support.

### 11.3 SignPath-Vorprüfung

- OSI-anerkannte Lizenz und vollständig offenen Lieferumfang nachweisen.
- Verifizierbaren öffentlichen Buildpfad, manuelle Signierfreigabe und getrennte
  Rollen für Committer/Reviewer und Approver entwerfen.
- Festhalten, dass die kostenlose SignPath-Stiftungslösung nach aktuellem Stand
  keine kommerzielle Doppellizenzierung der Projektkomponenten erlaubt. Eine
  spätere proprietäre Doppellizenz wäre daher ein anderer Veröffentlichungsweg.
- MFA für Repository- und spätere SignPath-Zugänge als Voraussetzung festlegen.
- Datenschutztext, Deinstallationsweg und angekündigte Systemänderungen gegen
  die SignPath-Bedingungen prüfen.
- Projekt-Reputation und „bereits veröffentlicht“-Voraussetzung als noch offene,
  erst nach einer öffentlichen Beta erfüllbare Punkte kennzeichnen.
- Festhalten, dass Annahme und Warnungsfreiheit nicht garantiert werden.

### Gate P5

- Beide Lizenzoptionen sind vollständig vergleichbar und rechtlich nicht durch
  unbekannte Abhängigkeiten blockiert.
- Beitragsregeln schützen die gewählte Lizenzrichtung und erzeugen keine
  versteckte kommerzielle Doppelstrategie.
- Alle vor einer SignPath-Bewerbung intern erfüllbaren Bedingungen sind erfüllt
  oder mit einer konkreten Restmaßnahme versehen.

Zwischenstand vom 4. September 2026: Die
[P5-Entscheidungsmappe](../qa/public-beta/p5-license-contribution-signpath.md)
vergleicht beide Lizenzwege, enthält nicht aktive Beitragsentwürfe und bewertet
die aktuellen SignPath-Bedingungen. `Apache-2.0` wurde mit ausdrücklicher
Owner-Freigabe aktiviert. `Kordariel666` wurde als vorläufiges öffentliches
Copyright-Pseudonym und „zunächst nur Issues“ als Beitragsmodus bestätigt. P5
ist damit abgeschlossen; es wurde kein Antrag begonnen und nichts
veröffentlicht.

## 12. P6 – Unveröffentlichter Beta-Dry-Run

### Inhalt der Entscheidungsmappe

- exakter Quellcommit und vorgesehener Beta-Tag;
- unveröffentlichter Installer und gegebenenfalls portable ZIP-Datei;
- SHA-256-Prüfsummen und SBOM;
- vollständiges Buildprotokoll;
- Entwurf der GitHub-Release-Notes;
- Entwurf von README, Lizenz, Drittanbieterhinweisen, Datenschutz-, Support-,
  Beitrags- und Codesignierungsdokumenten;
- reale QA-Matrix mit offenen Befunden;
- Screenshot der zu erwartenden unsignierten Windows-Warnung und ein sachlicher
  Nutzerhinweis;
- SignPath-Gapanalyse;
- Kosten- und Wartungsabschätzung;
- Liste aller öffentlichen Namen, Kontaktwege und Metadaten.

### Trockenübung

- Veröffentlichung aus einer bereinigten Kopie beziehungsweise einem
  freigabefähigen Branch simulieren, ohne Remote-Sichtbarkeit zu verändern.
- Alle Links und Installationsschritte aus Sicht eines neuen Nutzers prüfen.
- Sicherstellen, dass ein versehentlicher Push oder Release nicht Teil des
  Dry-Run-Befehls ist.
- Prüfen, dass eine Veröffentlichung mit genau einem bewusst bestätigten Schritt
  erfolgen könnte und ein `HOLD` keine Rückbauarbeiten erfordert.

### Gate P6

- Die Entscheidungsmappe ist vollständig, intern konsistent und enthält keine
  Behauptung einer vorhandenen Signatur.
- Zwischen `GO` und Veröffentlichung existiert kein unbekanntes technisches oder
  lizenzrechtliches Arbeitspaket mehr.
- Alle verbleibenden Risiken sind sichtbar und einzeln akzeptierbar.

Zwischenstand vom 4. September 2026: `0.3.0-rc.2` ist als unveröffentlichter
Kandidat festgelegt. Die tatsächliche Backendversion wird dauerhaft in der
Desktop-Seitenleiste angezeigt. Vollständige Laufzeit-Notices, Release-Notes
und die [P6-Entscheidungsmappe](../qa/public-beta/p6-release-candidate.md) sind
vorbereitet. P6 bleibt bis zum Clean-Commit-Dry-Run und der manuellen
Defender-/Installations-/Kernfunktionsabnahme offen.

## 13. PG – Veröffentlichungsentscheidung

Die Entscheidung wird in `docs/PUBLICATION_DECISION.md` dokumentiert. Sie enthält:

- Datum und betrachteten Commit;
- gewählte Option aus Abschnitt 4;
- gewählte Lizenz oder den Grund für `HOLD`/`ARCHIVE`;
- akzeptierte offene P2/P3-Befunde;
- bestätigte öffentliche Identität und Kontaktwege;
- akzeptierten Wartungsumfang;
- erwartete einmalige und laufende Kosten;
- Entscheidung über Beiträge Dritter;
- Entscheidung über eine zunächst unsignierte Beta;
- Bedingungen für den späteren SignPath-Antrag;
- ausdrückliche Freigabe oder Ablehnung der externen Zustandsänderung.

### Mindestbedingungen für `GO-GPL` oder `GO-APACHE`

- P0 bis P6 sind erfüllt.
- Das vollständige Qualitätsgate ist grün.
- P0/P1-Befunde sind geschlossen.
- Fremdlizenzen, Notices und SBOM sind vollständig.
- Eine reale Windows- und Mobilabnahme ist belegt oder ihre Restlücke wurde
  ausdrücklich akzeptiert.
- Der unsignierte Status und die damit mögliche Windows-Warnung werden sichtbar
  kommuniziert.
- Es gibt keine bezahlte oder automatisch verlängernde Verpflichtung.
- Der Nutzer genehmigt Veröffentlichung, Lizenz und öffentliche Identität
  ausdrücklich in einem eigenen Schritt.

## 14. Kostenkontrolle

Vor PG zulässig:

- lokaler Build und lokale Testgeräte;
- private GitHub-Funktionen innerhalb vorhandener kostenloser Kontingente;
- freie Auditwerkzeuge ohne Konto-, Zahlungs- oder Uploadpflicht;
- Dokumentation und unveröffentlichte Artefakte.

Vor PG nicht zulässig:

- Zertifikatskauf oder Signierabonnement;
- Domain, Hosting, Werbung oder bezahlte Tester;
- Microsoft-Store- oder andere Kontoregistrierung mit Außenwirkung;
- kostenpflichtige Rechts-, Steuer- oder Markenleistung ohne Einzelentscheidung;
- Aktivierung verbrauchsabhängiger Cloudkosten ohne hartes Nullbudget.

Ein später akzeptiertes `GO` darf weiterhin auf einen kostenlosen GitHub- und
SignPath-Pfad zielen. Wenn SignPath ablehnt, folgt keine automatische Ausgabe;
die Alternativen „vorerst unsigniert“, „Microsoft Store“, „bezahlte Signierung“
und `HOLD` werden erneut getrennt entschieden.

## 15. Hauptrisiken und Gegenmaßnahmen

| Risiko | Gegenmaßnahme |
|---|---|
| Windows-Warnung wird mit fehlender Lizenz verwechselt | Lizenz- und Signaturstatus getrennt in README und Release erklären |
| Fremdlizenz verhindert die gewählte Projektlizenz | vollständiger Lockfile-Audit vor Lizenzaktivierung |
| Öffentliche Historie enthält persönliche Daten | gesamte Historie prüfen; Rewrite nur nach Sicherung und Zustimmung |
| GPL erschwert spätere proprietäre Doppelverwertung | Lizenz- und Beitragsentscheidung vor erstem öffentlichen Merge treffen |
| Apache erlaubt proprietäre Ableitungen | nur wählen, wenn weite Nachnutzung bewusst akzeptiert wird |
| SignPath lehnt ein neues oder unbekanntes Projekt ab | keine Zusage machen; unsignierte Beta/HOLD als echte Alternativen behalten |
| Veröffentlichung erzeugt unbegrenzte Supporterwartung | Beta-, Support- und Archivierungsrichtlinie klar begrenzen |
| Funktionsausbau verdrängt Releasehärtung | R5.2 und Phase 6 bis PG pausieren |
| Kommerzielle Idee erzeugt vor Nachfrage Kosten | separates Discovery-Gate; keine Vermischung mit dieser Beta |

## 16. Quellen für die externe Entscheidung

- GitHub, „Licensing a repository“:
  <https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/licensing-a-repository>
- Open Source Initiative, FAQ:
  <https://opensource.org/faq>
- GNU, GPL FAQ:
  <https://www.gnu.org/licenses/gpl-faq.en.html>
- Choose a License, GPL-3.0:
  <https://choosealicense.com/licenses/gpl-3.0/>
- SignPath Foundation, Bedingungen:
  <https://signpath.org/terms.html>
- LocalSend, Codesignierungsrichtlinie:
  <https://github.com/localsend/localsend/blob/main/CODE_SIGNING.md>
- Microsoft, SmartScreen-Reputation:
  <https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/smartscreen-reputation>
- Europäische Kommission, Cyber Resilience Act und Open Source:
  <https://digital-strategy.ec.europa.eu/en/policies/cra-open-source>

Die Quellen wurden am 3. September 2026 geprüft. Rechtliche, steuerliche und
regulatorische Einzelfragen werden vor einer kommerziellen Bereitstellung erneut
anhand des dann geplanten Modells bewertet; dieser Plan ist keine individuelle
Rechts- oder Steuerberatung.
