# LDTG 0.3.0-rc.2 – Release-Notes

Stand: 4. September 2026
Status: **private, unsignierte Prerelease; noch nicht öffentlich freigegeben**

LDTG – Local Device Transfer Gateway überträgt Dateien direkt zwischen einem
Windows-PC und einem Handy im selben ausdrücklich vertrauten lokalen Netzwerk.
Es gibt kein Cloudkonto und LDTG lädt die übertragenen Dateien nicht zu einem
externen Dienst hoch.

## Was dieser Kandidat enthält

- Download vom freigegebenen PC-Ordner auf das Handy und Upload in einen
  getrennten PC-Eingangsordner;
- kurzlebigen Zugangscode, Gerätesitzungen und Trennen vom PC oder Handy;
- Freigabeprofile sowie begrenzte Transferqueues mit Pause, Fortsetzen, Abbruch
  und Retry;
- explizite Netzwerkbestätigung und eine eng auf Programm, TCP-Port und lokales
  Subnetz beschränkte Windows-Firewallregel;
- lokale Status-, Verlaufs- und Diagnoseansichten;
- dauerhaft sichtbare tatsächliche Programmversion unten in der
  Desktop-Seitenleiste;
- Apache-2.0-Projektlizenz, CycloneDX-SBOM und versionsgenaue
  Drittanbieterhinweise.

## Geprüfter Kernumfang

Die P4-Basis dieses Kandidaten wurde in den folgenden realen Kernpfaden
geprüft. Der finale `0.3.0-rc.2`-Installer bestand zusätzlich die in P6
festgelegte kurze Wiederholungsabnahme.

- Windows 11 25H2, x64;
- Installation, Reparaturinstallation, Firewallwechsel zwischen Port 8765 und
  8876, Dienststart/-stopp und vollständige Deinstallation;
- Erhalt von Einstellungen und Freigabedaten bei Reparaturinstallation,
  Deinstallation und Neuinstallation;
- Android 16 mit Firefox: Anmeldung, Textdatei vom PC laden und öffnen, Bild zum
  PC hochladen sowie Sitzung in beiden Richtungen trennen;
- gezielte Prüfung des finalen Installers mit dem oben genannten SHA-256-Hash
  am 4. September 2026 mit Microsoft Defender ohne Fund.

## Bekannte Grenzen

- Der Kandidat ist nicht digital signiert. Windows kann deshalb eine
  Herausgeber- oder SmartScreen-Warnung zeigen. Es wird keine Warnungsfreiheit
  versprochen.
- Nur Windows 11 25H2 sowie Android 16 mit Firefox wurden im Kernfluss real
  geprüft. iOS/iPadOS, andere Android-Versionen und -Browser sowie Chrome sind
  nicht als getestet zugesichert.
- Große Dateien, viele kleine Dateien, real erzwungener Retry, Pause/Abbruch
  unter Last, voller oder langsamer Datenträger und Netzwerkwechsel wurden nicht
  als Realsystem-Stresstest ausgeführt; die zugrunde liegenden Grenz- und
  Fehlerpfade sind automatisiert getestet.
- Browserdownloads zeigen ihren eigentlichen Fortschritt im Browser. Sehr kurze
  Transfers können in LDTG direkt im Verlauf erscheinen, bevor eine laufende
  Anzeige wahrnehmbar ist.
- Nur die jeweils neueste veröffentlichte Beta wäre unterstützt. Die Beta wäre
  Best Effort ohne SLA; Internetfreigabe, Portweiterleitung, NAS-Freigaben,
  Windows 10 und andere Desktopbetriebssysteme sind nicht unterstützt.

## Sicherheitshinweise

LDTG nur in einem vertrauten lokalen Netzwerk starten. Zugangscode nicht an
Unbefugte weitergeben und den Dienst nach der Übertragung stoppen. Empfangene
Dateien werden nicht automatisch ausgeführt, aber auch nicht auf Schadsoftware
geprüft. Sie sind wie andere externe Dateien zu behandeln.

## Lizenz und Beiträge

LDTG steht unter Apache-2.0, Copyright © 2026 Kordariel666. Die erste Beta nimmt
Fehlerberichte und Funktionsvorschläge über Issues an; Pull Requests sind
zunächst nicht geöffnet. Vertrauliche Sicherheitsmeldungen dürfen nicht in ein
öffentliches Issue geschrieben werden.

## Kandidatennachweis

- Quellcommit: `4956cd3de88d6d5d5cff6c3653a71d9695a60c32`
- Installer SHA-256:
  `7c7263d37e94cac525ac7de6d5ec1ffaf3c3a3ed680dc13e79017886c19d6f9c`
- SBOM SHA-256:
  `6cb9766a383b6fca5e5c3e947022f187cd8598d4fdd0fcaa61c57be00eb8fdb4`
- Authenticode: `NotSigned`

Die [GitHub-Prerelease](https://github.com/Kordariel666/LocalDeviceTransferGateway-LDTG/releases/tag/v0.3.0-rc.2)
mit Installer, Prüfsummen, SBOM, Buildmanifest und Lizenznachweisen ist im
privaten Repository vorbereitet. Sie wird erst durch eine gesonderte
Entscheidung zur öffentlichen Repositorysichtbarkeit allgemein zugänglich. Die
gezielte Microsoft-Defender-Prüfung genau des oben gehashten finalen Installers
wurde am 4. September 2026 ohne Fund abgeschlossen.
