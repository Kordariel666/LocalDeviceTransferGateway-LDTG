# Archiv: Design-QA zum LDTG-Branding

**Vergleichsziel**

- Source visual truth: `assets/ldtg-logo-lockup.png`, die ins Repository übernommene Kopie der vom Owner ausgewählten Variante 3.
- Die damaligen Umsetzungs- und Vergleichsaufnahmen wurden vor der
  Public-Beta-Vorbereitung aus dem aktuellen Arbeitsbaum entfernt, weil die
  historische QA-Sammlung noch die frühere DMDC-Bezeichnung enthielt. Dieses
  Protokoll bewahrt das Ergebnis; die Binärdateien bleiben über die lokale
  Git-Historie wiederherstellbar.
- Viewport: natives Tauri-Fenster mit einer Windows-Aufnahme von 1182 × 852 Pixeln.
- Density normalization: Die 1536 × 1024 Pixel große Vorlage und die 1182 × 852 Pixel große App-Aufnahme wurden für den Vollvergleich proportional eingepasst; der Fokusvergleich vergrößert ausschließlich den sichtbaren Markenbereich ohne inhaltliche Retusche.
- State: Desktop, Dark Theme, Übersicht, Dienst gestoppt, Ethernet, keine verbundenen Geräte oder aktiven Übertragungen.

**Findings**

- Keine verbleibenden P0-, P1- oder P2-Abweichungen.
- Brand direction: Die Buchstabenfolge LDTG, die warmweiße Typografie, der schwarze Grund und der gelb-orange Datei-Transfer-Pfeil der ausgewählten Richtung sind übernommen.
- Product identity: Fenstertitel, Seitenleiste, Langname und Programm-Icon verwenden konsistent `LDTG – Local Device Transfer Gateway`.
- Layout: Der bestehende App-Aufbau bleibt unverändert; das kompakte quadratische App-Zeichen sitzt in der bisherigen Icon-Fläche und die Langform bleibt daneben lesbar.
- Asset quality: Für die App wurde ein eigenes quadratisches Raster-Asset aus der ausgewählten Markenrichtung erzeugt; die Tauri-Plattformicons wurden daraus neu generiert. Die UI verwendet eine passend verkleinerte 128-Pixel-Variante und lädt nicht unnötig die große Quelldatei.
- Interaction: Die Navigation von Übersicht zu Freigaben und zurück wurde im nativen Windows-Fenster erfolgreich geprüft.

**Open Questions**

- Keine blockierende Frage für PB-03. Die Markenrichtung ist ausdrücklich zunächst ausgewählt und kann später als eigener Polishing-Schritt weiterentwickelt werden.

**Comparison History**

1. Ausgangspunkt: Die bisherige Produktidentität wurde verworfen; ein einzelnes `L` war nicht aussagekräftig genug.
2. Auswahl: Variante 3 wurde wegen der klaren Verbindung aus LDTG-Schriftzug und Datei-zu-Datei-Transfer gewählt.
3. Umsetzung: Lockup und kompaktes App-Zeichen wurden als echte Bildassets eingebunden, alle Tauri-Plattformicons neu erzeugt und die Produktbezeichnung im Code auf LDTG umgestellt.
4. Post-fix evidence: Der damalige Voll- und Fokusvergleich bestätigte die
   ausgewählte Richtung sowie die Umsetzung im Desktopprogramm. Die Aufnahmen
   wurden bei der späteren Repository-Bereinigung entfernt.

**Focused Region Comparison**

- Der Fokusvergleich bestätigt, dass Buchstabenfolge, Grundfarben und Transfermotiv in der App erhalten bleiben. Im kleinen Sidebar-Zeichen wird das Transferdetail naturgemäß subtiler, bleibt in der vergrößerten Prüfung aber erkennbar.

**Implementation Checklist**

- [x] Ausgewählte Markenrichtung als Repo-Asset übernehmen.
- [x] Kompaktes quadratisches App-Zeichen aus derselben Richtung erzeugen.
- [x] Tauri-Icons für alle Zielplattformen neu generieren.
- [x] Name, Langname, interne Bezeichner und sichtbare Texte auf LDTG umstellen.
- [x] Native Windows-App aufnehmen und primäre Navigation prüfen.
- [x] Vollansicht und fokussierten Markenvergleich erzeugen und visuell prüfen
  (historische Bilddateien anschließend aus dem aktuellen Arbeitsbaum entfernt).

**Follow-up Polish**

- P3: Bei 16–32 Pixeln ist der Datei-Transfer im Icon nur noch angedeutet. Falls die vorläufige Richtung später finalisiert wird, kann eine optisch vereinfachte Kleinstgrößenvariante mit kräftigerem Pfeil ergänzt werden.

final result: passed
