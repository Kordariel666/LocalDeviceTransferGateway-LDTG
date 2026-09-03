# Architektur von LDTG v1

## Grenzen und Module

Die Desktop-WebView ist eine reine Steueroberfläche. Sie besitzt nur eng definierte Tauri-Befehle für Einstellungen, Dienst, Sitzungen, Firewall und Diagnose. Allgemeine Shell- oder Dateisystemrechte sind nicht freigegeben.

Der Rust-Code ist in drei Schichten gegliedert:

- `domain`: persistierbare Einstellungen, Netzwerkerkennung, Pfad- und Dateinamenregeln sowie UI-Vertragstypen.
- `service`: kurzlebiger Dienstzustand, Authentifizierung, Sitzungen, Übertragungen und die Axum-HTTP-API.
- `platform`: betriebssystemspezifische Funktionen. In v1 ist nur der Windows-Firewallhelfer implementiert.

Die mobile React-App wird separat gebaut und mit `rust-embed` in die Rust-Binärdatei aufgenommen. Dadurch benötigt LDTG weder Internet noch einen separaten Webserver.

Innerhalb der Schichten gelten folgende Modulgrenzen:

- `service/api.rs` verdrahtet ausschließlich den Axum-Router. Die Untermodule
  `auth`, `common`, `directory`, `download`, `upload` und `assets` besitzen die
  jeweiligen HTTP-Abläufe; `common` ist die einzige Stelle für API-Fehler,
  Request-Guard und Sicherheitsheader.
- `service/state.rs` besitzt die gemeinsamen Records, Permits, Konstanten und
  die Initialisierung einer Dienstinstanz. `sessions`, `limits`, `cursors`,
  `uploads`, `downloads` und `journal` ergänzen darauf ausschließlich ihre
  domänenspezifischen Zustandsübergänge.
- Die Rust-Regressionen für diese beiden Bereiche liegen in den dedizierten
  Untermodulen `api/tests.rs` und `state/tests.rs`. Sie dürfen private
  Invarianten prüfen, werden aber nicht in Produktionsdateien eingebettet.
- Mobile verwendet `apiClient` für HTTP-Fehler und JSON, `useSession` für den
  synchronisierten Session-State, `DirectoryBrowser` für die
  Downloadnavigation sowie `uploadQueue` und `UploadQueueView` für fachlichen
  Queue-Zustand und Darstellung.
- Desktop verwendet `tauriClient` als IPC-Grenze, `useLifecycle` für native
  Ereignisse, Polling und Dirty-State-Spiegelung, `settingsDraft` für lokale
  Entwurfsregeln sowie getrennte `components`- und `pages`-Module für die
  Darstellung. `DesktopApp` koordiniert diese Bausteine und besitzt weiterhin
  die Benutzeraktionen und ihren Zustand.
- `packages/shared/src/presentation.ts` formatiert Bytewerte, Raten und Dauern
  und bewertet die Belastbarkeit einer Restzeitschätzung identisch für Desktop
  und Mobile. Das Modul akzeptiert ausschließlich Zeit- und Bytewerte.

Serialisierbare DTOs in `src-tauri/src/domain` bilden die einzige Quelle für die
von Rust, Desktop und Mobile gemeinsam verwendeten Verträge. `ts-rs` erzeugt
daraus `packages/shared/src/index.ts`; Dienst- und Transferzustände,
Transferrichtung sowie Download-Eintragsarten werden auf Rust-Seite als Enums und
auf TypeScript-Seite als benannte String-Unions dargestellt. Der Generator gibt
64-Bit-JSON-Zahlen bewusst als TypeScript-`number` aus, weil die HTTP- und
Tauri-Grenze JSON und kein BigInt transportiert. Ein read-only Drift-Vergleich ist
Teil des lokalen und des CI-Qualitätsgates.

Fallible Tauri-Befehle geben einen einheitlichen `CommandError` zurück. Sein
generierter Vertrag besteht aus stabilem Code, sicherem Anzeigetext und optionalem
diskriminiertem Kontext. Bestätigungstoken, Netzwerkname, breit gewählter Pfad und
Anzahl aktiver Übertragungen werden ausschließlich in den dafür vorgesehenen
Kontextvarianten transportiert; der Desktop verzweigt nur anhand von Code und
Kontextart. Interne Task-, Betriebssystem-, Datei- und Dienstursachen werden vor
der IPC-Grenze auf sichere Meldungen reduziert. Tauri-Befehlsfehler protokollieren
datensparsam nur Code und Operation, nicht die rohe Ursache oder einen darin
enthaltenen Pfad; die Dienstschicht kann zusätzlich technische Socket- und
Serverfehler protokollieren. Logs rotieren täglich und sind auf 14 Dateien
begrenzt.

## Zustandsmodell

Konfiguration ist nur im Zustand `stopped` änderbar. Start, Stop, Quit, Status-Reaping, Speichern und Firewallkonfiguration teilen einen Lifecycle-Transition-Mutex; ein zweiter Übergang kann daher weder einen noch nicht veröffentlichten Start überholen noch den Zustand einer neueren Dienstinstanz überschreiben. Jeder Übergang nach `running` erzeugt eine neue Dienst-ID und einen neuen Zugangscode. Sitzungen, Uploadzuordnungen und Übertragungsverlauf leben nur in dieser Instanz.

`settings.json` verwendet ein von der Appversion unabhängiges Konfigurationsschema.
Beim Laden wird zunächst nur die Schemaangabe gelesen; versionslose Daten werden
als Schema 0 und ältere Daten anschließend Schritt für Schritt bis zum aktuellen
Schema 4 migriert. Erst danach folgen Deserialisierung und semantische Validierung.
Neuere Schemata, falsche Feldtypen und ungültige Grenzen werden nicht übernommen:
Die App arbeitet mit sicheren Standardwerten, zeigt eine dauerhafte Warnung und
lässt die Quelldatei unverändert. Vor einem späteren bewussten Ersetzen entsteht
eine nummerierte `settings.recovery-N.json`. Speichern ist atomar, normalisiert
ältere Entwürfe auf Schema 4 und lehnt zukünftige Schemaangaben ab. Schema 3
ersetzt die früheren reinen Netzwerk-IDs durch begrenzte, eindeutig validierte
Vertrauensdatensätze mit stabiler ID, Anzeigename, Kategorie und letzter
Verwendung. Ein Start vertraut nur einer vollständig aufgelösten Identität mit
exakt derselben ID und bestätigten bekannten Kategorie; der Anzeigename ist nur
Darstellung. Nicht mehr auflösbare oder in der Kategorie geänderte IDs bleiben
sichtbar und können bei gestopptem Dienst einzeln oder vollständig entfernt werden. Die laufende
Buildversion gehört ausschließlich zum App-Snapshot und Diagnosebericht und wird
nicht in Benutzereinstellungen persistiert.

Schema 4 überführt die bisherigen beiden Einzel-Freigaben verlustfrei in ein
Profil namens „Standard“. Bis zu 32 lokal gespeicherte Profile besitzen jeweils
eine Download- und Uploadfreigabe sowie eine stabile zufällige ID. Netzwerk,
Port und der zusammengehörige Limitsatz erben standardmäßig die gemeinsamen
Werte; ein Profil kann jede dieser drei Gruppen explizit überschreiben. Vor dem
Start wird ausschließlich das aktive Profil in einen eigenständigen, unveränderlichen
`RuntimeSettings`-Wert aufgelöst. Dadurch sieht der HTTP-Dienst weiterhin genau
eine Download- und eine Uploadwurzel, und weder API v1 noch Pfad-, Rollen- oder
Ressourcengrenzen werden durch die Profilverwaltung aufgeweitet. Profilwechsel,
Duplizieren, Umbenennen und Löschen bleiben wie alle Konfigurationsänderungen auf
den gestoppten Zustand begrenzt.

Der Desktop hält gespeicherten Snapshot und bearbeiteten Entwurf getrennt. Ein
struktureller Vergleich erzeugt den sichtbaren Dirty-State; Hintergrundstatus und
Seitennavigation ersetzen den Entwurf nicht. Eine aus R1.3 stammende
Recovery-Warnung erlaubt separat die bewusste Übernahme sicherer Standardwerte,
ohne dies fälschlich als Benutzeränderung zu markieren. Lokale Feldprüfung deckt Zahlen,
Größenabhängigkeiten und fehlende Ordner ab. Aktivierte Freigaben werden zusätzlich
über einen Tauri-Befehl außerhalb des Async-Runtimes mit derselben kanonischen
Pfadpolitik wie beim Start geprüft. Erst ein aktuelles positives Ergebnis erlaubt
Speichern, Start oder Firewalländerung. Der Dirty-State wird an den nativen
Appzustand gespiegelt, damit Fensterschließen und Tray-Beenden nicht am React-State
vorbei ungespeicherte Änderungen verwerfen. Die Bestätigung dafür bleibt von der
separaten Bestätigung zum Abbruch aktiver Übertragungen unabhängig.

Ein Dienststopp widerruft alle Sitzungen, signalisiert laufenden Downloads und Uploadjobs den Abbruch und wartet kontrolliert auf den HTTP-Server. Die über offene Handles eindeutig zugeordneten unvollständigen Uploaddateien werden anschließend dienstbesessen und exklusiv entfernt; ein bereits laufender Blocking-Job verzögert die Steuerung oder den Shutdown dabei nicht. Die öffentliche Markierung des `.ldtg`-Arbeitsordners beweist nicht die Eigentümerschaft einzelner Dateien. Nach einem Prozessabsturz bleiben deshalb nicht mehr zweifelsfrei zuordenbare `.part`-Dateien zur manuellen Prüfung erhalten; vorhandene Inhalte werden nie rekursiv gelöscht.

Zur Ressourcenbegrenzung sind gleichzeitig höchstens 12 Downloads insgesamt, 4 pro Client-IP und 3 pro Sitzung zulässig; jeder Download besitzt zusätzlich eine absolute Laufzeitgrenze von 6 Stunden. Es gelten höchstens 64 unvollständige Uploads insgesamt und 4 pro Client-IP. Das globale Inbox-Objektbudget reserviert beim Anlegen einen Platz; das globale Bytebudget wächst dagegen ausschließlich um erfolgreich geschriebene Blöcke und reserviert nicht die angekündigte Restgröße. Beide Budgets schließen bereits abgeschlossene Inbox-Dateien ein und werden beim Anlegen eines Uploads mit dem Dateisystem abgeglichen. Zusätzlich bleibt eine Datenträgerreserve von 1 GiB unangetastet. Ein Upload läuft nach 30 Minuten ohne erfolgreich gespeicherten Block oder spätestens nach 24 Stunden absolut ab. Vor dem Body-Puffern gilt genau ein aktiver Datenblock pro Upload-ID und ein globales Limit von 8 aktiven Uploadblöcken. Es gelten außerdem feste globale und IP-bezogene Grenzen für TCP-Verbindungen und gleichzeitig bearbeitete HTTP-Anfragen; inaktive Verbindungs-I/O, vollständige Requests und bereits das Lesen eines HTTP-Headers besitzen eigene Zeitlimits. Auch bei fortlaufendem I/O endet jede Verbindung spätestens nach 6 Stunden. Blockierende Ordnerarbeit besitzt einen Pool von 4 Jobs, höchstens 2 pro Client-IP und 1 pro Sitzung. Download-Pfad- und HEAD-Metadatenprüfungen besitzen ebenfalls 4 globale und 2 IP-bezogene Slots. Uploadanlage, Inbox-Scan, Speicherprüfung, Chunk-Persistierung und Abschluss besitzen einen eigenen Pool von 4 Jobs mit höchstens 2 pro Client-IP; interne Partial-Löschungen teilen dessen globale Kapazität. Permits gehören dem tatsächlichen Blocking-Job beziehungsweise dem ihn abschließenden dienstbesessenen Task und bleiben deshalb auch nach HTTP-Timeout oder Clientabbruch bis zum konsistenten Jobende gehalten.

Der Sitzungspool enthält höchstens 128 Sitzungen, davon höchstens 4 pro Client-IP. Eine Anmeldung oberhalb dieser Grenzen wird explizit abgewiesen; frische Sitzungen und Übertragungen werden niemals durch eine neue Anmeldung verdrängt. Sitzungen enden nach 6 Stunden 15 Minuten Inaktivität, nach 24 Stunden absolut, durch eigenen Logout, lokalen Widerruf oder Dienststopp. Monotone Zeitstempel verhindern Uhrzeitmanipulation; Prüfung, Entfernung und Neuaufnahme unterliegen demselben Mutex. Die anschließende ressourcengenaue Bereinigung läuft außerhalb dieses Mutex und kann keine inzwischen neu angelegte Sitzung erfassen. Serverseitige Ordnercursor sind an Sitzung, Client-IP, Pfad und Filter gebunden, auf 4 aktive Cursor pro Sitzung, 8 pro Client-IP und 64 global begrenzt, laufen nach kurzer Inaktivität ab und werden beim Sitzungsende entfernt. Eine gecachte Seite je Cursor macht Wiederholungen nach verlorenen HTTP-Antworten idempotent.

Eine Anmeldung kann optional einen maximal 64 Zeichen langen Gerätenamen für
genau diese Sitzung mitsenden. Das Backend trimmt ihn, lehnt Steuerzeichen und
bidirektionale Formatierungszeichen ab und persistiert ihn nicht. Der rohe
User-Agent wird nur lokal klassifiziert und anschließend verworfen; der
Desktopvertrag erhält stattdessen eine feste Bezeichnung wie „Safari auf
iPhone“. Transferdatensätze tragen intern die zugehörige Sitzungs-ID. Damit kann
der Desktop pro Gerät aktive Uploads und Downloads aus den bestehenden
Live-Ereignissen zählen, ohne den Gerätenamen in Transferdaten zu duplizieren.
Frei eingegebene Gerätenamen werden von React escaped und zusätzlich in einem
`bdi`-Element isoliert angezeigt.

Der Code bleibt innerhalb einer Dienstinstanz absichtlich wiederverwendbar, damit
mehrere legitime Geräte nacheinander eine eigene Sitzung anlegen können. Eine
lokale Rotation ersetzt den Code und leert die Fehlversuchszähler, widerruft
aber keine bereits erzeugte Sitzung. Gerätename und User-Agent sind keine
Rate-Limit- oder Autorisierungsidentität. Fehlversuche und anonyme Verbindungen
verwenden unter Windows nach Möglichkeit denselben aus der Nachbartabelle
abgeleiteten physischen Peer-Schlüssel; ohne auflösbaren Eintrag gilt die IP.
Sitzungskapazität und Übertragungslimits bleiben an Dienst, IP und gegebenenfalls
Sitzung gebunden. Ein bereits lokal geblockter Peer erhöht den globalen
Fehlversuchszähler mit weiteren IP-Aliasen nicht weiter.

Freigaberollen sind in v1 Eigenschaften des laufenden Dienstes. Jede Sitzung
sieht genau die global aktivierten Download- und Uploadwurzeln. Eine vom Client
selbst gewünschte Rolle würde keine Rechte begrenzen, weil derselbe Client auch
beide Rollen anfordern könnte. Individuelle Rollen erfordern deshalb zuerst
einen Desktop-bestätigten Pending-Pairing-Ablauf mit serverseitig
unveränderlichen Sitzungsfähigkeiten. R4.3 implementiert diese zusätzliche
Zustandsmaschine mangels belegten Bedarfs nicht; Entscheidung, Invarianten und
Pflichttests stehen in [`PAIRING_DESIGN.md`](PAIRING_DESIGN.md).

## Datenwege

- Download: begrenzte blockierende kanonische Pfad- und Metadatenprüfung → read-only Datei-Handle → gestreamte HTTP-Antwort mit Attachment- und Range-Headern.
- Upload: dienstbesessene Metadaten-, Inbox- und Speicherprüfung → neue `.part`-Datei → frühe Besitzprüfung vor PATCH-Bodyverbrauch → exakt 8 MiB pro Zwischenblock und genau die verbleibende Größe im letzten Block am jeweils bestätigten Offset → positionsfestes Schreiben und `sync_data` im Blocking-Pool → atomare Übernahme von Offset, Bytebudget und genau einem Progress-Update → dienstbesessener Commit → atomare No-Replace-Umbenennung auf einen immer zufällig suffigierten Zielnamen. Erst nach erfolgreichem `sync_data` bestätigt die PATCH-Antwort den neuen Offset. Das Verschwinden des HTTP-Waiters trennt weder Chunkjob noch Commit ab; Abbruch und Stop signalisieren sofort und lassen die exklusive Bereinigung nach der laufenden Dateiarbeit folgen.
- Desktopstatus: Der vollständige Status wartet asynchron auf echte Sitzungs-
  und Transfersnapshots und erfindet bei kurzer Sperrbelegung keine leeren
  Listen. Typisierte Sitzungs- und Transferereignisse tragen die Dienst-ID und
  werden bei passender Instanz direkt in den Desktopstatus eingearbeitet.
  Transferereignisse werden bei einem Fortschrittsupdate nach mindestens
  250 Millisekunden, nach 1 MiB zusätzlichem Fortschritt oder bei jedem
  Terminalzustand gesendet; der
  maßgebliche Backendzustand bleibt davon unabhängig bytegenau. Lifecycle- und
  Netzwerkereignisse führen zu einer gedrosselten Vollabfrage, ein
  30-Sekunden-Polling bleibt als Resynchronisierungsfallback. Zu jedem Transfer
  wird nur die kurzlebige Sitzungs-ID für die gerätebezogene Zuordnung ergänzt;
  keine Dateiinhalte oder rohen User-Agent-Header werden über IPC übertragen.

Jeder Backendtransfer hält neben dem aktuellen Bytezähler die Startzeit, den
Zeitpunkt des letzten echten Bytefortschritts, eine mit dem Faktor 0,25
exponentiell geglättete Rate und die Zahl der Messungen. Für Geschwindigkeiten
wird ausschließlich monotone Prozesszeit verwendet; die serialisierten
RFC-3339-Zeitstempel dienen Anzeige und Stabilitätsprüfung. Ein rückläufiger
Offset verwirft die bisherige Rate. Die Restzeit wird nicht als scheinbar
autoritatives Backenddatum gespeichert, sondern aus Restbytes und Rate
abgeleitet und erst nach drei Messungen über zwei Sekunden als stabil bewertet.
Nach fünf Sekunden ohne Fortschritt wird auch eine zuvor stabile Schätzung
wieder als instabil angezeigt.

## Netzgrenze

Der Server bindet nur an die ausgewählte private IPv4-Adresse. Jede Verbindung wird zusätzlich gegen deren Subnetz geprüft. Host und Origin müssen exakt zur laufenden lokalen URL passen. Ändert oder verschwindet die Schnittstelle, eine verankerte Freigabewurzel oder das bestätigte Windows-Netzwerkprofil bei gleichbleibender Adresse, wird der Dienst kontrolliert beendet. Netzwerkerkennung, Rootidentitätsprüfung und Freigabevorbereitung laufen außerhalb der Async-Worker. Der Monitor besitzt höchstens eine gemeinsame blockierende Umgebungsprüfung gleichzeitig; Accept und Shutdown bleiben währenddessen reaktionsfähig. Bei gleichzeitigem Shutdown und Prüfergebnis hat Shutdown Priorität, sodass kein verspätetes Netzwerkereignis mehr erzeugt wird. Nicht erhöhte PowerShell-Hilfsprozesse besitzen zusätzlich ein hartes Zeitlimit von 15 Sekunden und werden bei Überschreitung beendet.

Der achtstellige Zugangscode wird kryptografisch zufällig erzeugt. Neben zehn Fehlversuchen pro IP gilt ein gemeinsamer dienstweiter Fehlversuchshaushalt. Erreicht dieser seinen Schwellenwert, beginnt eine globale Abkühlphase, ohne dass der Code erneuert wird. Während dieser Phase werden alle Anmeldeversuche vor dem Codevergleich blockiert, einschließlich eines korrekten Codes. Fehlversuchsdatensätze besitzen TTL und feste Kapazität.

Die Mobile-App führt Uploads über eine einzige in-memory Warteschlange aus. Ein reiner Reducer ist die fachliche Quelle für Reihenfolge, ausstehende Einträge, Fortschritt, Server-ID und die Zustände `queued`, `uploading`, `paused`, `finalizing`, `complete`, `failed` und `cancelled`. Einzel- und Sammelaktionen ändern denselben Zustand atomar: Alle noch wartenden oder laufenden Einträge können pausiert, alle pausierten fortgesetzt, alle fehlgeschlagenen erneut angestellt und terminale Einträge entfernt werden. Nur wartende Dateien dürfen einzeln vollständig aus der Queue verschwinden. Der Batchfortschritt gewichtet die Einzelwerte nach Dateigröße und führt übertragene Bytes sowie terminale Dateien getrennt. Nur eine ausdrückliche Fortsetzung entfernt den Pausezustand. Statusabgleich, Chunks und sämtliche Retry-Verzögerungen sind abbrechbar, sodass Pause, Abbruch und Sitzungsverlust die nächste wartende Datei unmittelbar freigeben. Eine bereits gesendete Create-Anfrage wird auf Transportebene absichtlich nicht abgebrochen, weil der Server die Upload-ID schon angelegt haben kann; ihre Promise wird unabhängig von der aktuellen Queue-Arbeit weiterverfolgt und wiederverwendet. Dadurch kann die Queue sofort fortfahren, während eine spätere Antwort die Server-ID übernimmt oder bei bereits ausgelöstem Abbruch beziehungsweise bereits entferntem Queue-Eintrag ein Best-effort-Löschen anstößt. Nach Eintritt in den serverseitig linearisierbaren Zustand `finalizing` werden Pause und Abbruch nicht mehr angeboten. Weitere Dateiauswahlen werden während eines laufenden Uploads an dieselbe Queue angehängt.

Die mobile Queue misst Start, letzten Bytefortschritt und geglättete Rate mit
denselben Feldern wie der Desktopvertrag. Jede erfolgreiche Chunkantwort gleicht
den exakten Bytezähler ab. Resume und Retry beginnen für die Rate mit einer
frischen Stichprobe, ohne den bereits bestätigten Offset als neue
Geschwindigkeit auszugeben; ein serverseitig kleinerer Offset setzt die
Messreihe zurück.

Bei Sitzungsverlust bleiben lokale Dateireferenzen und Reihenfolge erhalten, während Server-IDs und Fortschrittswerte verworfen werden. Laufende und wartende Einträge werden nach erneuter Anmeldung neu angestellt; ausdrücklich pausierte bleiben pausiert. Eine im Queue-Zustand gehaltene und ausblendbare Meldung erklärt diese Wiederherstellung nur dann, wenn tatsächlich nichtterminale Uploads betroffen waren. Ein Seitenreload verliert die in-memory Dateiauswahl weiterhin bewusst.

XHR-Antworten des Chunk-Endpunkts werden wie Fetch-Antworten als strukturierte `ApiError`-Objekte ausgewertet. Nicht transiente 4xx-Antworten behalten ihren stabilen Fehlercode und schlagen ohne zusätzliche Backoff-Wartezeit fehl; Netzwerkfehler, 408, 409, 425, 429 und Serverfehler werden begrenzt wiederholt. Eine laufende Anmeldung besitzt einen lokalen Einmal-Guard. Beim Logout wird die lokale Sitzung auch dann im `finally`-Pfad entfernt, wenn der Dienst nicht mehr erreichbar ist.

Der Windows-Uninstaller entfernt mit einem strikt fehlschlagenden erhöhten
PowerShell-Kindprozess sowohl `LDTG Local Transfer` als auch den historischen
Namen `DMDC Local Transfer` und prüft anschließend deren Abwesenheit. UAC-Abbruch,
PowerShellfehler oder eine Restregel führen zu einer sichtbaren Retry-/Cancel-
Entscheidung und verhindern einen fälschlich erfolgreichen Abschluss.
Konfiguration, Logs und mögliche Nutzdaten in AppData und Freigaben bleiben
erhalten; dort erfolgt keine rekursive Löschung.

Download- und Uploadwurzeln werden kanonisch auf vollständige Trennung geprüft. Benutzer- und systemweite Windows-Autostartpfade sind als Uploadziele gesperrt. Uploadnamen werden vor der Übertragung nach UTF-16-Komponenten- und Gesamtpfadbudget begrenzt; auch Kollisionssuffixe bleiben innerhalb dieser Grenze.

Die Windows-Firewallregel ist auf Programmpfad, TCP-Port und `LocalSubnet` begrenzt, gilt aber bewusst in allen Windows-Profilkategorien. Ein als „Öffentlich“ klassifiziertes Heimnetz blockiert den Start daher nicht; die LDTG-eigene Vertrauensbestätigung bleibt maßgeblich.

Der lokale Diagnoseexport projiziert Erstellungszeit, App-/Schemaversion,
Plattform, Dienstzustand, Zähler, Boolesche Werte, Port-/Limitwerte,
aggregierte Netzwerk-/Firewallzustände und einen konstanten Datenschutzhinweis. Freigabepfade,
IP-Adressen, Adapter- und Netzwerkkennungen, Netzwerk- und Dateinamen,
Programmpfade, Codes, Tokens, Dateilisten und rohe Fehlerdetails werden nicht in
den Bericht übernommen. Der vollständige Datenlebenszyklus steht in
[`PRIVACY.md`](PRIVACY.md).
