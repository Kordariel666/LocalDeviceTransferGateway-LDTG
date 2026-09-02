# Sicherheitsrichtlinie

## Unterstützter Umfang

Version 1 wird für Windows 10 und Windows 11 entwickelt. Sicherheitsmeldungen sollten sich auf den aktuellen Stand des Hauptzweigs beziehen.

Besonders relevant sind Befunde, durch die ein LAN-Client:

- außerhalb einer konfigurierten Freigabe lesen oder schreiben kann,
- Dateien löschen, überschreiben, verschieben, umbenennen oder ausführen kann,
- vorhandene Inhalte des Upload-Eingangs auflisten kann,
- Desktopsteuerung über die HTTP-API erreicht,
- Authentifizierung, Sitzung, CSRF-, Host-, Origin- oder Subnetzprüfung umgeht,
- Geheimnisse oder Dateiinhalte in Diagnoseausgaben findet.

## Bewusste Grenze von v1

DMDC v1 nutzt HTTP im bestätigten lokalen Netzwerk und beansprucht keine Transport- oder Ende-zu-Ende-Verschlüsselung. Ein Angreifer mit der Möglichkeit, LAN-Verkehr mitzulesen oder aktiv umzuleiten, liegt außerhalb der Schutzgarantie. Internetfreigabe, Router-Portweiterleitung, UPnP und der Einsatz in nicht vertrauenswürdigen Netzen werden nicht unterstützt.

## Durchgesetzte Sicherheitsgrenzen

- Eine Netzwerkfreigabe ist kurzlebig und an die vollständig neu ermittelte Windows-Netzwerkidentität gebunden. Fehlende Profilmetadaten gelten nicht als vertrauenswürdig.
- Freigabewurzeln werden beim Start verankert und während des Betriebs erneut geprüft. Downloads und Uploads verwenden stabile geöffnete Handles, sodass ein nachträglicher Austausch durch Junctions, Symlinks oder Umbenennungen nicht auf ein anderes Ziel umleitet.
- Upload-Eingänge auf entfernten oder unbekannten Laufwerkstypen sowie effektive Windows-Startordner und bekannte Office-Autoload-Verzeichnisse werden abgewiesen. Windows-Kurznamen werden nach der kanonischen Auflösung erneut gegen die Pfadrichtlinie geprüft.
- Abgeschlossene und aktive Uploads teilen sich endliche Byte- und Objektbudgets. Nicht abschließende Uploadblöcke sind exakt 8 MiB groß; Abschlusswiederholungen sind über eine begrenzte, ablaufende Quittung idempotent.
- Dateinamen mit bidirektionalen oder unsichtbaren Unicode-Steuerzeichen werden beim Upload entschärft und in den Oberflächen isoliert dargestellt.

Der `.dmdc`-Arbeitsordner ist kein öffentlicher Eigentumsnachweis. Laufende Uploads werden über ihre offenen Handles aufgeräumt; nach einem Prozessabsturz bleiben nicht mehr sicher zuordenbare Teildateien erhalten und müssen bei Bedarf manuell geprüft werden.

## Meldung

Bitte keine ungeprüften Details öffentlich veröffentlichen. Eine Meldung sollte betroffene Version, reproduzierbare Schritte, erwartetes und tatsächliches Verhalten sowie eine Einschätzung der Auswirkung enthalten. Zugangscodes, Sitzungstoken und private Dateinamen gehören nicht in Screenshots oder Logs.
