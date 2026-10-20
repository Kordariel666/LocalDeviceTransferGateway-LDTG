# P04a – Ordner als ZIP herunterladen

Stand: 20.10.2026, lokale Implementierung auf `dev`; keine Veröffentlichung.

## Bedienung und API

Mobile bietet pro sichtbarem Ordner „Als ZIP herunterladen“ und für den aktuellen
Ordner einschließlich Freigabewurzel „Ordner als ZIP herunterladen“ an.
Navigation und Einzeldateilinks bleiben erhalten. Suche und Listingseiten
beschränken das Archiv **nicht**: Es enthält den gesamten freigegebenen Inhalt
des gewählten Ordners, einschließlich Unterordnern und expliziten leeren Ordnern.
Die ZIP enthält einen obersten Ordner mit dem Namen des gewählten Verzeichnisses
(bei einer Laufwerkswurzel `Freigabe`). Sichere Unicode-Namen bleiben erhalten.

`GET /api/v1/download-folder?path=<URL-kodierter relativer Ordnerpfad>`
liefert `application/zip`, Attachment mit UTF-8-Dateiname, `Content-Length`,
`Accept-Ranges: none` und die bestehenden Sicherheits-/No-Store-Header.
`path=` exportiert die Freigabewurzel. HEAD liefert 405, Range liefert 416 mit
`ARCHIVE_RANGE_UNSUPPORTED`; Wiederholung startet einen neuen vollständigen Export.
Kein Ordnerupload und keine Mehrfachauswahl wurden ergänzt.

## Implementierung und Schutzgrenzen

[Rust-Endpunkt](../../src-tauri/src/service/api/folder_download.rs) und
[mobile Oberfläche](../../apps/mobile/src/DirectoryBrowser.tsx).
Ein begrenzter Blockingjob ermittelt ausschließlich Eintragsmetadaten und die
exakte Archivgröße. Danach erzeugt ein nachfragegesteuerter asynchroner Body
ZIP32 mit STORE (keine Kompression), UTF-8-Flag, CRC32 und Data Descriptors nach
[PKWARE APPNOTE](https://pkware.cachefly.net/webdocs/casestudies/APPNOTE.TXT).
CRC- und Stream-Trait stammen aus bereits im Lockfile vorhandenen Crates;
es wurden keine Paketversionen aktualisiert.

Dateibytes werden erst beim Lesen des Response-Bodys geöffnet und in höchstens
256-KiB-Blöcken gelesen. Es gibt keinen vorauseilenden Archivproduzenten, keine
Datei-/Archivpufferung und keine temporäre ZIP auf dem Server. Manifest und
Zentralverzeichniswerte sind durch Eintrags-/Namenslimits beschränkt; jeder
Zentralverzeichniseintrag wird einzeln ausgegeben. Zusätzlich zum Lesepuffer hält
der Stream höchstens den aktuellen kopierten Byteblock und kleine ZIP-Header.
Der Speicherbedarf hängt von begrenzten Metadaten, nicht von den Dateigrößen ab.

Authentisierung, IP-gebundene Sitzung, aktive Downloadfreigabe, Host-/Subnetz-
und Requestguards, Download-Leases und Dateisystem-Permits werden wiederverwendet.
Die gesamte Vorbereitung belegt bereits einen Downloadslot. Der Scan hält einen
Filesystem-Lookup-Slot; jede spätere Dateiöffnung benötigt erneut ein Permit.
`safe_existing`, `RootAnchor::open_existing` und die bestehende sichere
Dateiöffnung prüfen Komponenten, Rootidentität und den final geöffneten Handle.
Auch ein nach dem Scan durch eine externe Junction ersetzter Unterordner darf
keine Bytes nach außerhalb der Freigabe liefern.

Symlinks/Junctions/Reparse Points sowie versteckte, System- und verwaltete Inhalte
werden beim rekursiven Scan ausgeschlossen, entsprechend dem Listing. Wird ein
solcher Pfad direkt angefordert, folgt `PATH_UNAVAILABLE`. Andere nicht sicher
lesbare Inhalte brechen den Export ab. Archivnamen entstehen ausschließlich aus
geprüften einzelnen Namen mit `/` als Trenner; Parent-/absolute Komponenten,
ADS, Separatoren im Namen, Steuer-/Bidi-Zeichen, reservierte Windowsnamen und
mehrdeutige Endpunkte mit Leerzeichen/Punkt werden abgewiesen statt umbenannt.
Es werden keine absoluten Hostpfade oder Linkeinträge archiviert.

## Feste Limits

| Ressource | Grenze |
|---|---|
| Gesamtgröße der ZIP einschließlich aller Header | 1 GiB |
| Datei-/Verzeichniseinträge einschließlich oberstem Ordner | 10.000 |
| Tiefe relativ zum ausgewählten Ordner | 32 Komponenten; oberster Ordner Tiefe 0 |
| UTF-8-Archivname einschließlich oberstem Ordner und Schrägstrichen | 1.024 Bytes |
| Summe der Archivnamen einschließlich vorgemerkter Einträge | 4 MiB |
| Relativer Auswahlpfad | 1.024 native Pfadbytes |
| Untersuchte Einträge einschließlich ausgeschlossener Inhalte | 20.000 |
| Metadatenscan | 30 Sekunden |
| Downloadparallelität, gemeinsam mit Einzeldateien | 12 global / 4 pro IP / 3 pro Sitzung |
| Dateisystemprüfungen, gemeinsam mit vorhandenen Downloads | 4 global / 2 pro IP |
| Laufzeit, Vorbereitung eingeschlossen | bestehende Download-Lease, maximal 6 Stunden |

Limitüberschreitungen vor dem Versand liefern 413/`ARCHIVE_LIMIT`; Scanabbruch
oder Scanfrist 408/`ARCHIVE_SCAN_CANCELLED`. Vorbereitungsfehler senden keine ZIP.
Nach Responsebeginn beenden I/O-/Schutzfehler den Body mit Transportfehler, ohne
gültiges ZIP-Endverzeichnis. Die angekündigte Länge hilft dem Browser, einen
unvollständigen Download zu erkennen. Browser zeigen die JSON-Fehler einer
Attachmentnavigation möglicherweise außerhalb der LDTG-Seite an.

## Abbruch und veränderliche Inhalte

Das Verwerfen des Response-Bodys bei einem Browserdisconnect verwirft den
Generator und seine offenen Dateien. Der bestehende DownloadGuard meldet Abbruch
und gibt den Slot frei. Sitzungswiderruf/Dienststopp sowie die Leasefrist gewinnen
auch gegen gerade wartende Reads. Ein verworfener Scan-Waiter setzt ein atomares
Abbruchsignal; der Blockingjob prüft es zwischen Einträgen und gibt sein Permit
nach Rückkehr frei. Einzelne blockierende OS-Dateisystemaufrufe lassen sich nicht
gewaltsam abbrechen; die Anzahl solcher Jobs bleibt begrenzt.

Kein Dateisystemsnapshot: lokale Änderungen während des Exports sind möglich.
Geänderte Dateigrößen, verschwundene Dateien und nicht mehr sichere Pfade brechen
ab. Gleich große In-Place-Änderungen werden nicht als Versionswechsel erkannt.
Nach dem Scan neu angelegte Einträge sind nicht enthalten; zuvor erfasste leere
Verzeichnisse können im Archiv verbleiben, auch wenn sie lokal entfernt werden.
Zeitstempel werden fest auf 01.01.1980 gesetzt; ACLs, ADS, Links und sonstige
Dateisystemmetadaten werden nicht übertragen. Die bestehende Grenze für
Range-Resume einzelner Dateien wird nicht verändert.

## Verifikation und Browserannahmen

[Rusttests](../../src-tauri/src/service/api/folder_download/tests.rs) prüfen
verschachtelte/leere Ordner, Unicode, gleiche Basenames in verschiedenen Pfaden,
Null-Byte-Dateien, exakte Länge, einen unabhängigen Windows/.NET-ZIP-Reader,
begrenzte Frames, verzögerte Dateiöffnung, Body-Drop auch ohne vorheriges Polling,
einzelnen/globalen Sitzungswiderruf, Quoten, Scanfrist/-abbruch, manipulierte Pfade
und echte Windows-Junctions einschließlich Junctionwechsel nach dem Scan.
Der zusätzliche echte Datei-/Verzeichnissymlink-Test ist explizit ignoriert:
seine Fixture benötigt Windows Developer Mode oder `SeCreateSymbolicLinkPrivilege`.
Der Versuch in dieser Sitzung scheiterte mit OS-Fehler 1314. Der gemeinsame
Reparse-Point-Schutz läuft im Junctiontest real; eine eigenständige Symlink-
Laufzeitprüfung bleibt unter dieser Windows-Voraussetzung offen.
[UI-Tests](../../apps/mobile/src/DirectoryBrowser.test.tsx) prüfen ZIP-Links,
URL-Kodierung, leere Freigabe und unveränderte Navigation/Einzeldateilinks.
Bestehende Datei-/Range-/HEAD-Tests gehören weiterhin zum vollständigen Rustlauf.

Vollständiges `pnpm check`: **PASS**, Exit 0, mit Node **24.19.0**, pnpm
**11.19.0**, Rust **1.98.0**. Enthalten: Vertrags-/Notice-/Markdownprüfung,
Typecheck, ESLint, 16 Skripttests, Coverage mit 50 Desktop- und 52 Mobiletests,
beide Webbuilds, 140 erfolgreiche Rusttests bei einem explizit ignorierten
Symlinktest, `cargo fmt --check` und Clippy für alle Targets mit `-D warnings`.
Vorhandene Einzeldatei-/Range-/HEAD-Regressionen bestanden ebenfalls.
Die echte Symlinkfixture wurde zusätzlich versucht und scheiterte bereits an der
oben genannten Windows-Berechtigung; dies ist kein bestandener Symlinknachweis.

Die Oberfläche benutzt normale gleichursprüngliche Attachmentlinks mit Cookie;
sie lädt das Archiv nicht per Fetch/Blob in den Browserspeicher. Fortschritt,
Abbruch, Speicherort und anschließendes Entpacken liegen beim Browser bzw. OS.
ZIP-Unterstützung wird für aktuelle Browserdownloadmanager und UTF-8-fähige
Entpacker angenommen. Android Firefox/Chrome und iOS/iPadOS Safari wurden für
dieses Feature nicht an echten Geräten geprüft. Der .NET-Test belegt das
Archivformat, nicht mobiles Speicher-/Hintergrundverhalten oder Human-Abnahme.
