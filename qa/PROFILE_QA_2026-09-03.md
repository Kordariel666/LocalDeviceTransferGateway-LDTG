# Design-QA – Freigabeprofile

Stand: 3. September 2026

## Prüfumfang

- Desktop-Viewport: 1184 × 900 CSS-Pixel.
- Zustand: gestoppter Dienst, zwei gespeicherte Testprofile, aktives Profil
  „Fotos unterwegs“, zwei aktive Freigaben und explizite Netzwerk-, Port- und
  Limitüberschreibungen.
- Nachweise: `desktop-profile-shares.png` und
  `desktop-profile-overrides.png`.

## Ergebnis

- Profilwahl, Name und Aktionen bilden oberhalb der Freigaben eine eindeutige
  Bediengruppe; beide Freigabekarten bleiben ohne Überlagerung vollständig
  sichtbar.
- Die drei Override-Schalter sind gemeinsam gruppiert und die darunter
  angezeigten wirksamen Werte entsprechen dem aktiven Testprofil.
- Fokusreihenfolge, zugängliche Namen und Zustände wurden zusätzlich über den
  Browser-DOM geprüft. Profilwahl, Namensfeld, Aktionen und Override-Schalter
  sind semantisch adressierbar.
- Es bestehen keine offenen P0-, P1- oder P2-Abweichungen. Farben, Typografie,
  Abstände und Responsive-Verhalten verwenden die vorhandenen Desktop-Tokens.

Finales Ergebnis: bestanden.
