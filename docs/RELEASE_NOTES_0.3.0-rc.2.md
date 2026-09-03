# LDTG 0.3.0-rc.2 – Entwurf der Release-Notes

Stand: 4. September 2026
Status: **unveröffentlichter, unsignierter Public-Beta-Kandidat**

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
geprüft. Der frisch gebaute `0.3.0-rc.2`-Installer erhält vor einer möglichen
Veröffentlichung zusätzlich die in P6 festgelegte kurze Wiederholungsabnahme.

- Windows 11 25H2, x64;
- Installation, Reparaturinstallation, Firewallwechsel zwischen Port 8765 und
  8876, Dienststart/-stopp und vollständige Deinstallation;
- Erhalt von Einstellungen und Freigabedaten bei Reparaturinstallation,
  Deinstallation und Neuinstallation;
- Android 16 mit Firefox: Anmeldung, Textdatei vom PC laden und öffnen, Bild zum
  PC hochladen sowie Sitzung in beiden Richtungen trennen;
- gezielte Prüfung des gehärteten Installers mit Microsoft Defender ohne Fund.

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

Diese Datei ist nur ein Entwurf. Downloadlink, Quellcommit, Installer-Hash und
SBOM-Hash werden erst durch einen bestandenen privaten P6-Dry-Run festgelegt.
Eine Veröffentlichung erfordert danach eine eigene ausdrückliche Freigabe.
