# P4 – Realsystem- und Gerätematrix

Stand: 4. September 2026
Status: **P4-Kernpfade bestanden; verbleibende manuelle Stress- und Plattformlücken vom Owner für die Beta ausdrücklich akzeptiert**

Diese Matrix markiert kein Feld ohne reale Ausführung als bestanden. `PB-06`
autorisierte den kontrollierten Systemtest. Installation und grundlegender
Funktionslauf wurden ausgeführt. `P4-FW-01` wurde nach einer unabhängigen
Neuinstallation und erhöhten Regelprüfung als Testumgebungsartefakt geschlossen.
Upgrade-, Neuinstallations-, Barrierefreiheits- und weitere Mobiltests bleiben offen. Verfügbare
iOS-/iPadOS- und Androidgeräte werden nach dem Windows-Pfad separat bestätigt.
Der anschließend gemeldete Microsoft-Defender-Fund `Trojan:Win32/Bearfoos.A!ml`
disqualifiziert das zuvor getestete P3-Artefakt. Dessen frühere Funktionsbelege
bleiben Beobachtungen, gelten aber nicht als Freigabe des neuen Builds.

## Vorprüfung des Windows-Hosts

Der lokale Host meldet Version `25H2`, Build `26200.9168`, Edition
`Professional`, Installationstyp `Client` und Architektur `AMD64`. Microsoft
führt Build `26200.9168` als Windows 11 25H2 im General Availability Channel:
<https://learn.microsoft.com/en-us/windows/release-health/windows11-release-information>.
Das historische Registryfeld `ProductName` enthält trotzdem `Windows 10 Pro`
und wird deshalb nicht als maßgebliche Versionsquelle verwendet.

Die erste nicht erhöhte Vorprüfung meldete:

- kein registrierter LDTG-/DMDC-Uninstall-Eintrag;
- keine Firewallregel `LDTG Local Transfer` oder `DMDC Local Transfer`;
- ein vorhandenes aktuelles `%LOCALAPPDATA%\de.ldtg.desktop`-Verzeichnis;
- vorhandene historische `%APPDATA%`-/`%LOCALAPPDATA%`-DMDC-Verzeichnisse.

Die Firewall-Aussage war nicht belastbar: Eine spätere explizite Abfrage zeigte,
dass der nicht erhöhte Prozess für `Get-NetFirewallRule` nur „Zugriff
verweigert“ erhielt; der ursprüngliche `SilentlyContinue`-Pfad hatte das
maskiert. Die erhöhte Belegprüfung nach dem Test wird deshalb unten getrennt
dokumentiert. Die AppData-Inhalte wurden nicht gelesen oder verändert. Sie
gelten für P4 als zu erhaltende Nutzerdaten. Sentinel-Dateien wurden noch nicht
angelegt.

## Windows-Matrix

| ID | Reale Prüfung | Erwartung | Status/Beleg |
|---|---|---|---|
| W-01 | Installer starten, erste UAC-Abfrage abbrechen | keine Installation, keine neue Firewallregel, bestehende Daten unverändert | **Testannahme widerlegt:** Der Current-User-Installer verlangte keine UAC. Installation selbst erzeugte keine neue Firewallregel. Ein sinnvoller UAC-Abbruchtest bleibt für Firewall/Uninstall. |
| W-02 | Unsignierten x64-Installer aus P3 installieren | Warnung sachlich erfassen; Version `0.3.0-rc.1`; App startet | **AV-Härtungsretest bestanden:** Der hashgeprüfte neue Installer registrierte `0.3.0-rc.1` unter HKCU und startete ohne Defender-Meldung aus dem realen `%LOCALAPPDATA%\LDTG\ldtg.exe`-Pfad. Installiert sind nur Haupt-EXE, fester Firewall-Cleanup-Helfer und Uninstaller; der Vertragsgenerator fehlt. Der frühere P3-Build bleibt wegen seines Defender-Funds disqualifiziert. Kein SmartScreen-Reputationstest, da das Artefakt lokal gebaut wurde. |
| W-03 | Dienst in ausdrücklich bestätigtem privaten LAN starten | bewusste Netzbestätigung; nur aktuelle LDTG-Inboundregel; kein Internetversprechen | **AV-Härtungsretest bestanden:** Firewallkonfiguration und Dienststart auf Port 8765 liefen ohne Defender-Meldung oder Appfehler. Die lesende Nachprüfung bestätigte exakt eine enge Regel am realen Installationspfad und den LDTG-eigenen Listener an der privaten LAN-Adresse; als Kindprozess lief nur Microsoft Edge WebView, kein PowerShell-Prozess und keine breite Zusatzregel. |
| W-04 | Port ändern und Dienst erneut starten/stoppen | Regel folgt dem aktuellen Programm-/Portzustand; Dienst stoppt sauber | **AV-Härtungsretest bestanden:** Start/Stopp auf 8765, Regelwechsel auf 8876, mobiler Zugriff und abschließender Stopp auf 8876 funktionierten ohne Defender-Hinweis. Nach dem letzten Stopp waren beide Ports geschlossen und keine Dienstverbindung blieb bestehen; App, genau eine enge 8876-Regel und ein fehlerfreies Log blieben erhalten. |
| W-05 | Upgrade von geeignetem früherem RC auf P3-Build | Einstellungen migrieren; Nutzdaten und Freigaben bleiben erhalten | **für die erste öffentliche Beta nicht separat ausgeführt:** Es existiert kein früherer öffentlicher LDTG-Build. Der reale Reparatur-/Neuinstallationspfad erhielt Einstellungen und Freigaben; Schema-v1-bis-v4-Migrationen sind automatisiert geprüft. Als Beta-Evidenzlücke akzeptiert, nicht als realer Upgrade-Bestehensbeleg. |
| W-06 | Uninstall-UAC abbrechen | Deinstallation bricht sichtbar ab; Regel und Installation werden nicht halb entfernt | **bestanden:** Der Operator brach die erhöhte Firewallbereinigung und anschließend die Deinstallation ab. Die Nachprüfung bestätigte die vollständige Installation mit Haupt-EXE, Cleanup-Helfer und Uninstaller, den HKCU-Eintrag sowie die unveränderte enge 8876-Regel. Kein relevanter Prozess und kein Testport blieb aktiv. |
| W-07 | Deinstallieren mit aktuellen und historischen Sentinel-Daten | beide Firewallnamen fehlen danach; AppData/Freigaben/Sentinels bleiben erhalten | **bestanden mit korrigiertem Installer:** Reparaturinstallation stellte `LDTG 0.3.0-rc.1` in „Installierte Apps“ wieder her. Die neu erzeugte enge 8876-Regel wurde vor dem Uninstall exakt bestätigt. Die Deinstallation mit akzeptierter UAC endete ohne Fehler; Programmordner, Programmdateien, Eintrag, Verknüpfungen, Prozesse, Testportverbindungen und Produktregeln fehlten danach. Beide AppData-Verzeichnisse sowie die manuell geprüften IN-/OUT-Testdateien blieben erhalten. |
| W-08 | Neuinstallation nach Deinstallation | sauberer Start ohne Datenverlust; erhaltene Konfiguration wird kontrolliert behandelt | **bestanden:** Der korrigierte Installer wurde nach dem vollständigen Uninstall erneut aus dem Datei-Explorer installiert. LDTG startete sauber; Freigabeordner, bestätigtes Netzwerk, Port 8876 und weitere Einstellungen waren unverändert vorhanden. Die Installation allein erzeugte keine Firewallregel und keinen Listener; beide AppData-Verzeichnisse blieben erhalten. |
| W-09 | 200-%-Skalierung, Tastatur und Reduced Motion | Kernfluss bedienbar, Fokus sichtbar, kein unzugänglicher Dialog | **nicht ausgeführt; vorläufig akzeptiertes Restrisiko:** Der Owner verzichtet in diesem Lauf auf den manuellen Test und nimmt vorläufig an, dass die Pfade funktionieren. Die Annahme gilt nicht als Bestehensbeleg und muss vor einem späteren öffentlichen Gate erneut entschieden oder real geprüft werden. |

## `P4-FW-01` – zusätzliche breite Windows-Regeln

Der Operator richtete zuerst in LDTG die Firewall ein und bestätigte danach beim
Dienststart einen weiteren, als Firewalländerung wahrgenommenen Windows-Schritt.
Der grundlegende Appfluss funktionierte. Eine erhöhte, lesende Abfrage des
`PersistentStore` bestätigte anschließend sechs namens- oder pfadbezogene
Regeln:

- genau eine vorgesehene `LDTG Local Transfer`-Regel für den installierten
  LDTG-Programmpfad: Inbound, Allow, TCP 8765, `RemoteAddress=LocalSubnet`,
  `Profile=Any`, Edge Traversal blockiert;
- zwei automatisch von Windows erzeugte `ldtg.exe`-Regeln für den virtualisierten
  Codex-`LocalCache`-Pfad: je eine TCP- und UDP-Inbound-Allow-Regel mit allen
  lokalen Ports, allen Gegenstellen, Profil `Public` und nicht strikt
  blockiertem Edge Traversal;
- eine historische enge `DMDC Local Transfer`-Regel und zwei breite historische
  `dmdc.exe`-Regeln. Deren Entstehungszeit ist aus dem Lauf nicht beweisbar; sie
  wurden deshalb nicht verändert und sind nicht Teil des LDTG-Cleanups.

Der laufende Prozess meldete als physisches Executable den installierten
`%LOCALAPPDATA%\LDTG\ldtg.exe`-Pfad; der Computer-Use-/Windows-Regelkontext führte
ihn zugleich unter dem virtualisierten Codex-`LocalCache`-Pfad. Beide sichtbaren
Dateipfade hatten identische Größe, Zeitstempel und SHA-256. Der Quellpfad für
„Firewall einrichten“ verwendet `current_exe()` und erzeugt ausschließlich die
enge benannte Regel. Der Dienststart ruft keine Firewallmutation auf, sondern
prüft Regelstatus, Netzwerkvertrauen und Freigaben. Damit ist der zusätzliche
Dialog konsistent mit einer Windows-Autoregel infolge des paketierten
Elternprozesses, nicht mit einem zweiten LDTG-Konfigurationsaufruf.

Nach manuellem Entfernen der beiden breiten LDTG-Regeln beendete der Operator
die App. Ein Startversuch über den unabhängigen Datei-Explorer bestätigte, dass
`%LOCALAPPDATA%\LDTG\ldtg.exe` außerhalb des Codex-Paketkontexts nicht existiert;
Explorer meldete „Datei wurde nicht gefunden“. Die erste Installation ist damit
als paketvirtualisiertes Testartefakt und nicht als gültige Hostinstallation
klassifiziert. Für den Retest muss der private Installer manuell aus dem
Datei-Explorer gestartet werden.

Ein exakt auf die zwei neuen LDTG-Autoregeln begrenzter Cleanupversuch scheiterte
mit „Zugriff verweigert“; er änderte keine Regel. `PB-07` verlangt daher:

1. die zwei breiten `ldtg.exe`-Regeln für den Codex-`LocalCache`-Pfad manuell mit
   Administratorrechten entfernen;
2. LDTG vollständig beenden und den privaten Installer manuell aus dem
   Datei-Explorer starten, damit keine Codex-Paketvirtualisierung vererbt wird;
3. LDTG über die dadurch erzeugte installierte Verknüpfung oder den realen
   `%LOCALAPPDATA%\LDTG\ldtg.exe`-Pfad starten, enge Regel prüfen/einrichten und
   Dienst starten;
4. bei erneutem Windows-Angebot für breite Appregeln abbrechen und als
   reproduzierbaren Produktbefund behandeln;
5. bei ausbleibendem Zusatzdialog erhöht nachweisen, dass genau die enge
   LDTG-Regel existiert.

**Abschluss am 3. September 2026:** Der Operator entfernte die beiden breiten
LDTG-Autoregeln manuell. Danach wurde derselbe private Installer aus dem
unabhängigen Datei-Explorer gestartet. Windows registrierte LDTG `0.3.0-rc.1`
unter HKCU; Installation und laufender Prozess verwenden den realen
`%LOCALAPPDATA%\LDTG\ldtg.exe`-Pfad. Der wiederholte Firewall-/Dienstfluss
funktionierte ohne zweiten allgemeinen Windows-Firewalldialog. Eine erhöhte
`PersistentStore`-Abfrage fand genau eine LDTG-Regel mit dem vorgesehenen engen
Programm-, Port-, Protokoll-, Subnetz- und Edge-Traversal-Umfang. `P4-FW-01` und
`PB-07` sind damit geschlossen. Der Befund war ein Artefakt des ersten
paketvirtualisierten Codex-Starts; der unabhängige Produktpfad reproduzierte ihn
nicht.

## `P4-AV-01` / `PB-08` – Defender-Quarantäne

Microsoft Defender meldete am 3. September 2026 um 20:45 Uhr und erneut um
20:49 Uhr `Trojan:Win32/Bearfoos.A!ml` mit Schweregrad „Schwerwiegend“. Betroffen
waren die installierte beziehungsweise paketvirtualisierte `ldtg.exe` sowie die
zugehörigen Start- und Deinstallationseinträge. Der Operator entfernte beide
Funde; die danach ausgeführte Schnellüberprüfung fand nichts Weiteres. Es wurde
keine Defender-Ausnahme angelegt und keine Datei aus der Quarantäne
wiederhergestellt.

Die statische Prüfung fand keinen absichtlich eingebauten Trojaner und keinen
LAN-Pfad zu beliebiger Befehlsausführung. Der alte Windows-Laufzeitpfad
kombinierte jedoch eine unsignierte NSIS/Tauri-Anwendung mit versteckten,
Base64-kodierten PowerShell-Aufrufen für Firewall- und Netzwerkabfragen. Zudem
wurde der reine Buildgenerator `generate-contracts.exe` aufgrund automatischer
Cargo-Bin-Erkennung unnötig mitgebaut. Microsofts genaue ML-Begründung ist nicht
lokal einsehbar; diese Eigenschaften sind daher eine begründete Heuristik-,
keine Ursachenbehauptung.

Der AV-Härtungskandidat ersetzt sämtliche installierten PowerShell-Aufrufe durch
native Windows-COM-Schnittstellen, prüft Edge Traversal exakt als `DENY`, macht
den Vertragsgenerator zu einem nicht paketierten Cargo-Beispiel und begrenzt
die Paket-Binaries auf `ldtg.exe` sowie den parameterlosen, fest verdrahteten
`ldtg-firewall-cleanup.exe`. Der separate Helfer hält die Deinstallation auch
dann bereinigungsfähig, wenn Defender nur die Haupt-EXE entfernt. `pnpm check`
bestand einschließlich 36 Desktop-, 39 Mobile- und 122 Rust-Tests. Der neu
erzeugte private Testinstaller hat SHA-256
`646D658FA831B524ED6C3D84C19FCE364661D45388CE9BAC4263039184DE8482` und bleibt
unsigniert.

`PB-08` bleibt offen, bis der Operator die zwei alten Restordner und – nach
erhöhter Kontrolle – beide exakten Produktregeln entfernt, den neuen Installer
manuell mit Defender prüft, installiert und die Firewall-/Dienst-/Uninstall-
Pfade ohne erneuten Fund durchläuft. Das bloße erfolgreiche Erzeugen des Builds
und sein Fortbestand nach dem Build sind kein Malware-Freigabenachweis.

**Retest-Vorbereitung:** Der Operator entfernte beide Restordner und die alten
Produktregeln und prüfte den neuen Installer gezielt mit Microsoft Defender;
Defender meldete keinen Fund. Die anschließende unabhängige, nur lesende
Kontrolle bestätigte beide fehlenden Ordner, null Regeln mit den exakten Namen
`LDTG Local Transfer` oder `DMDC Local Transfer` und weiterhin den erwarteten
Installer-SHA-256
`646D658FA831B524ED6C3D84C19FCE364661D45388CE9BAC4263039184DE8482`.
Installation, Laufzeit und Deinstallation des neuen Builds bleiben offen.

**Installation und erster Start:** Der Operator installierte denselben
hashgeprüften AV-Härtungskandidaten aus dem Datei-Explorer und startete ihn ohne
Defender-Meldung. Die lesende Nachprüfung bestätigte den Prozesspfad
`%LOCALAPPDATA%\LDTG\ldtg.exe`, den HKCU-Deinstallationseintrag für
`0.3.0-rc.1`, den erwarteten Cleanup-Helfer mit SHA-256
`50ED9F5A61D0A2FB854437FDA84CF0B66F0353C04C0E3591C32BC6FFFA5F0389`, keinen
installierten `generate-contracts.exe`, noch keine Produktfirewallregel und
keinen Listener auf den LDTG-Testports. Firewallmutation, Dienstlauf und
Deinstallation bleiben für `PB-08` offen.

**Firewallkonfiguration:** Der Operator richtete die Firewallregel des neuen
Builds auf Port 8765 ohne Fehler ein. Unmittelbar danach bestätigte eine erhöhte,
nur lesende COM-Prüfung genau eine verwandte Regel: `LDTG Local Transfer` für
den realen Installationspfad, TCP 8765, `RemoteAddresses=LocalSubnet`,
`Profiles=All`, Inbound, Allow, Enabled und `EdgeTraversalOptions=DENY`. Es gab
keine Legacy- oder breite automatische Zusatzregel und noch keinen Listener auf
Port 8765. Der isolierte Dienststart bleibt offen.

**Dienststart:** Der Operator startete anschließend ausschließlich den Dienst
und meldete den laufenden Zustand ohne Defender-Hinweis. Die lesende
Nachprüfung ordnete den Listener auf TCP 8765 demselben installierten
`ldtg.exe`-Prozess an der privaten LAN-Adresse zu. Die enge Regel blieb
unverändert; als einziger Kindprozess lief die erwartete Microsoft Edge WebView,
kein PowerShell-Prozess. Damit ist W-03 für den AV-Härtungskandidaten bestanden.

**Dienststopp:** Der Operator stoppte den Dienst nach den erfolgreichen mobilen
Transfers. Die Nachprüfung fand keinen Listener und keine verbleibende
LDTG-Verbindung auf Port 8765. Der Desktop-Prozess lief weiter, die enge
Firewallregel blieb erwartungsgemäß bestehen und das Log enthielt keine Warnung
oder Fehlermeldung. Der Portwechselteil von W-04 bleibt offen.

**Portwechsel:** Bei gestopptem Dienst änderte der Operator den Port von 8765
auf 8876 und richtete die Firewall ohne Defender-Hinweis erneut ein. Die
unmittelbare lesende Nachprüfung fand keine alte 8765-Regel, genau eine enge
8876-Regel mit unverändertem Programm-, Subnetz-, Profil- und Edge-Traversal-
Umfang und noch keinen Listener auf einem der beiden Testports. Start und Stopp
auf 8876 bleiben offen.

**Neustart auf dem neuen Port:** Der Dienst startete ohne Defender-Meldung auf
8876 und war vom Android-Gerät erreichbar. Die Nachprüfung ordnete genau einen
8876-Listener dem installierten LDTG-Prozess zu, fand keinen Listener auf 8765,
weiterhin nur die enge 8876-Regel, nur Microsoft Edge WebView als Kindprozess
und keine Warnung oder Fehlermeldung im aktuellen Log. Der abschließende Stopp
auf 8876 bleibt offen.

**Abschließender Stopp auf 8876:** Der Operator stoppte den Dienst erneut. Die
Nachprüfung bestätigte null Listener und null verbleibende Dienstverbindungen
auf 8765 oder 8876. Der Desktop-Prozess und genau die enge 8876-Regel blieben
erhalten; das aktuelle Log blieb ohne Warnung oder Fehler. Damit ist W-04 für
den AV-Härtungskandidaten bestanden.

**Uninstall-Abbruch:** Nach geschlossenem Programm startete der Operator die
Deinstallation, lehnte die Administratorabfrage des Cleanup-Helfers ab und
brach den anschließenden Retry-/Cancel-Pfad ab. Die Prüfung bestätigte den
atomaren Zustand: Installationsordner mit genau den drei erwarteten EXE-Dateien,
HKCU-Deinstallationseintrag und enge 8876-Regel blieben erhalten; relevante
Prozesse und Listener fehlten. Als Vorherbeleg für W-07 wurden ohne Ausgabe von
Dateinamen oder Inhalten aggregierte Bestandsfingerabdrücke aufgenommen:
Local-Appdaten 350 Dateien / 41.579.070 Bytes /
`C4B2DE1E93DA85B52D43AA02757150D683752CF983F3CBBADAF2ED3F80DB3159`,
Roaming-Appdaten 1 Datei / 884 Bytes /
`8FF9709605C47FDCEFE3487B5164F3A6BA2AD0E95A59AC31CD9CAA4DAB57651E`.

## `P4-UN-01` / `PB-09` – falscher Fehler nach erfolgreichem Cleanup

Beim anschließenden vollständigen Deinstallationsversuch bestätigte der Operator
die Administratorabfrage des Firewall-Cleanup-Helfers. Der Uninstaller zeigte
trotzdem den Retry-/Cancel-Fehler an; der Operator brach daraufhin korrekt ab.
Die erste, nur lesende Prüfung belegte, dass die Produktfirewallregel bereits
entfernt war, während Installationsordner und alle drei erwarteten
Programmdateien erhalten blieben. Ein späterer Endzustandscheck fand keinen
HKCU-Deinstallationseintrag mehr; der alte Ablauf hinterließ damit trotz
Abbruch eine reparierbare Teilinstallation. Es gab weder einen relevanten
Prozess noch ein Anwendungsabsturz- oder Defender-Ereignis.

Ursache ist die bisherige Auswertung von `$0` nach dem eingebauten NSIS-Befehl
`ExecShellWait`: Dieser Befehl wartet auf den erhöhten Prozess, stellt aber
keinen Exitcode in einer Benutzervariable bereit. Der Hook interpretierte daher
einen alten beziehungsweise undefinierten Wert als Cleanupfehler, obwohl der
fest verdrahtete Helfer erfolgreich gearbeitet hatte.

Der korrigierte Hook startet den Cleanup weiterhin mit `ExecShellWait` und
`runas`. Danach führt er denselben parameterlosen Helfer in einem neuen,
nicht erhöhten Prozess mit `ExecWait` aus und wertet dessen echten Exitcode als
Postcondition aus: Sind keine Produktregeln mehr vorhanden, endet die Prüfung
mit `0`; verbliebene, nicht entfernbare Regeln lassen die Deinstallation
weiterhin geschlossen fehlschlagen.

**Korrigierter Build:** Der vollständige Projektcheck bestand mit 36 Desktop-,
39 Mobile- und 122 Rust-Tests sowie Typecheck, Lint, Webbuild, Formatprüfung und
Clippy. Der danach frisch erzeugte, weiterhin unsignierte private NSIS-Installer
hat SHA-256
`A96D52ECABEDD2D660581740587AE5D5BF19750C825BB8DB85989C0DFBCA1D98`.
Dieses Artefakt ersetzt ausschließlich für den nächsten Retest den vorherigen
Härtungsinstaller; ein bestandener Realtest ist damit noch nicht vorweggenommen.

**Reparaturinstallation:** Der Operator prüfte den korrigierten Installer und
installierte ihn über die nach dem abgebrochenen Alt-Uninstall verbliebenen
Programmdateien. Installation und anschließender Programmstart funktionierten.
Die native Windows-Seite „Installierte Apps“ zeigte danach wieder den Eintrag
`LDTG`, Version `0.3.0-rc.1`, mit Installationsdatum 3. September 2026. Eine aus
dem paketierten Prüfprozess ausgeführte Registry-Abfrage sah diesen Eintrag
nicht und wird deshalb nicht als Gegenbeleg verwendet.

**Erfolgreicher Abschluss:** Bei gestopptem Dienst erzeugte der Operator erneut
eine Regel für Port 8876. Die lesende COM-Prüfung bestätigte exakt eine enge
LDTG-Regel für den realen Programmpfad, TCP, `LocalSubnet`, alle Profile und
blockiertes Edge Traversal; es gab weder eine alte 8765- noch eine breite
Zusatzregel. Der anschließende Uninstall mit akzeptierter UAC endete ohne
Fehlermeldung. Die Nachprüfung fand weder Installationsordner und Programmdateien
noch Produktregeln, Verknüpfungen, Prozesse oder Testportverbindungen. Beide
AppData-Verzeichnisse blieben vorhanden; der Roaming-Bestand blieb bei einer
Datei und 884 Bytes. Der Operator bestätigte zusätzlich, dass LDTG aus
„Installierte Apps“ verschwunden und die zuvor verwendeten IN-/OUT-Testdateien
weiterhin vorhanden sind. Damit sind W-07, `PB-09` und der vollständige
Defender-Retest `PB-08` bestanden.

**Neuinstallation nach erfolgreichem Uninstall:** Der Operator installierte
denselben korrigierten Build erneut aus dem Datei-Explorer und bestätigte nach
dem Start, dass Freigabeordner, Netzwerk, Port 8876 und weitere Einstellungen
erhalten geblieben waren. Die lesende Nachprüfung fand den Installationsordner
und beide AppData-Verzeichnisse, aber erwartungsgemäß keine Produktfirewallregel
und keinen Listener auf 8765 oder 8876. Damit ist W-08 bestanden.

## Mobile und Transfermatrix

| ID | Reale Prüfung | Erwartung | Status/Beleg |
|---|---|---|---|
| M-01 | neuestes stabiles iOS/iPadOS mit Safari am Freeze-Tag | Login, Navigation, Download, Upload, Abbruch und Sitzungsverlust real bestanden | **nicht ausgeführt:** Kein verfügbares Testgerät; iOS/iPadOS und Safari werden für diese Beta nicht als getestet oder unterstützt ausgewiesen. |
| M-02 | Android 10+ mit aktuellem stabilem Chrome am Freeze-Tag | Login, Navigation, Download, Upload, Retry und Sitzungsverlust real bestanden | **reale Android-Kernfunktion mit Firefox bestanden:** Auf Android 16 mit Firefox funktionierten Login, LAN-Zugriff, der Download und das Öffnen einer kleinen Test-Textdatei sowie der Upload eines Bildes in den PC-Eingangsordner. Auch beide Abmelderichtungen wurden real bestätigt: Das Handy kann seine Sitzung selbst beenden, und ein Trennen am PC entzieht dem Handy den Zugriff wie vorgesehen. Prozess, Listener und enge Firewallregel blieben stabil; das Log blieb ohne Warnung oder Fehler. Der Owner verlangt keine redundante Wiederholung desselben Kernflusses in Chrome. Die browserspezifische Chrome-Abdeckung und ein gezielt erzwungener Transfer-Retry bleiben als Evidenzlücken sichtbar. `P4-UI-01` betrifft die Anzeige, nicht den erfolgreichen Dateiinhalt. |
| T-01 | große Datei und Range-Download | korrekter Inhalt, Fortschritt und Abbruch ohne unkontrollierte Parallelität | **Kernpfad real, Stresspfad akzeptiert offen:** Eine kleine Textdatei wurde auf Android 16/Firefox vollständig heruntergeladen, geöffnet und inhaltlich bestätigt. Große Datei, reale Range-Fortsetzung und Abbruch wurden auf Owner-Wunsch nicht vorbereitet; Range-, Lease-, Abbruch- und Kapazitätsgrenzen bleiben automatisiert geprüft. |
| T-02 | viele kleine Dateien und Uploadqueue | Reihenfolge, Retry, Pause und Abbruch ohne Überschreiben | **Kernpfad real, Stresspfad akzeptiert offen:** Ein Bild wurde erfolgreich von Android 16/Firefox in den PC-Eingangsordner geladen. Reale Queue-, Pause-, Retry- und Kollisionsläufe wurden auf Owner-Wunsch übersprungen; Reducer-, Retry-, Offset-, No-Replace- und Idempotenzpfade bleiben automatisiert geprüft. |
| T-03 | langsamer beziehungsweise voller Testdatenträger | sichtbarer sicherer Fehler; Offset, Budget und Partials bleiben konsistent | **nicht real ausgeführt; akzeptiertes Beta-Risiko:** Künstlicher langsamer/voller Datenträger wird nicht vorbereitet. Fehlerhafte Chunk-Schreibvorgänge, Reserven, Offset-/Budgetkonsistenz und Partialbereinigung sind automatisiert geprüft. |
| T-04 | Netzwerkwechsel während einer Sitzung | Verbindung endet sicher; neues Netz verlangt gegebenenfalls neue Bestätigung | **nicht real ausgeführt; akzeptiertes Beta-Risiko:** Ein erzwungener WLAN-Wechsel wird übersprungen. Profiländerung, erneute Vertrauensbindung, Netzwerkverlust und sicherer Dienststopp sind automatisiert geprüft. |

## `P4-UI-01` – inkonsistente Übertragungsanzeige

Der Operator beobachtete beim erfolgreichen bidirektionalen Firefox-/Android-
Test, dass die Übertragungsanzeige anscheinend nicht zuverlässig oder nur für
eine Richtung aktualisiert wird. Download- und Uploadinhalt kamen trotzdem
korrekt an; Prozess, Listener, Firewallregel und Log zeigten danach keinen
technischen Fehler. Richtung, betroffene Oberfläche, zeitlicher Verlauf und
Reproduzierbarkeit sind noch nicht isoliert.

Die Quellprüfung zeigte keine fehlende Übertragungsrichtung: Der Desktop erhält
Backendereignisse und Verlaufseinträge für Upload und Download. Auf dem Handy
zeigt LDTG den selbst gesteuerten Uploadfortschritt; Downloads werden bewusst an
die native Downloadfunktion des Browsers übergeben, deren Fortschritt außerhalb
der LDTG-Seite erscheint. Sehr kleine Dateien können außerdem zwischen zwei
Darstellungsframes abschließen und direkt im Verlauf landen. Das erklärt die
Beobachtung, ohne einen Inhalts- oder Protokollfehler zu belegen.

Die Mobile-Oberfläche erklärt nun ausdrücklich, wo Download- und
Uploadfortschritt erscheinen; der Desktop weist auf sofort im Verlauf landende
Kurztransfers hin. 39 Mobile- und 36 Desktoptests sichern die Texte und die
bestehenden Richtungs-/Fortschrittspfade ab. Der Owner verzichtet auf einen
manuellen Großdatei-Retest und akzeptiert die verbleibende Echtzeit-Anzeigelücke
als dokumentierte Beta-Grenze. `P4-UI-01` wird damit als P3-UX-Klarstellung
geschlossen, nicht als behaupteter Großdatei-Bestehensbeleg.

## Abbruch- und Rückrollregeln

- Ein P0- oder P1-Befund stoppt die Matrix sofort; keine weiteren mutierenden
  Schritte werden ausgeführt.
- Bestehende AppData- und Freigabeinhalte werden nie rekursiv gelöscht.
- Vor und nach jedem Installer-/Firewall-Schritt werden registrierte Version,
  beide exakten Regelnamen und Sentinelzustand verglichen.
- Nur explizit neu erzeugte temporäre P4-Testpfade dürfen nach erfolgreicher
  Belegsicherung entfernt werden.
- Der P3-Installer bleibt privat und unsigniert; Screenshots oder Logs werden vor
  Aufnahme in QA-Belege auf persönliche Pfade, Geräte- und Netzkennungen geprüft.
