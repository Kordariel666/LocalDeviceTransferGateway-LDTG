# Sicherheitsrichtlinie

## Unterstützter Umfang

Die öffentliche Beta unterstützt ausschließlich ihren neuesten veröffentlichten
Stand auf Windows 11 25H2 mit aktuellen Sicherheitsupdates und WebView2. Als
real geprüfte Mobilkombination ist Android 16 mit Firefox dokumentiert; andere
Mobilgeräte und Browser sind nicht als getestet oder unterstützt zugesichert.
Windows 10 und ältere beziehungsweise bereits abgekündigte Windows-11-Stände
gehören nicht zum zugesagten Beta-Umfang. Sicherheitsmeldungen sollten sich auf
den aktuellen Stand des Hauptzweigs beziehen.

Besonders relevant sind Befunde, durch die ein LAN-Client:

- außerhalb einer konfigurierten Freigabe lesen oder schreiben kann,
- Dateien löschen, überschreiben, verschieben, umbenennen oder ausführen kann,
- vorhandene Inhalte des Upload-Eingangs auflisten kann,
- Desktopsteuerung über die HTTP-API erreicht,
- Authentifizierung, Sitzung, CSRF-, Host-, Origin- oder Subnetzprüfung umgeht,
- Geheimnisse oder Dateiinhalte in Diagnoseausgaben findet.

## Bewusste Grenze von v1

LDTG v1 nutzt HTTP im bestätigten lokalen Netzwerk und beansprucht keine Transport- oder Ende-zu-Ende-Verschlüsselung. Ein Angreifer mit der Möglichkeit, LAN-Verkehr mitzulesen oder aktiv umzuleiten, liegt außerhalb der Schutzgarantie. Internetfreigabe, Router-Portweiterleitung, UPnP und der Einsatz in nicht vertrauenswürdigen Netzen werden nicht unterstützt.

## Durchgesetzte Sicherheitsgrenzen

- Gespeicherte Profile sind ausschließlich lokale Konfiguration. Vor jedem
  Dienststart wird genau ein aktives Profil aufgelöst und unveränderlich an den
  Dienst übergeben; ein LAN-Client kann weder Profile auswählen noch zusätzliche
  Freigabewurzeln aktivieren.
- Eine Netzwerkfreigabe ist kurzlebig und an stabile ID **und** die vom Operator
  bestätigte Windows-Profilkategorie gebunden. Geänderte Kategorien und fehlende
  oder als „Unbekannt“ aufgelöste Profilmetadaten gelten bis zu einer erneuten
  lokalen Bestätigung nicht als vertrauenswürdig; Anzeigenamen sind keine
  Sicherheitsidentität.
- Freigabewurzeln werden beim Start verankert und während des Betriebs erneut geprüft. Downloads und Uploads verwenden stabile geöffnete Handles, sodass ein nachträglicher Austausch durch Junctions, Symlinks oder Umbenennungen nicht auf ein anderes Ziel umleitet.
- Upload-Eingänge auf entfernten oder unbekannten Laufwerkstypen sowie effektive Windows-Startordner und bekannte Office-Autoload-Verzeichnisse werden abgewiesen. Windows-Kurznamen werden nach der kanonischen Auflösung erneut gegen die Pfadrichtlinie geprüft.
- Abgeschlossene und aktive Uploads teilen sich endliche Byte- und Objektbudgets. Nicht abschließende Uploadblöcke sind exakt 8 MiB groß; Abschlusswiederholungen sind über eine begrenzte, ablaufende Quittung idempotent.
- Dateinamen mit bidirektionalen oder unsichtbaren Unicode-Steuerzeichen werden beim Upload entschärft und in den Oberflächen isoliert dargestellt.
- Optionale Gerätenamen gelten nur für die aktuelle Sitzung, sind auf 64 Zeichen begrenzt, verbieten Steuer- und bidirektionale Formatierungszeichen und werden im Desktop bidi-isoliert ausgegeben. Rohe User-Agent-Header werden nach lokaler Klassifizierung nicht im Desktopstatus offengelegt.
- Der Zugangscode bleibt bis zur lokalen Rotation oder zum Dienstende für mehrere legitime Geräte verwendbar. Rotation ersetzt nur den Code und setzt dessen Fehlversuchszustand zurück; bestehende Sitzungen werden ausschließlich durch Ablauf, Einzel-/Gesamtwiderruf oder Dienststopp beendet. Gerätename und User-Agent beeinflussen weder Fehlversuchs- noch Sitzungslimits. Windows gruppiert Authentisierungsversuche nach Möglichkeit über die lokale Nachbartabelle pro physischem Peer; ohne auflösbaren Eintrag gilt die Client-IP. Ein einzelner Peer kann den dienstweiten 50er-Haushalt deshalb höchstens mit seinem eigenen Zehner-Haushalt belasten.
- Download- und Uploadrollen werden in v1 dienstweit durch die beim Start aktivierten Freigabewurzeln festgelegt. Eine vom LAN-Client selbst angegebene Rolle wäre keine Autorisierung und wird deshalb nicht als Sicherheitskontrolle angeboten.

Der `.ldtg`-Arbeitsordner ist kein öffentlicher Eigentumsnachweis. Laufende Uploads werden über ihre offenen Handles aufgeräumt; nach einem Prozessabsturz bleiben nicht mehr sicher zuordenbare Teildateien erhalten und müssen bei Bedarf manuell geprüft werden.

## Meldung

GitHubs **Private Vulnerability Reporting** ist der vertrauliche Meldeweg für
externe Sicherheitsmeldungen:
<https://github.com/Kordariel666/LocalDeviceTransferGateway-LDTG/security/advisories/new>.
Ein öffentliches Issue ist kein Ersatz für vertrauliche Exploitdetails.

Bitte keine ungeprüften Details öffentlich veröffentlichen. Eine Meldung sollte
betroffene Version, reproduzierbare Schritte, erwartetes und tatsächliches
Verhalten sowie eine Einschätzung der Auswirkung enthalten. Zugangscodes,
Sitzungstoken und private Dateinamen gehören nicht in Screenshots oder Logs. Für
Reaktions- oder Behebungszeiten besteht in der Beta keine Garantie; der
vorgeschlagene Rahmen steht in [SUPPORT.md](SUPPORT.md).
