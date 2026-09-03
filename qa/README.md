# QA-Bildartefakte

Die aktuellen Oberflächenaufnahmen werden als echte PNG-Dateien abgelegt:

- `desktop-stopped-1182x852.png`: Übersicht bei gestopptem Dienst.
- `desktop-running-1182x852.png`: Übersicht bei laufendem Testdienst.
- `desktop-network-running-1182x852.png`: Netzwerkeinstellungen bei laufendem Testdienst.
- `desktop-running-overlap-reference-1184x849.png`: gemeldeter Überlagerungsfehler bei nicht maximiertem Fenster.
- `desktop-running-responsive-1182x852.jpg`: korrigierter Zustand bei vergleichbarer Fenstergröße.
- `mobile-login-390x844.png`: mobile Anmeldung bei 390 × 844 CSS-Pixeln.
- `style-comparison.png`: automatisch erzeugte Gegenüberstellung der aktuellen Desktop- und Mobile-Aufnahmen.
- `responsive-code-comparison.png`: direkte Gegenüberstellung des gemeldeten und korrigierten Zugangscode-Layouts.

Desktop-Aufnahmen dürfen nur mit ausdrücklich ausgewählten Testordnern erfolgen.
Nach den Laufzustand-Aufnahmen wird der Dienst sofort gestoppt, damit der im Bild
sichtbare Zugangscode ungültig ist. Anschließend erzeugt

```powershell
python qa/make_comparison.py
```

die aktuelle Gegenüberstellung. Eine optionale externe Zielreferenz kann mit
`--reference <pfad>` ergänzt werden; das Skript enthält keine persönlichen oder
rechnerspezifischen Pfade.
