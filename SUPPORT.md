# Support für eine mögliche öffentliche Beta

Stand: 3. September 2026  
Status: **PB-04 am 3. September 2026 akzeptiert; noch kein wirksames Supportversprechen**

Dieser Rahmen wird erst mit einer späteren ausdrücklichen
Veröffentlichungsfreigabe wirksam. Das Repository bleibt derzeit privat; es gibt
keine öffentliche Beta und keinen externen Supportkanal.

## Akzeptierter Umfang

- Unterstützt wird nur die jeweils neueste veröffentlichte LDTG-Beta.
- Desktop: Windows 11 25H2 Home/Pro mit aktuellen Sicherheitsupdates, aktueller
  WebView2 Runtime und der im Release angegebenen Architektur.
- Mobil: die bei P4 tatsächlich bestandenen Kombinationen aus der zum
  Release-Freeze neuesten stabilen iOS-/iPadOS-Version mit Safari sowie Android
  10 oder neuer mit aktuellem stabilen Chrome. Exakte Geräte-, OS- und
  Browserversionen werden in der Release-Abnahmematrix festgehalten; eine nicht
  geprüfte Kombination gilt nicht stillschweigend als unterstützt.
- Windows 10, frühere oder bereits abgekündigte Windows-11-Stände, andere
  Desktopbetriebssysteme, entfernte/NAS-Freigaben, Internetexposition,
  Portweiterleitung, UPnP, Fernadministration und nicht vertrauenswürdige Netze
  sind nicht unterstützt. Freiwillige Kompatibilitätsbeobachtungen erweitern
  diesen Umfang nicht.

Windows 10 Home/Pro erreichte sein Supportende am 14. Oktober 2025. Eine private
oder kommerzielle ESU-Teilnahme einzelner Nutzer ist keine tragfähige allgemeine
Produktbasis. Windows 11 24H2 erreicht für Home/Pro bereits im Oktober 2026 sein
Supportende, während Windows 11 25H2 bis Oktober 2027 unterstützt wird. Deshalb
ist 25H2 die engste belastbare Beta-Basis:

- <https://learn.microsoft.com/en-us/lifecycle/products/windows-10-home-and-pro>
- <https://learn.microsoft.com/en-us/lifecycle/products/windows-11-home-and-pro>
- <https://learn.microsoft.com/en-us/windows/whats-new/enable-extended-security-updates>
- <https://support.google.com/chrome/answer/95414?co=GENIE.Platform%3DAndroid&hl=en>
- <https://support.apple.com/en-mide/100100>

## Kanäle und Reaktionsniveau

Nach einer Veröffentlichung dienen GitHub Issues ausschließlich gewöhnlichen
Fehlerberichten und nachvollziehbaren Verbesserungsvorschlägen. Vertrauliche
Sicherheitsdetails gehören in GitHub Private Vulnerability Reporting; dieser
Kanal muss vor dem öffentlichen Gate aktiviert und praktisch geprüft sein.

Die Beta ist ein Best-effort-Projekt ohne SLA. Es gibt keine garantierte
Antwort-, Triage-, Behebungs- oder Releasefrist und keinen Anspruch auf
Kompatibilität mit älteren Betas. Kritische Meldungen werden nach Möglichkeit
priorisiert, aber diese Priorität ist keine Fristzusage.

## Akzeptierte minimale Wartung

Solange die Beta ausdrücklich als aktiv geführt wird:

- einmal pro Kalendermonat Lockfile-, Abhängigkeits- und Sicherheitsmeldungen
  prüfen und das Datum dokumentieren;
- bestätigte kritische Risiken bestmöglich triagieren und, falls keine zeitnahe
  sichere Behebung möglich ist, die betroffene Beta sichtbar zurückziehen oder
  als nicht unterstützt kennzeichnen;
- nur den neuesten Beta-Stand als unterstützt darstellen.

Diese Mindestweise ist mit PB-04 als Produktentscheidung akzeptiert, aber keine
bereits abgegebene Dauerzusage. Sie wird nur zusammen mit einer späteren
ausdrücklichen Veröffentlichungsfreigabe und einem klaren Ende des Beta-Status
aktiviert.

## Ende oder Pause des Projekts

Der Owner kann die Beta jederzeit pausieren oder beenden. Dann werden README und
Releasehinweise sichtbar auf „nicht mehr gepflegt“ gesetzt, das Repository kann
archiviert werden und alte Binärdateien werden nicht als weiterhin sicher
unterstützt dargestellt. Bereits heruntergeladene Dateien können technisch nicht
zurückgerufen werden; der Hinweis darf daher nicht nur in einem neuen Release
stehen.

Entscheidung `PB-04` vom 3. September 2026: Der Owner akzeptiert diesen engen
Umfang samt monatlicher Best-effort-Prüfung und geordnetem Archivierungsweg für
den Fall einer später ausdrücklich freigegebenen öffentlichen Beta.
