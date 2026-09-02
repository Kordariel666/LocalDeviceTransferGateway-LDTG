# R4.3 – Entscheidung zur strengeren Kopplung

Status: Design-Spike abgeschlossen am 3. September 2026

## Ergebnis

DMDC behält für v1 den wiederverwendbaren, dienstlokalen Zugangscode und die
dienstweit aktivierten Freigaberollen bei. Eine automatische Rotation nach der
ersten Anmeldung und vom Mobilgerät selbst gewählte Sitzungsrollen werden nicht
implementiert.

Das ist keine Vertagung einer kleinen UI-Option, sondern eine bewusste
Sicherheitsentscheidung:

| Variante | Sicherheitsgewinn | Bedien- und Systemwirkung | Entscheidung |
|---|---|---|---|
| Code nach erster Anmeldung automatisch rotieren | Begrenzt spätere Wiederverwendung eines beobachteten Codes, solange der Beobachter noch keine Sitzung besitzt | Ein erstes Gerät sperrt weitere legitime Geräte unerwartet aus; konkurrierende Anmeldungen benötigen eine klar definierte Linearisierung; bestehende Sitzungen bleiben trotzdem gültig | Nicht als v1-Standard |
| Automatische Rotation als Schalter | Kann in einem bewusst gestarteten Einzelgeräteablauf sinnvoll sein | Ohne sichtbares Kopplungsfenster ist unklar, wann ein weiteres Gerät zugelassen wird und welche Anmeldung die Rotation auslöst | Bis zu einem Desktop-bestätigten Kopplungsablauf zurückgestellt |
| Rolle beim mobilen Login anfordern | Kann die Oberfläche auf eine Rolle reduzieren | Ein Client mit gültigem Code könnte weiterhin „beide“ anfordern; eine eigene Angabe ist keine Autorisierung | Nicht als Sicherheitskontrolle implementieren |
| Rolle lokal am Desktop freigeben | Begrenzt eine neue Sitzung wirksam auf eine Teilmenge der aktiven Freigaben | Benötigt wartende Kopplungsanfragen, lokale Bestätigung, Ablaufregeln und neue Zustände | Sinnvolle spätere Ausbaustufe bei belegtem Bedarf |

Der aktuelle Code ist bis zur manuellen Rotation oder zum Ende des Dienstlaufs
für mehrere legitime Geräte verwendbar. Eine manuelle Rotation ersetzt den Code
und setzt dessen Fehlversuchszustand zurück, widerruft aber keine bereits
angelegten Sitzungen. Einzelne oder alle Sitzungen können separat am Desktop
widerrufen werden (`src-tauri/src/service/state/sessions.rs:27-35`,
`src-tauri/src/service/state/sessions.rs:273-305`).

Die Antwort von `GET /session` leitet `downloadEnabled` und `uploadEnabled`
gegenwärtig ausschließlich aus den für den Dienst gestarteten Wurzeln ab. Der
Client kann diese Berechtigungen nicht wählen (`src-tauri/src/service/api/auth.rs:156-168`).

## Warum die zwei naheliegenden Kurzlösungen nicht genügen

Ein Einmal-Code reduziert nur das Zeitfenster für eine spätere Anmeldung. Hat ein
Angreifer bereits eine Sitzung erzeugt, ändert eine Rotation deren Rechte nicht.
Ein automatischer Codewechsel müsste deshalb ausdrücklich festlegen, ob bereits
authentisierte Sitzungen bestehen bleiben oder widerrufen werden. Ein impliziter
Gesamtwiderruf würde laufende legitime Übertragungen abbrechen und wäre keine
vertretbare Nebenwirkung einer normalen Anmeldung.

Eine vom Mobilgerät gesendete Rolle ist nur eine Funktionspräferenz. Da der
Besitz des Zugangscodes derzeit die vollständige Anmeldung autorisiert, könnte
ein Angreifer dieselbe Rolle anfordern. Eine wirksame Einschränkung muss von der
vertrauenswürdigen lokalen Desktopseite erteilt und serverseitig in der Sitzung
gebunden werden. Gerätename und klassifizierter User-Agent bleiben dabei reine
Anzeigedaten und dürfen weder Rate-Limit-Schlüssel noch Identitätsnachweis sein
(`src-tauri/src/service/api/auth.rs:73-124`,
`src-tauri/src/service/state.rs:84-112`).

## Mindestentwurf für eine spätere strenge Kopplung

Eine spätere Umsetzung ist erst sinnvoll, wenn der konkrete Mehrgerätebedarf den
zusätzlichen Ablauf rechtfertigt. Dann gelten mindestens diese Invarianten:

1. Eine mobile Kopplungsanfrage prüft den Code, erzeugt aber noch keine
   Browsersitzung. Sie erhält eine zufällige, kurzlebige Anfrage-ID.
2. Fehlversuche werden weiterhin vor jeder Kopplungsanlage pro Client-IP und
   dienstweit gezählt. Gerätename, User-Agent und Rollenwunsch dürfen die
   Zählerschlüssel nicht aufteilen.
3. Offene Kopplungsanfragen besitzen feste globale und IP-bezogene Grenzen, eine
   kurze TTL und genau einen terminalen Entscheidungszustand.
4. Der Desktop zeigt die Anfrage an und erteilt explizit eine unveränderliche
   Teilmenge der aktuell aktivierten Rollen: nur Download, nur Upload oder beide.
5. Erst die lokale Bestätigung erzeugt Sitzungstoken und CSRF-Token. Jeder
   Handler erzwingt die Sitzungsrolle zusätzlich zu vorhandener Sitzung, CSRF,
   IP-, Host-, Origin- und Subnetzbindung.
6. Eine optionale Einmal-Code-Rotation erfolgt atomar erst nach erfolgreicher
   Sitzungsanlage. Noch offene Anfragen der alten Codegeneration werden ungültig;
   bestehende Sitzungen bleiben nur nach einer ausdrücklich dokumentierten
   Betreiberentscheidung aktiv.
7. Ein sichtbarer Desktopablauf „Weiteres Gerät koppeln“ hält Mehrgerätebetrieb
   möglich, ohne einen dauerhaft wiederverwendbaren Code vorauszusetzen.

Die Protokolländerung benötigt neue serverseitige Sitzungsfähigkeiten, ein
Pending-Pairing-Modell, Desktopereignisse und eine abgestimmte Mobile-UI. Sie wird
nicht als bloßes optionales Feld an `POST /auth` ergänzt.

## Pflichtprüfungen vor einer Umsetzung

- konkurrierende korrekte Anmeldungen an der Rotationsgrenze,
- ungültige und abgelaufene Anfrage-IDs sowie Wiederholung verlorener Antworten,
- globale, IP-bezogene und ausstehende Kopplungsgrenzen,
- wechselnde Gerätenamen, User-Agents und Rollenwünsche unter demselben
  Fehlversuchs- und Sitzungslimit,
- ausschließlich serverseitige Rollenteilmenge und Ablehnung jeder
  Rollenerweiterung,
- Rotation ohne stillen Widerruf bestehender Sitzungen sowie expliziter
  Gesamtwiderruf mit Transferabbruch,
- Dienststopp, Netzwerkwechsel und Rootwechsel während einer offenen Anfrage.

R4.3 ergänzt bereits Regressionen dafür, dass wechselnde Gerätenamen weder das
IP-bezogene Codeversuchslimit noch das Sitzungslimit aufteilen und dass eine
manuelle Rotation den Fehlversuchszustand zurücksetzt, ohne bestehende Sitzungen
zu widerrufen. Das aktuelle, quellgestützte Bedrohungsmodell steht in
[`THREAT_MODEL.md`](THREAT_MODEL.md).
