# Design QA – responsiver Zugangscode

**Vergleichsziel**

- Source visual truth: `qa/desktop-running-overlap-reference-1184x849.png` plus die Anforderung, dass Zugangscode und Aktionen bei einem nicht maximierten Fenster vollständig und ohne Überlagerung lesbar bleiben.
- Implementation screenshot: `qa/desktop-running-responsive-1182x852.jpg`.
- Full-view comparison evidence: `qa/responsive-code-comparison.png`.
- Viewport: Tauri-Fenster mit konfigurierten 1180 × 820 CSS-Pixeln; Windows-Aufnahmen inklusive Fensterrahmen 1184 × 849 Pixel (Referenz) und 1182 × 852 Pixel (Implementierung).
- Density normalization: keine Dichtekonvertierung der Originale; beide Aufnahmen wurden zusätzlich proportional und ohne Zuschnitt in gleich breite Vergleichsfelder eingepasst.
- State: Desktop, Dark Theme, Übersicht, Dienst aktiv, Ethernet, keine verbundenen Geräte oder aktiven Übertragungen, zwei aktive Freigaben.

**Findings**

- Keine verbleibenden P0-, P1- oder P2-Abweichungen.
- Fonts and typography: Schriftfamilie, Gewichte, Größen, Zeilenhöhen und Zeichenabstand bleiben unverändert. Der achtstellige Code wird vollständig in einer Zeile dargestellt.
- Spacing and layout rhythm: Bei mittleren Fensterbreiten liegt die Aktionsgruppe nun in einer eigenen zweiten Zeile. Code und Schaltflächen haben sichtbaren Abstand und überlagern sich nicht; Seitenleiste, Kopfzeile und nachfolgende Karten behalten ihre bisherigen Proportionen.
- Colors and visual tokens: Hintergrund-, Rahmen-, Text-, Status- und Akzentfarben entsprechen der Referenz und verwenden unverändert die bestehenden Tokens.
- Image quality and asset fidelity: Das bestehende QR-Code-Rendering und die vorhandenen Markenassets bleiben unverändert, scharf und korrekt skaliert.
- Copy and content: Zugangscode, Adresse, Beschriftungen und Aktionen sind unverändert und vollständig sichtbar.

**Open Questions**

- Keine.

**Comparison History**

1. Ausgangszustand: P1 – `4430 6357` überlagerte bei der Referenzbreite die Schaltflächen `Code kopieren` und `Code erneuern`; dadurch waren Code und Aktionen teilweise verdeckt.
2. Erste Korrektur: Der bisherige dreispaltige Kartenumbruch wurde von 1120 auf 1240 Pixel vorgezogen. Die Sichtprüfung zeigte zwar die behobene Überlagerung, aber auch eine unnötig früh verkleinerte Seitenleiste (P2-Abweichung außerhalb der betroffenen Karte).
3. Finale Korrektur: Der neue 1240-Pixel-Umbruch wurde auf Verbindungs-Karte, QR-Code und Aktionsgruppe begrenzt; der bestehende 1120-Pixel-Umbruch für das restliche App-Layout blieb erhalten.
4. Post-fix evidence: `qa/desktop-running-responsive-1182x852.jpg` zeigt den vollständigen Code `7180 0371` oberhalb der beiden vollständig sichtbaren Aktionen. `qa/responsive-code-comparison.png` stellt Referenz und korrigierte Implementierung gemeinsam dar.

**Focused Region Comparison**

- Kein separater Ausschnitt erforderlich: Verbindungs-Karte, Zugangscode und beide Schaltflächen sind in der gemeinsamen Vollansicht eindeutig lesbar; zusätzlich wurden beide Originalaufnahmen in Originalauflösung geprüft.

**Implementation Checklist**

- [x] Responsiven Umbruch vor der problematischen Fensterbreite aktivieren.
- [x] Änderung auf die Verbindungs-Karte begrenzen.
- [x] Zugangscode und beide Aktionen im nativen Tauri-Fenster prüfen.
- [x] Dienst nach der Aufnahme stoppen, damit der sichtbare Testcode ungültig ist.
- [x] Vergleichsartefakt erzeugen und erneut visuell prüfen.

**Follow-up Polish**

- Keine offenen P3-Punkte für diesen Fix.

final result: passed
