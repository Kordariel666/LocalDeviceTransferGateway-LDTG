# Veröffentlichungsentscheidung

Stand: 4. September 2026
Status: **noch nicht entschieden; keine Veröffentlichungsfreigabe**

P0 bis P6 sind für den unveröffentlichten, unsignierten Kandidaten
`0.3.0-rc.2` abgeschlossen. Dieses Dokument bereitet nur die bewusste
Owner-Entscheidung vor und löst selbst keine externe Änderung aus.

## Betrachteter Kandidat

- Quellcommit: `a70c1612f2c673b0be7cede2676b851ae134eccd`
- vorgesehener Tag: `v0.3.0-rc.2` – nicht angelegt
- Installer SHA-256:
  `1cc647a3db9eb874bf620efba58c9e57310ab756ebc8d943715f059f5c4cb9fc`
- Lizenz: `Apache-2.0`
- öffentliche Copyright-Bezeichnung: `Kordariel666`
- Beiträge: zunächst nur Issues, keine Pull Requests
- Signatur: keine; mögliche Windows-Warnung ist in den Release-Notes genannt
- Support: neueste Beta, Windows 11 25H2 und real geprüftes Android 16 mit
  Firefox; Best Effort ohne SLA
- wiederkehrende Kosten: 0 Euro

## Mögliche Entscheidungen

- `GO-APACHE`: genau diesen Kandidaten unter Apache-2.0 als ausdrücklich
  unsignierte erste öffentliche Beta freigeben;
- `HOLD`: privat und unveröffentlicht lassen, ohne Rückbau;
- `ARCHIVE`: Entwicklung beenden und keine Beta veröffentlichen;
- `COMMERCIAL-DISCOVERY`: öffentliche Beta anhalten und zuerst einen getrennten
  Geschäfts-/Lizenzweg bewerten.

`GO-GPL` ist für diesen Kandidaten nicht mehr passend, weil Apache-2.0 bereits
bewusst aktiviert wurde. Ein Lizenzwechsel wäre ein neues Arbeitspaket und
keine bloße Veröffentlichungsentscheidung.

## Vor einem späteren `GO-APACHE`

Der Owner bestätigt in einem eigenen Schritt:

1. den betrachteten Commit und Installerhash;
2. Apache-2.0 und `Kordariel666` als öffentliche Angaben;
3. den Issues-only-Beitragsmodus;
4. den begrenzten Support- und Wartungsumfang;
5. die bekannte unsignierte Windows-Warnung und die dokumentierten
   Plattform-, Stress- und Barrierefreiheitslücken;
6. dass Repositorysichtbarkeit, Tag, Push und GitHub Release extern geändert
   werden dürfen.

Private Vulnerability Reporting wird bei einem späteren `GO-APACHE` unmittelbar
nach dem Sichtbarkeitswechsel praktisch geprüft und muss vor Bereitstellung des
Binärdownloads funktionieren. Ein eventueller SignPath-Antrag bleibt eine
spätere, separat zu genehmigende Entscheidung. P6 hat beides nicht aktiviert
und keine kostenpflichtige Verpflichtung erzeugt.
