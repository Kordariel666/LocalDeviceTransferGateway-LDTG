# Datenschutz und lokale Datenverarbeitung

Stand: 3. September 2026  
Geltung: Quellstand `0.3.0-rc.1` einschließlich der P2-Härtung auf Basis von
`950e4301a61bbef79d4ecca3ed81b646baa356ca`

LDTG arbeitet ohne Konto, Cloud-Backend, Telemetrie, Werbung oder
produktseitige Updateabfrage. Die Desktop-App stellt ihre eingebetteten
Mobile-Assets und die Transfer-API nur über die ausgewählte private IPv4-Adresse
im lokalen Netz bereit. Dateiinhalt verlässt den Rechner ausschließlich als vom
Nutzer ausgelöster LAN-Transfer zu einem angemeldeten Browser; LDTG übermittelt
keine Produktdaten an den Projektbetreiber oder sonstige Dritte.

## Dateninventar

| Daten | Ort und Empfänger | Aufbewahrung | Kontrolle und Löschung |
|---|---|---|---|
| Einstellungen: Profile, Freigabepfade, aktive Profil-ID, Port, Grenzen, bevorzugter Adapter sowie bestätigte Netzwerk-ID, -name und -kategorie | `%APPDATA%\de.ldtg.desktop\settings.json`; nur lokaler Desktopprozess und Windows-Benutzer | bis zum Ändern oder manuellen Löschen; Deinstallation erhält die Datei | in der Desktop-App ändern; bestätigte Netze einzeln/gesamt entfernen; nach Beenden der App die Datei oder den exakt benannten App-Ordner manuell löschen |
| Recovery-Kopien ungültiger Einstellungen | daneben als `settings.recovery-1.json` bis `settings.recovery-100.json` | keine automatische Frist oder Verdrängung; höchstens 100 nummerierte Plätze; Deinstallation erhält sie | nach Prüfung und bei beendeter App manuell löschen |
| Lokale Logs | `%LOCALAPPDATA%\de.ldtg.desktop\logs`; stabile Fehlercodes, Operationen und technische Dienst-/Socketfehler, aber keine Zugangscodes, Tokens, Dateiinhalte oder Dateilisten | tägliche Rotation, höchstens 14 Logdateien; Deinstallation erhält sie | nach Beenden der App den exakt benannten Logordner manuell löschen |
| Zugangscode, Session-/CSRF-Token, IP-Adresse, physischer Peer-Ersatzschlüssel, optionaler Gerätename, klassifizierter Browser-/Gerätename und Zeitstempel | nur Arbeitsspeicher des laufenden Desktopdienstes; Session-Cookie und CSRF-Wert zusätzlich im jeweiligen Browser | bis Ablauf, Logout, lokalem Widerruf, Dienststopp oder Prozessende; Sitzungen höchstens 6 h 15 min inaktiv und 24 h absolut | eigene Browser-Sitzung abmelden; Sitzung lokal einzeln/gesamt widerrufen; Dienst stoppen |
| Roher HTTP-User-Agent | wird vom mobilen Browser an den lokalen Dienst gesendet und dort unmittelbar in eine feste grobe Bezeichnung wie „Safari auf iPhone“ umgewandelt | der rohe Wert wird nicht im Sitzungszustand, Desktopstatus, Log oder Diagnoseexport gespeichert | Dienst/Sitzung beenden entfernt die abgeleitete Bezeichnung |
| Transferverlauf: Dateiname, Richtung, Sitzungs-ID, Bytewerte, Status und Zeitstempel | nur Arbeitsspeicher des Desktopdienstes und lokale Desktop-WebView | Zielgröße 100 Einträge; ältere abgeschlossene Einträge werden zuerst verworfen, aktive nie; vollständig weg bei Dienst-/Prozessende | abgeschlossenen Verlauf in der Desktop-App leeren oder Dienst stoppen |
| Mobile Uploadqueue einschließlich lokaler Browser-Dateireferenzen, Dateinamen, Größen und Fortschritt | nur Arbeitsspeicher der ausgelieferten Browserseite | bis Entfernen, Seitenreload/-schluss oder Sitzungsverlust; kein Browser-Dauerspeicher | Einträge entfernen/abbrechen oder Seite schließen/neu laden |
| Kurzlebige Ordnercursor und Abschlussquittungen | nur Arbeitsspeicher des Desktopdienstes | Cursor 2 Minuten inaktiv; Abschlussquittungen 24 Stunden und höchstens 256 | Sitzung widerrufen beziehungsweise Dienst stoppen; Frist/Kapazität räumt automatisch auf |
| Freigegebene Downloadinhalte und abgeschlossene Uploads | ausschließlich in den vom Operator gewählten Ordnern; angemeldete LAN-Browser lesen Downloads oder schreiben neue Uploads | unter Kontrolle des lokalen Dateisystems; keine automatische Löschung | nur manuell außerhalb von LDTG; die App implementiert kein Löschen, Überschreiben, Verschieben oder Ausführen |
| Laufende Uploadteile | `<Uploadwurzel>\.ldtg\<UUID>.part` mit LDTG-Marker | live nach 30 Minuten ohne Block, spätestens nach 24 Stunden oder beim geordneten Stopp; nach Absturz bleiben nicht sicher zuordenbare Teile absichtlich erhalten | verwaiste Teile erst nach manueller Prüfung bei gestopptem Dienst löschen; der öffentliche `.ldtg`-Marker allein beweist keinen Dateibesitz |
| Diagnoseexport | vom Operator gewählte lokale JSON-Datei | bis zum manuellen Löschen | Export enthält Erstellungszeit, App-/Schemaversion, Plattform, Dienstzustand, Zähler, Boolesche Werte, Port/Limitwerte, aggregierten Netzwerk-/Firewallstatus und einen konstanten Datenschutzhinweis; keine Pfade, IPs, Adapter-/Netzwerkkennungen, Netzwerk-/Dateinamen, Codes, Tokens, Dateilisten oder rohen Fehlerdetails |
| Windows-Firewallregel | persistenter Windows-Systemzustand `LDTG Local Transfer` mit Programmpfad, TCP-Port und `LocalSubnet` | bis Neukonfiguration oder erfolgreicher Deinstallation | Einrichtung/Änderung und Entfernung verlangen UAC; der Uninstaller entfernt aktuelle und historische Regel strikt und bricht bei unbestätigter Entfernung ab |

Die tatsächlichen Windows-Dateirechte folgen dem lokalen Benutzerkonto und dem
gewählten Dateisystem. LDTG verschlüsselt lokale Einstellungen, Logs oder
Freigaben nicht zusätzlich. Wer den Rechner oder einen gewählten Freigabeordner
mit anderen Konten teilt, muss dessen ACLs selbst passend setzen.

## Deinstallation

Die Deinstallation löscht absichtlich keine AppData- oder Freigabeverzeichnisse
rekursiv. Dadurch werden Nutzerdateien und nicht eindeutig LDTG gehörende
Uploadreste nicht versehentlich entfernt. Nach einer erfolgreichen,
UAC-bestätigten Deinstallation können lokale Reste bei Bedarf manuell gelöscht
werden:

- `%APPDATA%\de.ldtg.desktop` für Einstellungen und Recovery-Kopien;
- `%LOCALAPPDATA%\de.ldtg.desktop` für Logs;
- nur nach eigener Prüfung einzelne `.part`-Dateien im `.ldtg`-Unterordner eines
  zuvor gewählten Upload-Eingangs.

Die gewählten Download- und Uploadordner selbst sind Nutzerdaten und gehören
nicht zu den AppData-Resten. Vor manueller Löschung muss LDTG beendet sein.

## Keine externen Laufzeitressourcen

Die eingecheckten Desktop- und Mobile-Quellen sowie ihre Produktionsbuilds
enthalten keine Telemetrie-SDKs, externen Fonts, CDN-Assets oder fest verdrahtete
öffentliche Produktendpunkte. Die Mobile-App verwendet ausschließlich
Same-Origin-Aufrufe an den lokalen LDTG-Dienst. npm-, crates.io- und GitHub-Zugriffe
sind Entwicklungs-/CI-Vorgänge und nicht Teil der installierten
Produktlaufzeit. Sicherheits- oder Buildprüfungen des Repositorys können je nach
vom Entwickler gewähltem Werkzeug eigene Dienste verwenden; dabei gelten die
Verträge dieses Werkzeugs, nicht der Produktdatenfluss von LDTG.

Quellbelege und Prüfergebnisse stehen im
[P2-Nachweis](../qa/public-beta/p2-security-privacy-support.md).
