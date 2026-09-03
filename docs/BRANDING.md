# LDTG – Marken- und Namensentscheidung

Stand: 3. September 2026

## Entscheidung

Der Repositoryinhaber hat **LDTG – Local Device Transfer Gateway** als neuen
Produktnamen gewählt. Die frühere Bezeichnung wird vor der öffentlichen
Vorbereitung ersetzt. Bei der ersten Nennung in öffentlicher Kommunikation soll
der volle Name gemeinsam mit dem Kürzel erscheinen.

Die visuelle Richtung ist vorläufig gewählt: ein warmweißes `LDTG`-Wortzeichen
auf Anthrazit, in das eine gelbe Dateiübertragung von einer Quelldatei im `D` zu
einer Zieldatei im `G` integriert ist. Spätere Detailkorrekturen am Zeichen
öffnen die Namensentscheidung nicht erneut.

## Kollisions-Vorprüfung

Die folgenden exakten Suchen wurden am 3. September 2026 wiederholt:

| Quelle | Exakte Begriffe | Ergebnis |
|---|---|---|
| npm-Registry | `ldtg` | kein Paket; Registry antwortete mit HTTP 404 |
| crates.io | `ldtg` | `cargo search ldtg` ohne Treffer |
| PyPI | `ldtg` | kein exaktes Projekt gefunden |
| WinGet Community Repository | `LDTG` | kein exaktes Paket gefunden |
| DPMAregister-Basissuche | `LDTG`, `Local Device Transfer Gateway` | kein exakter Treffer im geprüften deutschen, EU- oder IR-Markenbestand |
| allgemeine Web- und GitHub-Suche | `LDTG`, `Local Device Transfer Gateway` | kein offensichtlicher exakter Konflikt mit einer Dateiübertragungssoftware |

Das Kürzel besitzt fachfremde Verwendungen, unter anderem als historische
Dateiendung von Logo Design Studio und als Schreibweise `LDTg` in der
Neurowissenschaft. Diese Treffer ändern die Produktentscheidung nicht, werden
aber als verbleibendes Auffindbarkeitsrisiko akzeptiert.

Quellen für die reproduzierbare Vorprüfung:

- [npm-Registry](https://registry.npmjs.org/ldtg)
- [crates.io-Suche](https://crates.io/search?q=ldtg)
- [PyPI-Suche](https://pypi.org/search/?q=ldtg)
- [WinGet Community Repository](https://github.com/microsoft/winget-pkgs)
- [DPMAregister-Basissuche](https://register.dpma.de/DPMAregister/marke/basis)
- [GitHub-Repositorysuche](https://github.com/search?q=LDTG&type=repositories)

Die Prüfung ist eine offensichtliche Kollisions- und Auffindbarkeitsprüfung,
keine formale Markenfreigabe, Ähnlichkeitssuche oder Rechtsberatung.

## Markenassets

- `assets/ldtg-logo-lockup.png`: ausgewähltes horizontales Wortzeichen, mit
  KI-Unterstützung erzeugt.
- `assets/ldtg-app-icon.png`: aus der gewählten Richtung abgeleitetes
  quadratisches App-Zeichen, mit KI-Unterstützung erzeugt.
- `assets/ldtg-ui-icon.png`: für die kleinen Markenflächen optimierte
  128-Pixel-Ableitung des App-Zeichens.
- `src-tauri/icons/**`: mit der Tauri-CLI reproduzierbar aus
  `assets/ldtg-app-icon.png` erzeugte Plattform- und Installericons.

Der frühere Markenassetbestand wird nicht weiter ausgeliefert. Die vollständige
Owner-Bestätigung für Quellcode, Texte, generierte Assets und QA-Testdaten bleibt
separat unter `PB-02` offen.

## Repositoryziel

Der vorgesehene öffentliche Repositoryname lautet
`LocalDeviceTransferGateway-LDTG`. Die Umbenennung des GitHub-Repositorys folgt
erst nach einem lokal erfolgreichen Build- und Prüfstand.

Damit ist die Owner-Entscheidung aus `PB-03` dokumentiert und das Namensrisiko
für die weitere Vorbereitung bewusst eingegrenzt.
