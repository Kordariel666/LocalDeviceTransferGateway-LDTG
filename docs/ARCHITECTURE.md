# Architektur von DMDC v1

## Grenzen und Module

Die Desktop-WebView ist eine reine Steueroberfläche. Sie besitzt nur eng definierte Tauri-Befehle für Einstellungen, Dienst, Sitzungen, Firewall und Diagnose. Allgemeine Shell- oder Dateisystemrechte sind nicht freigegeben.

Der Rust-Code ist in drei Schichten gegliedert:

- `domain`: persistierbare Einstellungen, Netzwerkerkennung, Pfad- und Dateinamenregeln sowie UI-Vertragstypen.
- `service`: kurzlebiger Dienstzustand, Authentifizierung, Sitzungen, Übertragungen und die Axum-HTTP-API.
- `platform`: betriebssystemspezifische Funktionen. In v1 ist nur der Windows-Firewallhelfer implementiert.

Die mobile React-App wird separat gebaut und mit `rust-embed` in die Rust-Binärdatei aufgenommen. Dadurch benötigt DMDC weder Internet noch einen separaten Webserver.

## Zustandsmodell

Konfiguration ist nur im Zustand `stopped` änderbar. Start, Stop, Quit, Status-Reaping, Speichern und Firewallkonfiguration teilen einen Lifecycle-Transition-Mutex; ein zweiter Übergang kann daher weder einen noch nicht veröffentlichten Start überholen noch den Zustand einer neueren Dienstinstanz überschreiben. Jeder Übergang nach `running` erzeugt eine neue Dienst-ID und einen neuen Zugangscode. Sitzungen, Uploadzuordnungen und Übertragungsverlauf leben nur in dieser Instanz.

Ein Dienststopp widerruft alle Sitzungen, signalisiert laufenden Downloads den Abbruch, wartet kontrolliert auf den HTTP-Server und entfernt eigene unvollständige Uploaddateien. Der `.dmdc`-Arbeitsordner trägt eine eindeutige DMDC-Besitzmarkierung; nach einem Absturz entfernt die nächste Dienstinstanz ausschließlich reguläre UUID-`.part`-Dateien aus diesem markierten Ordner. Fremde oder nicht eindeutig markierte Inhalte werden nie rekursiv gelöscht.

Zur Ressourcenbegrenzung sind gleichzeitig höchstens 12 Downloads insgesamt, 4 pro Client-IP und 3 pro Sitzung zulässig; jeder Download besitzt zusätzlich eine absolute Laufzeitgrenze von 6 Stunden. Pro Client-IP sind höchstens 4 unvollständige Uploads reserviert. Restgrößen unterliegen zusätzlich Bytebudgets pro IP und Sitzung: Eine IP kann höchstens die Hälfte des noch nutzbaren Datenträgers reservieren, wobei eine einzelne konfigurativ zulässige Datei möglich bleibt; eine Sitzung ist auf eine konfigurativ zulässige Datei beziehungsweise bei unbegrenzter Dateigröße auf die Hälfte des IP-Budgets begrenzt. Ein Upload läuft nach 30 Minuten ohne erfolgreich gespeicherten Block oder spätestens nach 24 Stunden absolut ab. Vor dem Body-Puffern gilt genau ein aktiver Datenblock pro Upload-ID und ein globales Limit von 8 aktiven Uploadblöcken. Es gelten außerdem feste globale und IP-bezogene Grenzen für TCP-Verbindungen und gleichzeitig bearbeitete HTTP-Anfragen; inaktive Verbindungs-I/O, vollständige Requests und bereits das Lesen eines HTTP-Headers besitzen eigene Zeitlimits. Auch bei fortlaufendem I/O endet jede Verbindung spätestens nach 6 Stunden. Blockierende Ordnerarbeit besitzt einen Pool von 4 Jobs, höchstens 2 pro Client-IP und 1 pro Sitzung. Download-Pfad-, HEAD-Metadaten- und Upload-Speicherprüfungen besitzen zusätzlich einen Pool von 4 Jobs, höchstens 2 pro Client-IP. Ein Permit wird in den tatsächlichen Blocking-Job verschoben und bleibt deshalb auch nach HTTP-Timeout oder Clientabbruch bis zu dessen Ende gehalten.

Der Sitzungspool enthält höchstens 128 Sitzungen, davon höchstens 4 pro Client-IP. Eine Anmeldung oberhalb dieser Grenzen wird explizit abgewiesen; frische Sitzungen und Übertragungen werden niemals durch eine neue Anmeldung verdrängt. Sitzungen enden nach 6 Stunden 15 Minuten Inaktivität, nach 24 Stunden absolut, durch eigenen Logout, lokalen Widerruf oder Dienststopp. Monotone Zeitstempel verhindern Uhrzeitmanipulation; Prüfung, Entfernung und Neuaufnahme unterliegen demselben Mutex. Die anschließende ressourcengenaue Bereinigung läuft außerhalb dieses Mutex und kann keine inzwischen neu angelegte Sitzung erfassen. Serverseitige Ordnercursor sind an Sitzung, Client-IP, Pfad und Filter gebunden, auf 4 aktive Cursor pro Sitzung, 8 pro Client-IP und 64 global begrenzt, laufen nach kurzer Inaktivität ab und werden beim Sitzungsende entfernt. Eine gecachte Seite je Cursor macht Wiederholungen nach verlorenen HTTP-Antworten idempotent.

## Datenwege

- Download: begrenzte blockierende kanonische Pfad- und Metadatenprüfung → read-only Datei-Handle → gestreamte HTTP-Antwort mit Attachment- und Range-Headern.
- Upload: Metadaten- und begrenzte Speicherprüfung → neue `.part`-Datei → frühe Besitzprüfung vor PATCH-Bodyverbrauch → beliebige nicht leere Blöcke bis maximal 8 MiB am jeweils exakt bestätigten Offset → Größen- und Datenträgersynchronisation → dienstbesessener Commit → atomare No-Replace-Umbenennung auf einen immer zufällig suffigierten Zielnamen. Hat der Commit linearisiert, warten Abbruch, Widerruf und Stop auf dessen Ergebnis; das Verschwinden des HTTP-Waiters trennt den Commit nicht ab.
- Desktopstatus: kleine Metadatenereignisse und typisierte Tauri-Aufrufe. Keine Dateiinhalte über IPC.

## Netzgrenze

Der Server bindet nur an die ausgewählte private IPv4-Adresse. Jede Verbindung wird zusätzlich gegen deren Subnetz geprüft. Host und Origin müssen exakt zur laufenden lokalen URL passen. Ändert oder verschwindet die Schnittstelle oder wechselt das bestätigte Windows-Netzwerkprofil bei gleichbleibender Adresse, wird der Dienst kontrolliert beendet. Netzwerkerkennung und Freigabevorbereitung laufen außerhalb der Async-Worker. Der Monitor besitzt höchstens eine reine blockierende Netzwerkprüfung gleichzeitig; Accept und Shutdown bleiben währenddessen reaktionsfähig. Bei gleichzeitigem Shutdown und Prüfergebnis hat Shutdown Priorität, sodass kein verspätetes Netzwerkereignis mehr erzeugt wird. Nicht erhöhte PowerShell-Hilfsprozesse besitzen zusätzlich ein hartes Zeitlimit von 15 Sekunden und werden bei Überschreitung beendet.

Der achtstellige Zugangscode wird kryptografisch zufällig erzeugt. Neben zehn Fehlversuchen pro IP gilt ein gemeinsamer dienstweiter Fehlversuchshaushalt. Erreicht dieser seinen Schwellenwert, wird der Code erneuert; während der globalen Abkühlphase werden weitere falsche Versuche blockiert, der aktuelle korrekte Code bleibt für nicht bereits IP-blockierte Geräte verwendbar. Fehlversuchsdatensätze besitzen TTL und feste Kapazität.

Die Mobile-App führt Uploads über eine einzige in-memory Warteschlange aus. Nur eine ausdrückliche Fortsetzung entfernt den Pausezustand. Eine bereits gesendete Create-Anfrage wird absichtlich nicht abgebrochen, weil der Server die Upload-ID schon angelegt haben kann: Die Antwort wird zuerst übernommen, danach greift Pause beziehungsweise ein nachträgliches Best-effort-Löschen bei Abbruch. Retry-Backoff, Statusabgleich und jeder Chunk sind abbrechbar. Nach Eintritt in den serverseitig linearisierbaren Zustand `finalizing` werden Pause und Abbruch nicht mehr angeboten. Weitere Dateiauswahlen werden während eines laufenden Uploads an dieselbe Queue angehängt.

Der Windows-Uninstaller entfernt weiterhin die DMDC-Firewallregel, bewahrt jedoch Konfiguration, Logs und mögliche Nutzdaten in den AppData-Verzeichnissen. Er führt dort keine rekursive Löschung aus.

Download- und Uploadwurzeln werden kanonisch auf vollständige Trennung geprüft. Benutzer- und systemweite Windows-Autostartpfade sind als Uploadziele gesperrt. Uploadnamen werden vor der Übertragung nach UTF-16-Komponenten- und Gesamtpfadbudget begrenzt; auch Kollisionssuffixe bleiben innerhalb dieser Grenze.

Die Windows-Firewallregel ist auf Programmpfad, TCP-Port und `LocalSubnet` begrenzt, gilt aber bewusst in allen Windows-Profilkategorien. Ein als „Öffentlich“ klassifiziertes Heimnetz blockiert den Start daher nicht; die DMDC-eigene Vertrauensbestätigung bleibt maßgeblich.
