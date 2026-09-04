# Veröffentlichungsentscheidung

Stand: 4. September 2026
Status: **private Prerelease vorbereitet; keine öffentliche Freigabe**

P0 bis P6 sind für den nicht öffentlich freigegebenen, unsignierten Kandidaten
`0.3.0-rc.2` abgeschlossen. Tag, Push und die GitHub-Prerelease wurden nach
ausdrücklicher Owner-Bestätigung im privaten Repository vorbereitet. Der
Sichtbarkeitswechsel bleibt eine getrennte Owner-Entscheidung.

## Betrachteter Kandidat

- Quellcommit: `4956cd3de88d6d5d5cff6c3653a71d9695a60c32`
- Tag: `v0.3.0-rc.2` – angelegt, gepusht und auf den Quellcommit gebunden
- private Prerelease:
  `https://github.com/Kordariel666/LocalDeviceTransferGateway-LDTG/releases/tag/v0.3.0-rc.2`
- Installer SHA-256:
  `7c7263d37e94cac525ac7de6d5ec1ffaf3c3a3ed680dc13e79017886c19d6f9c`
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

## Bestätigter Vorbereitungsstand

Der Owner hat bestätigt:

1. die am 4. September 2026 ohne Fund abgeschlossene gezielte
   Microsoft-Defender-Prüfung genau des oben gehashten finalen Installers;
2. den betrachteten Commit und Installerhash;
3. Apache-2.0 und `Kordariel666` als öffentliche Angaben;
4. den Issues-only-Beitragsmodus;
5. den begrenzten Support- und Wartungsumfang;
6. die bekannte unsignierte Windows-Warnung und die dokumentierten
   Plattform-, Stress- und Barrierefreiheitslücken;
7. dass Commit, Push, Tag und die private GitHub-Prerelease vorbereitet werden
   dürfen.

Für ein späteres `GO-APACHE` fehlt nur noch die gesonderte Bestätigung, das
Repository auf öffentlich zu stellen. Dieser Sichtbarkeitswechsel macht Code,
Tag, Prerelease und deren Anhänge öffentlich.

Private Vulnerability Reporting wird bei einem späteren `GO-APACHE` unmittelbar
nach dem Sichtbarkeitswechsel praktisch geprüft und muss vor Bereitstellung des
Binärdownloads funktionieren. Ein eventueller SignPath-Antrag bleibt eine
spätere, separat zu genehmigende Entscheidung. P6 hat beides nicht aktiviert
und keine kostenpflichtige Verpflichtung erzeugt.
