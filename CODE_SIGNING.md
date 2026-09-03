# Code signing policy (Entwurf)

Stand: 4. September 2026
Status: **Entwurf; keine SignPath-Zusage, kein Zertifikat und keine aktive Signierung**

LDTG wird derzeit weder veröffentlicht noch signiert. Es wurde kein SignPath-
Antrag begonnen, kein Konto oder kostenpflichtiger Dienst eingerichtet und kein
Schlüsselmaterial erzeugt. Dieser Entwurf beschreibt lediglich eine später
entscheidbare, SignPath-kompatible Trennung.

## Getrennte Vertrauensgrenzen

1. **Build:** Der private Releasepfad baut aus einem sauberen, vollständigen
   Commit ein unsigniertes Artefakt. Er besitzt nur Leserechte und gibt Commit,
   Lockfiles, SBOM, Buildlog, Manifest und SHA-256 gemeinsam aus.
2. **Manuelle Freigabe:** Ein Owner vergleicht Installerhash, Quellcommit,
   bestandenes Gate, SBOM und QA-Matrix. Dieser Schritt baut keine Datei neu und
   ist weder durch P3 noch durch `PB-04` freigegeben.
3. **Signierung:** Ein späterer isolierter Dienst darf ausschließlich den exakt
   freigegebenen Installerhash signieren. Buildjobs erhalten keinen privaten
   Schlüssel und keinen allgemeinen Release-Schreibzugriff.
4. **Nachweis:** Der signierte Installer ist ein neues Artefakt. Signaturstatus,
   Zertifikatkette, Zeitstempel und neuer SHA-256 werden separat geprüft und an
   das unveränderte Buildmanifest gekoppelt.
5. **Veröffentlichung:** Upload und Sichtbarkeitswechsel bleiben ein nochmals
   getrennter, ausdrücklich genehmigter Schritt nach dem finalen `GO`.

## Mindestanforderungen an eine spätere Umsetzung

- Signieranforderungen müssen den erwarteten vollständigen SHA-256 als
  unveränderliche Eingabe verwenden; Dateiname oder Branchname reichen nicht.
- Schlüssel oder Tokens dürfen weder im Repository noch in Buildlogs oder
  allgemeinen Runnerartefakten stehen.
- Rollen für Build, Freigabe, Signierung und Veröffentlichung erhalten jeweils
  nur die minimal nötigen Rechte.
- Ein Signierfehler, Hashkonflikt, abgelaufenes beziehungsweise widerrufenes
  Zertifikat oder fehlgeschlagene Authenticodeprüfung blockiert die
  Veröffentlichung.
- Die Prüfsummen des unsignierten und signierten Artefakts dürfen nicht
  verwechselt werden; beide erhalten einen eindeutigen Status.
- Zertifikatsidentität, Zeitstempeldienst, Widerrufsweg, Aufbewahrung und
  verantwortliche Personen müssen vor Aktivierung ausdrücklich entschieden
  werden.

## SignPath-Stand und geplanter Weg

Die am 4. September 2026 geprüften Bedingungen der SignPath Foundation
verlangen unter anderem eine OSI-anerkannte Lizenz ohne kommerzielle
Doppellizenzierung, vollständig offenen Projektcode, aktive Wartung und eine
bereits veröffentlichte Version in der Form, die später signiert werden soll.
Sie verlangen außerdem MFA, getrennte Rollen, manuelle Freigabe jedes
Signierauftrags, einen verifizierbaren Build und eine öffentlich verlinkte
„Code signing policy“. Datenschutz, angekündigte Systemänderungen und ein
funktionierender Deinstallationsweg gehören ebenfalls zum Prüfrahmen.

Damit ist für den kostenlosen Foundation-Pfad folgende Reihenfolge vorgesehen:

1. eine ausdrücklich freigegebene erste öffentliche Beta ohne behauptete
   Signatur veröffentlichen;
2. öffentliche Herkunft, Buildpfad, Wartung und Releaseform nachweisen;
3. erst danach einen SignPath-Antrag separat genehmigen und stellen;
4. nach einer eventuellen Annahme die konkrete Rollen- und
   Repositorykonfiguration nochmals prüfen;
5. nur neu gebaute und eindeutig als signiert ausgewiesene Folgeartefakte
   veröffentlichen.

Eine Annahme ist kein Anspruch und wird insbesondere bei einem neuen Projekt
nicht vorausgesetzt. Die vollständige Vorprüfung und die offenen Entscheidungen
stehen in der
[P5-Entscheidungsmappe](qa/public-beta/p5-license-contribution-signpath.md).
Die Projektlizenz `Apache-2.0` ist aktiviert. R5.2, Phase 6, Anmeldung und
Veröffentlichung bleiben bis zu den vorgesehenen Freigaben pausiert. Eine
unsignierte private P3-Datei ist kein
Downloadangebot und begründet keine Aussage über Publisher-Identität,
SmartScreen-Reputation oder zukünftige Signierbarkeit.

Quelle: [SignPath Foundation conditions for Open Source projects](https://signpath.org/terms.html)
