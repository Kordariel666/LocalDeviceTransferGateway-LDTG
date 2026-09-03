# P1-Abhängigkeits- und Lizenz-Audit

Stand: 3. September 2026

Auditierter Quellstand: `af632670b29346f1700f97e1bc00048712a00475`

Ergebnis: **Abhängigkeitsanteil des P1-Gates erfüllt; kein Projektlizenzentscheid**

Dieses Audit ist eine technische Lizenzinventur, keine Rechtsberatung. Es
aktiviert weder `GPL-3.0-only` noch `Apache-2.0`, erzeugt noch kein finales
Auslieferungs-Notice und sagt keine SignPath-Annahme zu.

## Reproduzierbarkeit und Artefakte

Der Generator [`scripts/public-beta-audit.mjs`](../../scripts/public-beta-audit.mjs)
liest beide Lockfiles, Cargo-Metadaten für den vollständigen und den
Windows-spezifischen Abhängigkeitsgraphen sowie npm-Paketmetadaten. Der erste
vollständige Lauf darf fehlende plattformspezifische npm-Metadaten rein lesend
aus der öffentlichen Registry ergänzen; danach ist die eingecheckte Inventur
der Cache für exakt denselben Lockfile-Hash.

```powershell
# reproduzierbar/offline, solange Lockfiles und Inventur zusammenpassen
pnpm audit:public-beta

# nur nach einer Lockfile-Änderung und bewusster Netzfreigabe
pnpm audit:public-beta:online
```

| Artefakt | Zweck |
|---|---|
| [`dependency-licenses.json`](dependency-licenses.json) | vollständige versionsgenaue Lizenz-, Herkunfts-, Scope-, Hash- und Notice-Inventur |
| [`sbom.cdx.json`](sbom.cdx.json) | CycloneDX 1.6 SBOM-Entwurf mit PURLs, Hashes und Abhängigkeitskanten |
| `pnpm-lock.yaml` | SHA-256 `7019f5e2240e5e72806d2dbd69ef01170b08659ac2e6075f68fc82788d2b72d5` |
| `src-tauri/Cargo.lock` | SHA-256 `3d068ef7833c58e6cf8b1290a17eaf6c8b3edec47077e18284b9329bb8aa658d` |

Der Audit bricht ab, wenn ein Lockfile nicht geparst werden kann, Cargo die
gesperrte Auflösung nicht bestätigt, npm-Metadaten fehlen oder ein Paket keine
deklarierte Lizenz besitzt.

## Umfang und Lieferklassen

| Klasse | Pakete | Einordnung |
|---|---:|---|
| npm gesamt | 305 | alle über `pnpm-lock.yaml` erreichbaren direkten/transitiven Pakete einschließlich optionaler Plattformvarianten |
| npm direkte Abhängigkeiten | 23 | alle Workspaces zusammen |
| npm Frontend-Laufzeit | 7 | Bestandteil des gebauten Frontends |
| npm Entwicklung | 301 | Build-, Typ-, Lint- und Testgraph; Überschneidung mit Laufzeit möglich |
| Cargo gesamt | 552 | alle im Lockfile aufgelösten Registry-Pakete für alle Ziele |
| Cargo direkte Abhängigkeiten | 31 | normale, Build- und gegebenenfalls Testkanten des Workspace-Pakets |
| Cargo für `x86_64-pc-windows-msvc` aufgelöst | 358 | für den geplanten Windows-Build relevanter Graph |
| Cargo Windows-Laufzeitgraph | 337 | konservativ über normale Cargo-Kanten erreichbar |
| Cargo Windows-Buildgraph | 201 | über Build-Kanten erreichbar; Überschneidung mit Laufzeit möglich |
| Cargo Windows-Testgraph | 0 | keine eigenständige Dev-Kante im Rootmanifest |
| Pakete ohne deklarierte Lizenz | **0** | Gate-relevanter Befund |

Cargo behandelt Proc-Makros als normale Abhängigkeiten, obwohl sie beim
Kompilieren ausgeführt werden. Die Scope-Zahlen sind deshalb bewusst
konservativ und überlappend; für die Lizenzprüfung wird kein möglicherweise
ausgelieferter Bestandteil herausgerechnet.

Die sieben npm-Laufzeitpakete sind:

| Paket | Version | Lizenz |
|---|---:|---|
| `@tauri-apps/api` | 2.11.1 | `Apache-2.0 OR MIT` |
| `@tauri-apps/plugin-dialog` | 2.7.3 | `MIT OR Apache-2.0` |
| `@tauri-apps/plugin-notification` | 2.4.0 | `MIT OR Apache-2.0` |
| `qrcode.react` | 4.2.0 | `ISC` |
| `react` | 19.2.8 | `MIT` |
| `react-dom` | 19.2.8 | `MIT` |
| `scheduler` | 0.27.0 | `MIT` |

## Vollständige Lizenzverteilung

Die folgende Tabelle fasst alle 857 inventarisierten Paketversionen zusammen.
`OR`-Ausdrücke erlauben eine Lizenzwahl; `AND`-Ausdrücke verlangen die
Einhaltung aller genannten Bedingungen.

| SPDX-Ausdruck | Anzahl |
|---|---:|
| `MIT` | 358 |
| `MIT OR Apache-2.0` | 277 |
| `Apache-2.0 OR MIT` | 70 |
| `Apache-2.0` | 23 |
| `Unicode-3.0` | 18 |
| `Zlib OR Apache-2.0 OR MIT` | 17 |
| `MPL-2.0` | 17 |
| `ISC` | 14 |
| `Unlicense OR MIT` | 12 |
| `BSD-3-Clause` | 9 |
| `BSD-2-Clause` | 8 |
| `Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT` | 5 |
| `BlueOak-1.0.0` | 2 |
| `BSD-2-Clause OR Apache-2.0 OR MIT` | 2 |
| `BSD-3-Clause OR MIT OR Apache-2.0` | 2 |
| `MIT OR Apache-2.0 OR LGPL-2.1-or-later` | 2 |
| `MIT OR Apache-2.0 OR Zlib` | 2 |
| `MIT OR Zlib OR Apache-2.0` | 2 |
| `MIT-0` | 2 |
| alle übrigen Ausdrücke mit jeweils einem Paket | 12 |

Die zwölf Einzelvorkommen sind in `dependency-licenses.json` vollständig
aufgeführt: Kombinationen aus `0BSD`, Apache-2.0, BSL-1.0, BSD, CC0-1.0,
CC-BY-4.0, LLVM-Exception, MIT und Unicode-3.0. Es gibt keine
Source-available-, proprietäre oder unbekannte Lizenzdeklaration.

## Besonders zu behandelnde Lizenzen

| Lizenz/Familie | Betroffene Pakete | Reichweite | Bewertung/Auflage |
|---|---|---|---|
| `MPL-2.0` | Cargo: `cssparser`, `cssparser-macros`, `dtoa-short`, `option-ext`, `selectors`; npm: `lightningcss` plus 11 Plattformpakete | fünf Cargo-Pakete im Windows-Graph; npm nur Entwicklung | schwaches Copyleft auf Dateiebene; MPL-Text und Hinweise erhalten, Quellform der MPL-bestimmten Dateien samt Änderungen verfügbar halten |
| `CC-BY-4.0` | `caniuse-lite` | nur Entwicklung | nicht als separates Paket ausgeliefert; bei späterer Redistribution Attribution und Lizenzlink aufnehmen |
| `BlueOak-1.0.0` | `lru-cache`, `minimatch` | nur Entwicklung | permissiv, Lizenztext/Hinweis bei Redistribution beibehalten |
| `Unicode-3.0` | 18 Cargo-Pakete beziehungsweise Kombinationsausdrücke | Windows-Graph | Copyright-/Permission-Notice beibehalten |
| Apache mit LLVM-Exception | sechs Ausdrücke/Pakete | Cargo | Apache-Text und Exception gemeinsam erhalten |
| `Unlicense OR MIT` | zwölf npm-Pakete | Entwicklung | für ein einheitliches Notice-Verfahren die MIT-Alternative wählen und dokumentieren |
| `MIT OR Apache-2.0 OR LGPL-2.1-or-later` | `r-efi` 5.3.0 und 6.0.0 | nicht im Windows-Graph | MIT- oder Apache-Alternative wählen; keine Abhängigkeit von der LGPL-Alternative nötig |

Die fünf MPL-Cargo-Pakete besitzen in den geprüften Quellpaketen keinen an die
Quelldateien angehängten Exhibit-B-Hinweis „Incompatible With Secondary
Licenses“. Eine GPL-Kombination bleibt trotzdem an die MPL-2.0-Regeln gebunden;
die MPL-Dateien werden nicht stillschweigend zu LDTG-Projektcode.

## Vereinbarkeit der beiden Projektlizenzpfade

### `GPL-3.0-only`

Die ausgelieferten MIT-/BSD-/ISC-/Zlib-/Apache-2.0-/Unicode-Komponenten sind mit
einer GPLv3-Gesamtdistribution grundsätzlich kombinierbar, wenn ihre jeweiligen
Hinweise erhalten bleiben. Apache-2.0 ist mit GPLv3, nicht pauschal mit älteren
GPL-Versionen kompatibel; der betrachtete Pfad ist ausdrücklich
`GPL-3.0-only`. MPL-2.0 besitzt einen Mechanismus für Kombinationen mit GPLv3,
behält aber Pflichten für die MPL-bestimmten Dateien. Der geplante
Notice-/Quellnachweis muss diese fünf Windows-relevanten Cargo-Pakete daher
ausdrücklich führen.

### `Apache-2.0`

Die permissiven Abhängigkeiten sind mit einer Apache-2.0-Lizenz des eigenen
LDTG-Codes kombinierbar. MPL-2.0-Dateien bleiben unter MPL-2.0; eine
Apache-lizenzierte größere Arbeit ist möglich, darf die MPL-Dateipflichten aber
nicht überdecken. `AND`-Ausdrücke sowie Apache-`NOTICE`-Dateien müssen zusätzlich
vollständig übernommen werden.

### SignPath

Die Inventur enthält keinen proprietären oder Source-available-Baustein, der
eine vollständig offene Quellverteilung offensichtlich verhindert. Das ist
nur die Dependency-Voraussetzung. SignPath-Eignung setzt weiterhin eine
gewählte OSI-Lizenz, einen freigegebenen öffentlichen Quellstand, geklärte
Provenienz und den späteren Build-/Signaturprozess voraus. Diese Punkte bleiben
außerhalb des P1-Abhängigkeitsbefunds offen; ein Antrag wurde nicht gestartet.

## Notice- und Quellnachweisplan

Die Inventur hat bei 740 Paketversionen insgesamt 1.151 lokale
Lizenz-/Copyright-/Notice-Dateien mit relativen Pfaden und SHA-256 erfasst. Alle
sieben npm-Laufzeitpakete besitzen mindestens eine solche Datei. Bei 15
konservativ als Cargo-Windows-Laufzeit erreichbaren Paketen liegt im
Crate-Wurzelverzeichnis keine passend benannte Datei; deren deklarierter
SPDX-Ausdruck, Autoren, Repository, Version und Crate-Prüfsumme sind dennoch in
der Inventur festgehalten. Für diese Pakete wird der standardisierte SPDX-Text
zusammen mit den Paketmetadaten verwendet.

Ein späteres, durch P5 autorisiertes `THIRD_PARTY_NOTICES` muss aus dem exakt
gebauten Zielgraphen erzeugt und mindestens enthalten:

- Paketname, Version, gewählte Alternative bei `OR`, Autoren/Copyright und
  Repository;
- vollständige einschlägige Lizenztexte und vorhandene `NOTICE`-Dateien;
- bei `AND` alle Teilbedingungen;
- für MPL-2.0 den MPL-Text, den Hinweis auf die betroffenen Quellen und einen
  dauerhaft erreichbaren Quellstand exakt dieser Versionen samt Änderungen;
- Prüfsummenbezug zu den beiden Lockfiles und zum veröffentlichten SBOM.

Für nicht ausgelieferte Build-/Testwerkzeuge bleibt die Inventur als
Auditnachweis erhalten; ihre Texte müssen nicht irreführend als Bestandteil der
Binärdistribution dargestellt werden. Der finale Zielgraph und das Notice-
Bündel werden erst nach Projektlizenz- und Releaseentscheidung festgeschrieben.

## Gatebewertung

- Versionsgenaue direkte/transitive Inventur: **erfüllt**.
- Build-/Test-/Laufzeit- und Windows-Zielbezug: **erfüllt, konservativ und
  maschinenlesbar**.
- Unbekannte, proprietäre oder Source-available-Lizenzen: **keine**.
- Vereinbarkeit mit `GPL-3.0-only` und `Apache-2.0`: **für den geprüften Graphen
  unter den genannten Notice-/MPL-Auflagen gegeben**.
- Drittanbieterhinweise: **vollständig aus Inventur, Standardtexten und
  Paketquellen ableitbar; finales Bündel noch nicht aktiviert**.
- SignPath: **kein Dependency-Blocker erkannt, Gesamteignung noch offen**.

Damit ist der Abhängigkeitsanteil von P1 geschlossen. P1 insgesamt bleibt wegen
der drei Repository-/Owner-Blocker aus [`blockers.json`](blockers.json) offen.
