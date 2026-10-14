# GridfinityStudio

Native Windows-Desktop-App in Rust für Gridfinity-Behälter, Grundplatten,
Werkzeugeinsätze, Einschubschilder und Tischboxen mit Schubladen. Die Modellberechnung läuft lokal.

[Portable Windows-Versionen](https://github.com/LucasWeiss36/gridfinity-studio/releases)
· [Quellcode](https://github.com/LucasWeiss36/gridfinity-studio)

Entwürfe und Einstellungen liegen unter `%LOCALAPPDATA%\GridfinityStudioDesign2`. Vorhandene `.gridfinity.json`-Dateien lassen sich weiterhin öffnen. Externe Werkzeug-STLs müssen erhalten bleiben.

## Starten

Zum Benutzen der App die Windows-x64-ZIP unter **Releases** herunterladen und
vollständig entpacken. Die GitHub-Schaltfläche **Code → Download ZIP** enthält
den Quellcode; daraus muss die Anwendung zunächst gebaut werden.

Im Ordner **GridfinityStudio** die `GridfinityStudio.exe` öffnen.
Eine optionale Verknüpfung heißt **GridfinityStudio**.
Alternativ `Start.cmd` verwenden. Auf der Startseite links einen Modelltyp im
Katalog auswählen. Rechts erscheinen die große Vorschau und die Schaltfläche
zum Erstellen, zum Beispiel **Kasten erstellen**.
Unter **Datei → Beispiele** stehen unter anderem **Fachbox**, **Werkzeugeinsatz**
und **Schubladenbox** bereit.

Die linke Leiste führt zu **Start**, **Entwurf**, **Projekte**, **Drucker** und
**Hilfe**. Im Editor stehen rechts direkt die zum Modell passenden
Eigenschaften: Abmessungen, Ausführung, Wände, Aussparungen und Beschriftung.
Die Abschnitte lassen sich nach Bedarf aufklappen.

Unten links zwischen **Hell** und **Dunkel** wechseln. Die Auswahl gilt für
die Oberfläche und Vorschau und wird unabhängig vom Modell unter
`%LOCALAPPDATA%\GridfinityStudioDesign2\appearance.json` gespeichert.

1. Links **Drucker** öffnen: Druckerprofil auswählen oder eigene Maße eingeben.
   Im Entwurf rechts unter **Abmessungen** Rastergröße und Höhe einstellen.
   Die 3D-Vorschau aktualisiert sich.
2. Mit der Maus die Ansicht drehen, mit dem Mausrad zoomen. Shift+Ziehen
   verschiebt die Ansicht. „Passend zur Ansicht“ setzt die Kamera zurück.
   Die X/Y/Z-Knöpfe unten rechts zeigen Ansichten entlang der gewählten Achse.
3. Rechts unter **Elemente → Hinzufügen +** eine Wand hinzufügen. Über **Grundriss
   bearbeiten** zur maßhaltigen Wandbearbeitung wechseln. Unter der aufklappbaren
   **Fachaufteilung** wählen: **Keine Trennwände**,
   **Gleichmäßige Fächer** mit einstellbaren Reihen und Spalten oder **Eigene
   Wände**. Gleichmäßige Fächer erhalten dieselben freien Innenmaße je Richtung;
   die Stärke der Trennwände wird dabei mitgerechnet. Bei eigenen Wänden eine
   waagerechte/senkrechte Wand hinzufügen oder
   **Freie Wand ziehen** wählen und im Grundriss drücken und ziehen.
   Ganze Wände lassen sich verschieben; runde Griffe ändern die Endpunkte.
   Beim Zeichnen und Ziehen eines Endpunkts rastet dieser an nahen Wänden ein.
   T-Anschlüsse sind bündig, auch bei schrägen Wänden oder verschiedenen Stärken.
   Strg fixiert beim Zeichnen, Verschieben und Ändern der Endpunkte die Achse;
   Shift fixiert beim Zeichnen ebenfalls die Achse. Escape bricht das Zeichnen ab.
   Stärke, Höhe ab Innenboden, Länge und exakte Koordinaten stehen im
   Wandinspektor links neben dem Grundriss.
   **Maßzahlen direkt im Grundriss anklicken** und den gewünschten Millimeterwert
   eingeben (Komma oder Punkt). **Enter** übernimmt, **Escape** oder ein Klick
   außerhalb bricht ab. Linkes/rechtes bzw. unteres/oberes Fachmaß verschiebt die
   Wand; die Wandlänge verändert den zweiten Endpunkt bei gleichem Winkel.
   Nachbarwände und Wandstärken werden berücksichtigt. Zu große Maße werden
   mit einem Hinweis abgewiesen; das Modell bleibt unverändert. Exakte Eingaben
   werden unabhängig vom Zeichenraster übernommen. Wände lassen sich
   duplizieren und um 90° drehen.
   Rückgängig/Wiederholen und Strg+Z/Strg+Shift+Z gelten für die Fachwände.
   **Keine Trennwände** entfernt die inneren Wände auch aus dem Druckmodell;
   die Außenwand bleibt erhalten. Das optionale Hilfsraster wird nicht gedruckt.
4. Für Werkzeugeinsätze Aussparungen hinzufügen: rund, abgerundet, Sechskant
   oder STL-Werkzeug. Position, Drehung, Tiefe und Maßzugabe einstellen.
   Aussparungen können im Grundriss ausgewählt und verschoben werden.
5. Rechts unter **Beschriftung** zwischen **Direkter Text**, **Etikettenfeld** und
   **Einschubschild** wählen. Direkter Text wird erhaben oder graviert Teil der
   Box; ein Etikettenfeld bleibt zum Bekleben frei. Ein Einschubschild ist ein
   separates Druckteil mit optionalem Text. Die Blockschrift unterstützt
   Großbuchstaben, Zahlen, Leerzeichen und Bindestriche sowie **Ä, Ö, Ü und ß**
   als eigene Zeichen. Kleinbuchstaben werden groß dargestellt; **ß** bleibt erhalten.
   Auf Schubladengriffen kurze Texte wie BITS verwenden.
6. Entwürfe in der Projektablage speichern und später öffnen. Einzelteile oder alle Teile
   als STL exportieren. Während einer Neuberechnung bleibt Export gesperrt.

## Drucker und Bauraum

Die Druckerauswahl ist über **Drucker** in der linken Leiste erreichbar.
Der Dialog zeigt auch die Bauraumprüfung für den aktuell geöffneten Entwurf.
Profile von **Bambu Lab, Prusa, Creality, Elegoo und Anycubic** sind enthalten.
Unter **Benutzerdefiniert** Breite X, Tiefe Y und Druckhöhe Z in Millimetern
eintragen. Jedes Profil lässt sich als eigene Vorgabe übernehmen und anpassen.
Ein optionaler **freier Rand je Seite** wird beidseitig von Breite und Tiefe
abgezogen, etwa für einen Brim. Standardmäßig ist dieser Rand 0 mm.

Die App zeigt die maximalen 42-mm-Rasterfelder auf der rechteckigen Fläche
und prüft danach die tatsächlichen Abmessungen jedes Druckteils einschließlich
der Höhe. Bei 256 × 256 mm ohne Rand sind zum Beispiel 6 × 6 Rasterfelder
möglich, bei 220 × 220 mm 5 × 5. Die Auswahl verändert oder verkleinert
keinen Entwurf automatisch. Die allgemeine Rastergrenze der App bleibt bei
8 × 8 Feldern, auch wenn ein größerer Drucker mehr Platz hätte.

**Export** zeigt für jedes Teil: passt, passt nach 90°-Drehung auf dem Bett
oder überschreitet den Bauraum. Ein Einschubschild wird dabei in seiner
flachen STL-Drucklage geprüft; die montierte Baugruppe ist nicht das Druckmaß.
Eine erforderliche Drehung um 90° selbst im Slicer vornehmen. Die Prüfung
bezieht sich auf jedes Teil einzeln, nicht auf die gemeinsame Belegung einer
Druckplatte. Übergroße Teile bleiben exportierbar, zum Beispiel zum späteren
Teilen im Slicer. Große Grundplatten können stattdessen als mehrere kleinere
Platten mit Clip-Aufnahmen geplant werden; automatisches Aufteilen ist nicht enthalten.

Die Profile beschreiben rechteckige Bauraummaße. Druckerspezifische Sperrflächen,
Wischbereiche, Stützen und Slicer-Einstellungen werden nicht geometrisch geprüft.
Bei Mehrdüsendruckern auf den richtigen Modus achten. Quellen, hinterlegte
Maße und Einschränkungen stehen in [PRINTERS.md](../PRINTERS.md).
Die Auswahl wird unabhängig vom Entwurf in
`%LOCALAPPDATA%\GridfinityStudioDesign2\printer.json` gespeichert. Sie bleibt nach
einem Programmupdate erhalten und ist nicht in der portablen ZIP enthalten.

## Projekte und Ordner

**Projekte** in der linken Leiste oder oben im Editor öffnet **Meine Projekte**.
Mit **Projekt anlegen** etwa „Schrank“ erstellen. Darin mit **Ordner anlegen**
Unterordner wie „Kabel“, „Stifte“ oder „Werkzeug“ anlegen; weitere Unterordner
sind ebenfalls möglich. Ein Doppelklick oder **Öffnen** öffnet den markierten
Ordner bzw. Entwurf. Über die Pfadleiste kommst du wieder nach oben.

**Speichern** öffnet die Ablage für den aktuellen Entwurf: Projekt und Ordner
wählen, einen Entwurfsnamen vergeben und **Hier speichern** drücken. Mehrere
Modelle können im selben Ordner liegen, etwa Grundplatte, Clip und Stiftebox.
Vor dem Ersetzen einer bestehenden Datei fragt die App nach.
Projekte, Ordner und Entwurfsdateien lassen sich über **Umbenennen** benennen.

Ungespeicherte Änderungen werden vor einem neuen Modell, dem Laden eines
Beispiels, dem Öffnen eines anderen Entwurfs aus der Ablage oder einer externen
Projektdatei und beim Schließen der App abgefragt. Du kannst zuerst speichern,
die Änderungen verwerfen oder abbrechen. Beim Speichern wird der Wechsel erst
nach erfolgreichem Abschluss fortgesetzt; ein Fehler oder ein abgebrochener
Speicherdialog lässt den bisherigen Entwurf offen.

Zum Löschen einen Eintrag markieren und **Löschen …** wählen. Die Bestätigung
nennt den Eintrag; bei Projekten und Ordnern werden auch sämtliche Unterordner
und Dateien darin gelöscht. **Endgültig löschen** entfernt sie ohne Windows-
Papierkorb. **Abbrechen**, Escape oder ein Klick außerhalb der Bestätigung
verändert nichts. Ein betroffener offener Entwurf bleibt im Editor erhalten
und muss anschließend neu gespeichert werden. Der Hauptordner der Ablage
kann in der App nicht gelöscht werden.

Die Ablage liegt unter `%LOCALAPPDATA%\GridfinityStudioDesign2\Projekte` und bleibt
bei einem EXE-Update erhalten. Es sind normale Ordner mit `.gridfinity.json`
Dateien. Zum Sichern oder Übertragen auf einen anderen PC den gesamten
Projektordner kopieren. Die Programm-ZIP enthält keine privaten Projekte.
Externe Werkzeug-STLs weiterhin zusätzlich mitnehmen und gegebenenfalls neu
auswählen. **Datei → Projektdatei öffnen** liest bisherige Einzeldateien;
über **Speichern** kannst du sie danach in die Ablage aufnehmen. Für eine
einzelne Datei außerhalb der Ablage gibt es **Datei → Als Projektdatei speichern**.

STL-Dateien verwenden Millimeter. Gehäuse und jede Schublade werden separat
exportiert; die geöffnete Stellung in der Vorschau verändert die Druckdateien nicht.
Grundplatten in den Schubladen nehmen separat erzeugte Gridfinity-Behälter auf.
Eigene Wände und Werkzeug-Aussparungen gelten für Behälter bzw. Einsätze.

## Etikettenfelder und Einschubschilder

Bei Behältern und Werkzeugeinsätzen stehen unter **Beschriftung → Ausführung**
drei Möglichkeiten zur Auswahl:

- **Direkter Text:** Die vorhandene erhabene oder gravierte Schrift sitzt auf
  der Box. Ein leeres Textfeld erzeugt keine Schrift.
- **Etikettenfeld:** Eine leere Vertiefung mittig an der Vorderseite zum
  Aufkleben eines Etiketts oder Beschreiben. Breite, Höhe und Tiefe einstellen.
  Eine bereits eingegebene Beschriftung bleibt im Projekt gespeichert, erscheint
  in diesem Modus aber nicht auf der Box. Mindestens 0,8 mm Außenwand bleiben
  hinter der Vertiefung stehen. Die Tiefe ist höchstens 1,2 mm; bei einer
  1,2-mm-Wand sind bis zu 0,4 mm möglich.
- **Einschubschild:** Die Box erhält unten an der Vorderseite eine
  **Schildaufnahme** mit zwei seitlichen Führungen und einem unteren Anschlag.
  Das Schild wird separat gedruckt und von oben eingeschoben. Box mit Aufnahme
  und Schild bleiben getrennte Druckteile im selben Entwurf.
  **Nutzbreite** und **Nutzhöhe** beziehen sich auf die freie Schildfläche.
  Der Außenrand kommt mit 1 mm pro Seite dazu: Eine Nutzfläche von 40 × 12 mm
  ergibt eine Schildplatte von 42 × 14 mm bei 1,2 mm Dicke. Das Textfeld leer
  lassen, um das Schild zu bekleben oder zu beschreiben, oder einen kurzen
  Text wie KABEL erhaben bzw. graviert ergänzen. Die Schienen enden 1,5 mm
  unter der Schildoberkante, damit sie zum Herausziehen greifbar bleibt.

Die Standard-Nutzfläche ist 30 × 9 mm, das Standard-**Einschubspiel** 0,25 mm.
Das Spiel ist von 0,15 bis 0,6 mm einstellbar. Die erlaubte Breite und Höhe
richten sich nach der Box. Beim Verkleinern der Box werden die Feldmaße auf
den verfügbaren Platz begrenzt. Eine 2-U-Box bietet für ein vertieftes
Etikettenfeld höchstens 7,25 mm Höhe.

Über die Auswahl in der 3D-Ansicht ein einzelnes Druckteil ansehen oder
**Montiert anzeigen** für die Box mit eingeschobenem Schild aktivieren.
Im **Export** unter **Druckteil** das **Einschubschild** auswählen und
**Ausgewähltes Teil exportieren** verwenden. **Alle Teile separat exportieren**
schreibt Box und `Einschubschild.stl` als getrennte Dateien. Der Export legt
das Schild flach auf die Rückseite, mit der Schrift nach oben, auf Z = 0.
Die Box bleibt beim Erstellen und Exportieren des Schilds im Projekt erhalten.

Auf der Startseite gibt es auch den Modelltyp **Einschubschild** für eine
einzelne Schildplatte. Nutzfläche, Einschubspiel und Text entsprechend der
vorhandenen Aufnahme einstellen; Boxprofil, Lippenstil oder Rastermaß werden
dafür nicht benötigt. Einzelne Schildplatten erlauben 10–300 mm Nutzbreite
und 3–90 mm Nutzhöhe. Am einfachsten entsteht das passende Schild direkt im
Boxprojekt. Unter **Datei → Beispiele** stehen eine Box mit Etikettenfeld,
eine Box mit Schildaufnahme und ein leeres Schild für 40 × 12 mm Nutzfläche bereit.

Die aufgesetzte Führung liegt vollständig vor der unteren Boxfront;
Innenraum und Außenwandstärke bleiben erhalten. Eine 45°-Stützfase unter dem
Rahmen führt den Vorsprung beim Druck schrittweise aus der Boxwand heraus.
Der Rahmen steht um
1,2 mm Plattendicke + zweimal Einschubspiel + 0,8 mm Haltekante vor der Box.
Bei 0,25 mm Spiel sind das 2,5 mm: vor direkt angrenzenden Boxen entsprechend
Platz lassen. Der obere Stapelrand bleibt frei. Die Aufnahme gehört zu
dieser App; ein bereits vorhandener glatter Behälter benötigt eine neu
gedruckte Box mit Schildaufnahme. Passung und Halt sind noch nicht physisch
geprüft. Zuerst ein Schild testdrucken und das Spiel an Drucker und Material
anpassen; mehr Spiel erleichtert das Einschieben.

**Die bisherigen Schildclips mit Bügeln über dem Boxrand passen nicht in
diese Einschubaufnahme.** Alte Projektdateien mit der Clip-Ausführung werden
beim Laden auf den Einschub umgestellt; Text und übrige Modelleinstellungen
bleiben erhalten. Zu große alte Schildmaße werden auf den verfügbaren
Frontbereich begrenzt. Box und Schild für das neue System erneut exportieren.

## STL-Werkzeuge

Der Import benötigt eine geschlossene Werkzeug-STL. Die App übernimmt ihre
XY-Abmessungen und erzeugt aus der XY-Silhouette eine Aussparung mit einstellbarer
Tiefe. Werkzeug vorher passend ausrichten. Das Ergebnis ist eine senkrecht
extrudierte Kontur, keine dreidimensional angepasste Negativform.
Die Grundrissansicht zeigt für STL-Aussparungen den Begrenzungsrahmen;
die 3D-Vorschau zeigt die tatsächliche Kontur.

Die Werkzeugdatei wird über ihren Pfad referenziert und muss erhalten bleiben.
Beim Übertragen eines Projekts auf einen anderen PC die STL mitnehmen und in der
App erneut auswählen. Grenze: 40 MB und 500.000 Dreiecke.

## Offene Grundplatten und Clips

Bei **Grundplatte** ist **Offene Rasterfelder (ohne Boden)** standardmäßig aktiv.
Die Mitte jeder Rasterzelle ist durchgehend offen; Rahmen und Auflageprofil
tragen weiterhin die Behälter. Bei Bedarf lässt sich der Boden wieder aktivieren.
Unter **Befestigung** eine passende Kombination auswählen: ohne Aufnahmen,
vier äußere Magnete, Magnete je Rasterfeld, vier äußere Schrauben oder Magnete
zusammen mit vier äußeren Schrauben. Die App zeigt die tatsächliche Anzahl
der Aufnahmen an. Neue Kombinationen mit Schrauben verwenden vier äußere
Befestigungspunkte; dadurch entstehen nicht unnötig viele Schraubenaufnahmen.
Ältere Entwürfe mit anderen Einstellungen erscheinen als **Bestehende Kombination**
und bleiben unverändert bearbeitbar und exportierbar. Erst eine bewusste neue
Auswahl ersetzt ihre Befestigungseinstellungen. Magnetdurchmesser und
Taschentiefe sind einstellbar. Die Magnettaschen besitzen mindestens 1 mm Boden;
Schrauben erhalten einen Durchgang von 3,2 mm und eine 90°-Senkung von 6,4 mm.
Sind beide Optionen aktiv, sitzen die Schraubenaufnahmen an den Zellrandmitten
neben den Magneten. Bei offenen Platten verbinden schmale Arme die Aufnahmen
mit dem Raster; die Zellmitten bleiben frei.

Der nötige Unterbau erhöht die Grundplatte: Mit den Standard-Magnetmaßen
6,2 × 2,4 mm beträgt die Gesamthöhe etwa **8,05 mm**. Die Clip-Aufnahmen und
das Sitzprofil werden gemeinsam angehoben. Zum Verbinden nebeneinanderliegender
Platten dieselbe Sitzhöhe verwenden; unterschiedliche Höhen ergeben versetzte
Clip-Aufnahmen. Ein Beispiel steht unter **Beispiele → Grundplatte mit Magneten
und Schrauben** bereit. Vor dem Seriendruck die gewählten Magnete und Schrauben
an einer kleinen Probe prüfen.

Grundplatten sind nun exakt Rasteranzahl × 42 mm groß, ohne den bisherigen
zusätzlichen Außenrand. Dadurch bleibt das Raster über mehrere Platten hinweg
erhalten. Bereits exportierte Grundplatten mit zusätzlichem Rand passen nicht
bündig zu dieser neuen Variante.

**Clip-Aufnahmen** fügt Aufnahmen an allen vier Seiten hinzu, eine pro
angrenzender Rasterzelle. **Grundplatte** erzeugt und exportiert ausschließlich
`Grundplatte.stl`. Auf der Startseite oder im Modellmenü **Clips** separat
wählen; dieser Modelltyp erzeugt ausschließlich `Verbindungsclip.stl`.
Beide Modelle können unabhängig in der Projektablage gespeichert werden.
Den Clip im Slicer nach Bedarf
vervielfältigen. Platten bündig aneinanderlegen und die länglichen Clips **von
oben** über die abgesenkten Randabschnitte drücken. Die beiden Schenkel greifen
an die Haltekanten. Die Beispiele für Grundplatte und Clip sind getrennt
auswählbar. Die Clip-STL steht wie im Referenzgenerator auf ihrer
Stirnfläche (konstanter Querschnitt, 19,6 mm hoch); bei Bedarf im Slicer einen
Brim für die kleine Aufstandsfläche hinzufügen.

Clip und Randaufnahmen wurden anhand der tatsächlichen Profile von
gridfinitygenerator.com neu in Rust implementiert: 20-mm-Aufnahmen pro Zelle,
19,6-mm-Clip, 4,3-mm-Außenbreite, zwei Haken und abgerundete Oberkante.
Die offene Grundplatte ist jetzt ca. 4,65 mm hoch und besitzt gerade Innenstege.
Ein optionaler geschlossener Boden fügt 1,5 mm hinzu. Die ideal scharfe obere
Profilkante wird für einen stabilen STL-Export um 0,001 mm gekürzt; die untere
Clip-Aussparung beginnt aus demselben Grund 0,001 mm höher.

Ein zusätzlicher Test verwendet direkt von der Website exportierte STL-Dateien:
unser Clip passt rechnerisch in deren Grundplatte und deren Clip in unsere.
Diese Aussage gilt für die geprüfte 42-mm-Variante ohne Magnet-/Schraubtaschen.
Zuerst zwei 1×1-Grundplatten und einen Clip drucken; **Clip-Toleranz** bei Bedarf
anpassen. Standard 0 mm entspricht der Website; positive Werte vergrößern den
inneren Freiraum. Rastkraft und Haltbarkeit wurden noch nicht physisch geprüft.
Die alten Kreuzclips und die bisherigen 6,5-mm-Grundplatten dieser App passen
nicht zu dieser neuen Verbindung; bereits exportierte Dateien neu erzeugen.

Die Beispielprojekte sind in der EXE eingebettet und funktionieren auch, wenn
die EXE direkt auf dem Desktop liegt.

## Maße und Druck

Raster: 42 mm; Höhenstufe: 7 mm. Neue Behälter verwenden das Referenzprofil:
Außenmaß Rasteranzahl × 42 − 0,62 mm; 2×2×3 mit Standardlippe misst ca.
83,38 × 83,38 × 25,85 mm. Stapelabstand: Höhenstufen × 7 + 0,245 mm.
Standard- und dünne Lippe oder keine Lippe sind auswählbar. Alte Projektdateien
behalten das bisherige Profil (−0,5 mm) und ihre bisherigen Wand-/Bodenwerte.
Der Abgleich mit der gelieferten box-2-2-3.stl ergibt ca. 0,03 % symmetrische
Volumendifferenz. Das ist eine rechnerische Geometrieprüfung.
Stapellippe und Fußprofil folgen öffentlich dokumentierten Gridfinity-Maßen.
Magnettaschen und optionale blinde Schraublöcher sind verfügbar.
Schubladenspiel ist pro Seite einstellbar (Standard 0,4 mm).

Automatisierte Prüfungen kontrollieren geschlossene Oberflächen, Volumen,
Abmessungen, STL-Rückimport und rechnerische Passungen. Es wurden noch keine
physischen Testdrucke durchgeführt. Vor Serienfertigung oder Verkauf zunächst
Fuß/Grundplatte und eine Schubladenpassung testdrucken; Drucker, Material und
Elefantenfuß beeinflussen das Spiel. Für Behälter in Schubladen ausreichend
Höhe einschließlich Stapellippe und Einsetzspiel einplanen.

## Technik und Entwicklung

Rust mit egui/eframe, GPU-Vorschau über WGPU/DirectX 12 und Tiefenpuffer.
Die CAD-Vorschau verwendet Konturlinien, geglättete Rundungen und vierfache
Kantenglättung. Die Darstellungsproportionen bleiben auch auf Ultrawide-Monitoren
erhalten.
Manifold wird als native C++-Geometriebibliothek statisch eingebunden.
Zur Laufzeit werden kein Python, OpenSCAD, KI-Dienst oder Webserver benötigt.
Ein Windows-Grafiktreiber mit DirectX-12-Unterstützung ist erforderlich.

Zum Bauen werden Windows x64, Rust für `x86_64-pc-windows-msvc`, Git und Visual
Studio C++ Build Tools mit dem Workload **Desktopentwicklung mit C++**,
Windows SDK und CMake benötigt. Getestet mit Rust **1.99.0**. Die benötigten
Schrift-, Shader- und Beispieldateien sind im Repository enthalten.

Das Repository klonen und im Projektverzeichnis folgende Befehle ausführen.
`scripts/build.ps1` sucht CMake im Suchpfad oder in der Visual-Studio-Installation,
baut mit gesperrten Abhängigkeiten und kopiert die EXE ins Projektverzeichnis.

```powershell
.\scripts\build.ps1
cargo fmt --check
cargo test --locked --release
cargo clippy --locked --release -- -D warnings
```

Falls CMake nur in Visual Studio installiert ist, dessen `bin`-Verzeichnis für
direkte Cargo-Aufrufe ebenfalls in `PATH` aufnehmen. Die Build-Skript-Erkennung
gilt innerhalb des Skripts.

Für eine portable Ausgabe einschließlich der Visual-C++-Laufzeitdateien:

```powershell
.\scripts\package.ps1 -OutputPath "$PWD\GridfinityStudio-v0.1.1-Windows-x64.zip"
```

Ein bereits vorhandenes ZIP wird nicht überschrieben. Bei einem eigenen
Build-Verzeichnis `-BuildExe` mit dem Pfad der gebauten EXE angeben.
Das Build-Skript verwendet ausdrücklich `x86_64-pc-windows-msvc`; das Cargo-Artefakt
liegt unter `target/x86_64-pc-windows-msvc/release/`. Ein über
`CARGO_TARGET_DIR` gesetztes Verzeichnis wird auch beim Verpacken berücksichtigt.
Falls die automatische Runtime-Suche nicht passt, mit `-CrtDir` den Ordner
`Microsoft.VC*.CRT` der x64-Laufzeit angeben.
Persönliche Projekte liegen außerhalb des Repositorys in der lokalen
Anwendungsablage und werden weder eingecheckt noch in die Programm-ZIP gepackt.

Der erste Build lädt Rust-Abhängigkeiten und festgelegte Manifold/Clipper2-Quellen.
Details zur Quellcodeprüfung stehen in `SECURITY-REVIEW.md`.

Der Referenzvergleich kann zusätzlich mit
`cargo test --locked --release reference_generator_interoperability -- --ignored --nocapture`
ausgeführt werden. Er benötigt die lokal abgelegten Original-STL-Exporte in
`reviews/reference-site/`; diese Prüfdaten werden nicht mit der App verteilt.

Batch-Export und direkter Projektstart:

```powershell
.\GridfinityStudio.exe --export examples\tools.gridfinity.json --output output\tools
.\GridfinityStudio.exe --project examples\deskbox.gridfinity.json
```

## Quellen

- Referenz für gewünschte Bedienung: https://gridfinitygenerator.com/de
- Gridfinity-Spezifikation: https://gridfinity.xyz/specification/
- Öffentliche Maßkonstanten: https://github.com/kennetek/gridfinity-rebuilt-openscad/blob/main/src/core/standard.scad
- Geometriebibliothek: https://github.com/elalish/manifold

Die Geometrie wurde eigenständig implementiert; OpenSCAD-Code wird nicht ausgeführt
oder eingebettet. Lizenzhinweise stehen in `THIRD-PARTY-NOTICES.md`.

## Drucköffnung mit gedruckter Feder (Prototyp)

Unter **Schubladenbox → Drucköffnung mit gedruckter Feder** aktivieren oder
**Datei → Beispiele → Drucköffnung – Federprototyp** öffnen. Die Option ist
standardmäßig aus. Bestehende Schubladen bleiben unverändert.

Dieser experimentelle Entwurf verwendet ausschließlich gedruckte Teile:
Je Schublade zwei austauschbare Mäanderfedern und einen biegsamen Rastfinger.
Eine herzförmige Führungsbahn mit Rampen und Absatzstufen sitzt hinten an der
Schublade. Das Gehäuse wird 70 mm tiefer; mindestens zwei Rasterfelder Breite
und 3 U Schubladenhöhe sind erforderlich. Die Federstärke ist zwischen 0,8 und
1,2 mm einstellbar. Zum ersten Test nur eine Schublade und kein oberes Raster verwenden.

**Noch kein funktionell validierter Verschluss:** Geprüft wurden geschlossene
STL-Oberflächen, Einbauraum und geometrische Kollisionsfreiheit in ausgewählten
Stellungen. Federkräfte, zulässige Verformung, gerichteter Lauf durch die
Rastbahn, Wiederholgenauigkeit und Lebensdauer sind nicht durch Druckversuche
oder eine Festigkeitsrechnung bestätigt. Die Animation zeigt die Feder nur
schematisch. Das Modell ist ein Ausgangspunkt für Testdruck und Abstimmung.

### Druck und Montage

- Alle Druckteile einzeln exportieren: Gehäuse, Schublade, Feder links/rechts
  und Rastfinger. Bei einer Schublade entstehen fünf STL-Dateien. Die Federn
  werden unverformt exportiert; die montierte Vorschau verschmilzt keine Teile.
- Der Export legt Federn flach und den Rastfinger mit der ebenen Rückseite aufs
  Druckbett, sodass sein Stift nach oben steht. Das Gehäuse liegt auf seiner
  Rückwand. Die Schublade bleibt auf ihrem Boden; ihre hinten überstehende
  Führungsbahn benötigt gegebenenfalls Unterstützung **vom Druckbett**.
  Slicer-Vorschau prüfen; keine Stützen in die Führungsbahn setzen.
- Schmale Federbahnen vollständig als durchgängige Extrusionsbahnen drucken.
  Material und Druckparameter zunächst an einem einzelnen Federteil erproben.
  Es gibt noch keine freigegebene Material-/Parameterkombination.
- Je eine hintere Federlasche in die seitlichen Taschen hinten im Gehäuse
  einsetzen. Die vorderen Laschen liegen an der Rückwand der Schublade an.
  Die breite Lasche des Rastfingers von vorn in die mittlere Aufnahme schieben;
  der Stift zeigt nach unten. Schublade langsam einschieben und den Stift in
  den offenen Eingang der Führungsbahn führen. Nicht mit Gewalt einrasten.
- Vorgesehen sind etwa 6 mm Druckweg zum Entriegeln und etwa 10 mm Ausstoß;
  danach die Schublade von Hand weiter herausziehen. Ein Anschlag begrenzt den
  Weg nach innen. Diese Werte sind Konstruktionsziele, keine gemessene Leistung.

Zuerst unbeladen prüfen, ob der Stift beim wiederholten Drücken und Loslassen
abwechselnd die Raststelle und den Rückweg erreicht. Bei Klemmen oder bleibender
Verformung abbrechen und Geometrie/Passung anpassen. Danach mit geplanter Last
und nach längerer geschlossener Lagerung erneut prüfen. Kunststofffedern können
unter Dauerlast nachgeben; höhere Federstärke allein löst das nicht automatisch.
Vor Verkauf sind eigene Funktions- und Haltbarkeitstests erforderlich.

Grundlagen für die eigenständige Konstruktion (kein fremder Programmcode oder
STL übernommen):
- Herzkurve mit Stufen und federndem Abtaster:
  https://patents.google.com/patent/DE2926824A1/en
- Druck-Druck-Verschluss mit Kurvenführung:
  https://patents.google.com/patent/US4916276A/en
- Kriechen und Spannungsrelaxation nachgiebiger Mechanismen:
  https://scholarsmine.mst.edu/mec_aereng_facwork/4090/

### Druckbett in der Vorschau

Nach der Druckerauswahl oder dem Übernehmen eigener Maße öffnet sich die
**Druckbett**-Ansicht. Sie zeigt das aktive Druckteil in seiner STL-Drucklage,
mittig auf dem maßstabsgetreuen Bett. Der eingestellte Rand ist farblich
abgesetzt. X/Y-Bettmaße und maximale Druckhöhe stehen direkt in der Vorschau.
Zu große Teile erscheinen rot; auch eine Überschreitung der Druckhöhe wird
angezeigt. Falls eine 90°-Drehung auf dem Bett genügt, zeigt die Vorschau diese
Drehung mit einem Hinweis – sie muss anschließend im Slicer übernommen werden.

Bei mehreren Druckteilen lässt sich oben jedes Teil einzeln auswählen. Unter
**Modell** bleibt die bisherige Ansicht mit montierten Teilen verfügbar.
**Druckbett anzeigen** blendet die Bettfläche ein oder aus. Die Bettfläche wird
nicht in STL-Dateien exportiert. Es werden keine Druckteile automatisch
zusammen auf einer Druckplatte angeordnet.
