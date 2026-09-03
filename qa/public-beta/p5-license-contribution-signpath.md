# P5 – Lizenz-, Beitrags- und SignPath-Entscheidungsmappe

Stand: 4. September 2026

Status: **P5 abgeschlossen; Apache-2.0, `Kordariel666` und zunächst nur Issues
vom Owner bestätigt; keine Veröffentlichung und kein SignPath-Antrag**

Diese Mappe ist eine technische und organisatorische Vorbereitung, keine
Rechtsberatung. Die Projektlizenz wurde am 4. September 2026 nach ausdrücklicher
Owner-Freigabe auf `Apache-2.0` festgelegt. Die Repositorysichtbarkeit wurde
nicht verändert.

## Ergebnis

P4 ist mit den ausdrücklich akzeptierten Testlücken abgeschlossen. Der
versionsgenaue P1-Audit enthält 857 Drittanbieterpakete und keine unbekannte,
proprietäre oder Source-available-Lizenz. Sowohl `Apache-2.0` als auch
`GPL-3.0-only` sind für den geprüften Graphen unter den dokumentierten
Notice- und MPL-2.0-Pflichten technisch möglich.

Der Owner bestätigte am 4. September 2026 zusätzlich:

1. `Kordariel666` als vorläufiges öffentliches Copyright-Pseudonym;
2. für die erste Beta nur Issues und noch keine Pull Requests.

`Apache-2.0` ist die verbindliche Projektlizenz. `Kordariel666` steht als bereits
öffentlich verwendetes Repository-Pseudonym in `NOTICE` und Paketmetadaten.
Für die erste Beta werden nur strukturierte Fehlerberichte und
Funktionsvorschläge geöffnet; ein klar ausgewiesener unsignierter Installer
bleibt der vorgesehene erste Veröffentlichungsweg.

## Vergleich der Projektlizenzen

| Frage | `Apache-2.0` | `GPL-3.0-only` |
|---|---|---|
| Private und kommerzielle Nutzung | erlaubt | erlaubt |
| Veränderung und Weitergabe | erlaubt; Lizenz, Änderungs- und bestehende Attribution-/Notice-Hinweise müssen erhalten werden | erlaubt; bei Weitergabe gelten GPLv3, vollständiger korrespondierender Quellcode und die weiteren GPL-Bedingungen |
| Proprietäre Ableitung | grundsätzlich möglich, sofern die Apache-Pflichten eingehalten werden | eine weitergegebene abgeleitete Gesamtarbeit muss grundsätzlich GPLv3 bleiben; private Änderungen müssen nicht veröffentlicht werden |
| Patente | ausdrückliche Patentlizenz der Beitragenden mit Beendigungsklausel bei bestimmter Patentklage | ausdrückliche Patentregelungen und Schutz gegen zusätzliche Beschränkungen |
| Beiträge | Abschnitt 5 stellt absichtlich eingereichte Beiträge standardmäßig unter Apache-2.0, sofern nichts anderes erklärt oder vereinbart wurde | die Beitragsregeln müssen denselben `GPL-3.0-only`-Pfad ausdrücklich festlegen |
| Späterer Lizenzwechsel | Rechteinhaber können ihren eigenen Code anders lizenzieren; fremde Beiträge bleiben ohne zusätzliche Erlaubnis gebunden | fremde Beiträge erschweren eine spätere proprietäre Lizenzierung ebenso; ein DCO überträgt kein Copyright |
| SignPath Foundation | OSI-anerkannt; kostenlose Foundation-Signierung schließt trotzdem kommerzielle Doppellizenzierung aller Komponenten aus | OSI-anerkannt und grundsätzlich passend, solange auch alle übrigen Bedingungen erfüllt sind |
| Geeignet, wenn … | breite Nutzung, einfache Integration und spätere geschäftliche Flexibilität wichtiger sind | veröffentlichte Ableitungen des Programms ebenfalls frei bleiben sollen |

Apache-2.0 verlangt bei Weitergabe insbesondere eine Lizenzkopie, deutliche
Änderungshinweise und den Erhalt einschlägiger Copyright-, Patent-, Marken- und
Attributionshinweise. Ein vorhandenes `NOTICE` muss in vorgeschriebener Form
weitergegeben werden. Die Lizenz gewährt keine allgemeinen Markenrechte.

GPLv3 erlaubt Nutzung, Änderung, Verkauf und Weitergabe. Wer eine Binärfassung
weitergibt, muss die jeweils anwendbaren Quellcode- und Lizenzpflichten
erfüllen. Interne oder private Änderungen lösen für sich keine Pflicht aus, sie
zu veröffentlichen. LDTG würde ausdrücklich `GPL-3.0-only` und nicht automatisch
„oder später“ wählen.

Maßgebliche Primärquellen:

- [Apache License 2.0](https://www.apache.org/licenses/LICENSE-2.0.html)
- [Apache: Applying the License](https://www.apache.org/legal/apply-license)
- [GNU General Public License v3.0](https://www.gnu.org/licenses/gpl.en.html)
- [GNU GPL FAQ](https://www.gnu.org/licenses/gpl-faq.en.html)
- [OSI Approved Licenses](https://opensource.org/licenses)

## Auswirkungen der vorhandenen Abhängigkeiten

Der [P1-Abhängigkeitsaudit](dependency-license-audit.md) bleibt die technische
Quelle. P6 erzeugt aus dem tatsächlich ausgelieferten Windows-Graphen die
versionsgenaue Datei [`THIRD_PARTY_NOTICES.md`](../../THIRD_PARTY_NOTICES.md).
Besonders sichtbar behandelt werden:

- MPL-2.0-Komponenten und deren dateibezogene Quellpflichten;
- Apache- und LLVM-Exception-Texte;
- Unicode-, MIT-, BSD-, ISC-, Zlib- und sonstige Attributionen;
- bestehende Upstream-`NOTICE`-Dateien;
- die dokumentierte Wahl einer zulässigen Alternative bei `OR`-Ausdrücken.

Die Projektlizenz ersetzt keine dieser Drittanbieterbedingungen.

## Beiträge Dritter

### Gewählter Start: nur Issues

Für die erste Beta werden Fehlerberichte und Funktionsvorschläge angenommen,
aber noch keine Codebeiträge zugesagt. Das hält Review-, Rechte- und
Supportaufwand klein. Ein Pull Request kann geschlossen oder bis zur späteren
Öffnung der Beiträge zurückgestellt werden.

### Spätere Option: Pull Requests mit DCO

Wenn Pull Requests geöffnet werden, gilt „inbound = outbound“: Beiträge müssen
unter derselben Lizenz wie das Projekt eingereicht werden. Zusätzlich bestätigt
ein `Signed-off-by` nach dem Developer Certificate of Origin 1.1, dass die
beitragende Person zur Einreichung berechtigt ist. Das DCO ist keine
Copyright-Abtretung und verschafft dem Maintainer keine automatische
proprietäre Zweitlizenz.

Quelle: [Developer Certificate of Origin 1.1](https://developercertificate.org/)

Ein CLA wird für die Beta nicht empfohlen: Es erhöht die Einstiegshürde und ist
nur sinnvoll, wenn später tatsächlich weitergehende Rechte benötigt werden.
Eine kommerzielle Doppellizenzstrategie wäre außerdem nicht mit dem kostenlosen
SignPath-Foundation-Pfad vereinbar.

Aktiv vorbereitet sind:

- [Beitragsrichtlinie](../../CONTRIBUTING.md)
- [Verhaltensregeln](../../CODE_OF_CONDUCT.md)
- strukturierte GitHub-Vorlagen für Fehlerberichte und Funktionsvorschläge.

Die [Pull-Request-Vorlage](../../docs/archive/project-history/PULL_REQUEST_TEMPLATE_DRAFT.md) bleibt bewusst ein
nicht aktiver Entwurf für eine mögliche spätere Beitragsphase.

## SignPath-Vorprüfung

Die Bedingungen wurden am 4. September 2026 anhand der aktuellen
[SignPath-Foundation-Bedingungen](https://signpath.org/terms.html) geprüft.

| Bedingung | Stand | Restmaßnahme |
|---|---|---|
| kein Malware-/PUA-Inhalt | lokale Defender-Kernabnahme bestanden | finalen P6-Installer erneut prüfen; spätere Bewertung bleibt extern veränderlich |
| OSI-Lizenz, keine kommerzielle Doppellizenz | `Apache-2.0` gewählt und aktiviert; keine Doppellizenz eingerichtet | vor Antrag gegen dann öffentlichen Lieferumfang prüfen |
| kein proprietärer Projektbestandteil | Audit fand keinen entsprechenden Bestandteil | finalen Auslieferungsumfang gegen Quellrepository abgleichen |
| aktiv gepflegt | privates Projekt wird aktiv bearbeitet | erst nach Veröffentlichung öffentlich belegbar |
| bereits in zu signierender Form veröffentlicht | noch nicht erfüllt | erste Beta separat freigeben und ehrlich als unsigniert kennzeichnen |
| Funktion auf Download-/Store-Seite dokumentiert | README-Entwurf vorhanden | finale Release-Seite in P6 entwerfen |
| Datenschutz, Systemänderungen, Deinstallation | Dokumentation und reale Kernabnahme vorhanden | finale Installer-/Release-Texte in P6 gegenprüfen |
| MFA für Repository und SignPath | nicht verifiziert | vor einem Antrag manuell bestätigen |
| Rollen für Autor/Reviewer/Approver | Prozessentwurf vorhanden | öffentliche Personen beziehungsweise Teams nach Projektaufnahme benennen |
| verifizierbarer Build und manuelle Freigabe | private Pipeline und Hashbindung vorbereitet | auf öffentliche CI-/SignPath-Konfiguration übertragen und prüfen |
| öffentlich verlinkte „Code signing policy“ | lokaler Entwurf vorhanden | bei Veröffentlichung von Start- und Release-Seite verlinken |
| Projektkontrolle und Reputation | Repositorykontrolle intern belegt, Reputation noch nicht | erst nach einer öffentlichen Version durch SignPath bewertbar |

Die Foundation kann einen Antrag ablehnen und verlangt für Zertifikate die
Verwendung ihrer Publisher-Identität. Eine SignPath-Annahme, SmartScreen-
Warnungsfreiheit oder ein bestimmter Zeitpunkt wird daher nicht versprochen.
Der praktische kostenlose Pfad beginnt mit einer separat genehmigten,
unsignierten öffentlichen Beta. Ein SignPath-Antrag folgt nur nach einer neuen
ausdrücklichen Genehmigung.

## Gate P5

- Lizenzvergleich, Owner-Entscheidung und Aktivierung von `Apache-2.0`:
  **erfüllt**.
- Beitragsoptionen und nicht aktive Entwürfe: **erfüllt**.
- intern mögliche SignPath-Vorprüfung: **erfüllt**.
- Öffentliche Identität und Beitragsmodus: **erfüllt**.

P5 ist abgeschlossen. `licenseActivated` ist in `blockers.json` auf `true`
gesetzt; `published` und `signPathStarted` bleiben auf `false`. P6 ist der
nächste interne Schritt.
