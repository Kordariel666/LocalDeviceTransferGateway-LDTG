# Entwurf zur Codesignierung

Stand: 3. September 2026  
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

## Offene Entscheidung

SignPath-Eignung, Lizenzvoraussetzungen und konkrete Anbieterbedingungen werden
erst im dafür vorgesehenen P5-Entwurf bewertet. R5.2, Phase 6, Lizenzaktivierung
und jede tatsächliche Anmeldung bleiben pausiert. Eine unsignierte private
P3-Datei ist kein Downloadangebot und begründet keine Aussage über Publisher-
Identität, SmartScreen-Reputation oder zukünftige Signierbarkeit.
