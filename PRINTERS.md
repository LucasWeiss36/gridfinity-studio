# Druckerprofile

Stand der Quellenprüfung: **13. Oktober 2026**. Die Auswahl enthält 34 geprüfte Modell-/Betriebsartprofile von fünf Herstellern. Sie ist kein vollständiger Druckerkatalog. Die Anwendung speichert diese Daten lokal in `src/printer_profiles.rs`; für die Profilauswahl ist keine Internetverbindung nötig.

Alle Maße sind **X × Y × Z in Millimetern**: Breite, Tiefe und Höhe des Druckbereichs. Es handelt sich nicht um die Außenmaße des Druckers oder die Abmessungen der herausnehmbaren Druckplatte. Die Tabelle nennt genau die im Generator hinterlegten Werte.

## Bambu Lab

| Modell / Betriebsart | Hinterlegter Bauraum | Primärquelle |
| --- | --- | --- |
| A1 mini | 180 × 180 × 180 | [Herstellerhandbuch](https://cdn1.bambulab.com/documentation/quick-start-f507128172bdf/Quick%20start%20guide%20-%20A1%20mini-EN.pdf) |
| A1 | 256 × 256 × 256 | [Herstellerhandbuch](https://cdn1.bambulab.com/documentation/quick-start-a75adcb1d5d5e/Quick%20Start%20Guide%20for%20A1.pdf) |
| A2L | 330 × 320 × 325 | [Herstellervorstellung](https://blog.bambulab.com/bambu-lab-a2l-creative-playground-extra-large/) |
| P1P | 256 × 256 × 250 | [Herstellervergleich P1S/P1P und Höhenhinweis](https://us.store.bambulab.com/products/p1s?skr=yes) |
| P1S | 256 × 256 × 250 | [Herstellerdaten und Höhenhinweis](https://us.store.bambulab.com/products/p1s?skr=yes) |
| P2S | 256 × 256 × 256 | [Herstellervorstellung](https://blog.bambulab.com/the-icon-redefined-meet-the-p2s-a-completely-reengineered-version-of-the-ultra-productive-p1-series/) |
| X1 | 256 × 256 × 250 | [Offizielles BambuStudio-Profil](https://raw.githubusercontent.com/bambulab/BambuStudio/master/resources/profiles/BBL/machine/Bambu%20Lab%20X1%200.4%20nozzle.json), siehe Profilvererbung unten |
| X1 Carbon | 256 × 256 × 250 | [Herstellerdaten und Höhenhinweis](https://us.store.bambulab.com/products/x1-carbon?variant=42698346037384) |
| X1E | 256 × 256 × 250 | [Offizielles BambuStudio-Profil](https://raw.githubusercontent.com/bambulab/BambuStudio/master/resources/profiles/BBL/machine/Bambu%20Lab%20X1E%200.4%20nozzle.json), siehe Profilvererbung unten |
| H2S | 340 × 320 × 340 | [Herstellervorstellung](https://blog.bambulab.com/h2s-the-ultimate-single-nozzle-3d-printer-now-bigger-than-ever/) |
| H2D · Einzeldüse | 325 × 320 × 325 | [Herstellerhandbuch](https://cdn1.bambulab.com/documentation/h2d/en/H2D_Laser_Full_Combo_20250305.pdf) |
| H2D · Dual | 300 × 320 × 325 | [Herstellerhandbuch](https://cdn1.bambulab.com/documentation/h2d/en/H2D_Laser_Full_Combo_20250305.pdf) |
| H2D Pro · Einzeldüse | 325 × 320 × 325 | [Technische Herstellerdaten](https://bambulab.com/ko-kr/h2d-pro/tech-specs) |
| H2D Pro · Dual | 300 × 320 × 325 | [Technische Herstellerdaten](https://bambulab.com/ko-kr/h2d-pro/tech-specs) |
| H2C · Einzeldüse | 305 × 320 × 325 | [Herstellervorstellung](https://blog.bambulab.com/bambu-lab-h2c-where-multi-material-vortek-system-meets-engineering-precision/) |
| H2C · Dual | 300 × 320 × 325 | [Herstellervorstellung](https://blog.bambulab.com/bambu-lab-h2c-where-multi-material-vortek-system-meets-engineering-precision/) |
| X2D · Hauptdüse | 256 × 256 × 260 | [Herstellererklärung zu den Düsenbereichen](https://blog.bambulab.com/two-extruders-one-purpose-what-is-x2d-direct-drive-extrusion-and-auxiliary-extrusion/) |
| X2D · Dual / Hilfsdüse | 235,5 × 256 × 256 | [Herstellererklärung zu den Düsenbereichen](https://blog.bambulab.com/two-extruders-one-purpose-what-is-x2d-direct-drive-extrusion-and-auxiliary-extrusion/) |

**Höhe bei P1 und X1:** Die Hardware-Nennangabe lautet 256 × 256 × 256 mm. Der Generator verwendet für P1P, P1S, X1, X1 Carbon und X1E die übliche Slicerhöhe von 250 mm. Zusätzlich können Cutter-Sperrflächen die nutzbare XY-Fläche begrenzen. Das vergrößerte Hardware-Nennmaß darf nicht als uneingeschränkt nutzbarer Standardbereich verstanden werden. Einen bereits fachgerecht angepassten Drucker-/Slicerbereich kann man über „Benutzerdefiniert“ abbilden. [P1-Daten mit Fußnote](https://us.store.bambulab.com/products/p1s?skr=yes), [X1-Carbon-Daten mit Fußnote](https://us.store.bambulab.com/products/x1-carbon?variant=42698346037384).

Für X1E wurde die Profilvererbung im offiziellen BambuStudio-Repository geprüft: Das [X1E-Profil](https://raw.githubusercontent.com/bambulab/BambuStudio/master/resources/profiles/BBL/machine/Bambu%20Lab%20X1E%200.4%20nozzle.json) erbt [fdm_bbl_3dp_001_common](https://raw.githubusercontent.com/bambulab/BambuStudio/master/resources/profiles/BBL/machine/fdm_bbl_3dp_001_common.json) mit 256 × 256 mm Druckfläche; dessen Basis [fdm_machine_common](https://raw.githubusercontent.com/bambulab/BambuStudio/master/resources/profiles/BBL/machine/fdm_machine_common.json) setzt `printable_height` auf 250 mm. X1E überschreibt diese Höhe nicht. Das [X1E-Datenblatt](https://cdn1.bambulab.com/x1e/spec/X1E%20Spec%28EN%29.pdf) nennt dagegen das Hardware-Nennmaß 256³. Die JSON-Dateien wurden als Quellen gelesen; es wird kein Hersteller-G-Code übernommen oder ausgeführt.

Die ebenfalls geprüften Profile für [X1](https://raw.githubusercontent.com/bambulab/BambuStudio/master/resources/profiles/BBL/machine/Bambu%20Lab%20X1%200.4%20nozzle.json) und [P1P](https://raw.githubusercontent.com/bambulab/BambuStudio/master/resources/profiles/BBL/machine/Bambu%20Lab%20P1P%200.4%20nozzle.json) verwenden dieselbe Bauraumvererbung ohne Höhenüberschreibung. Das [X1-/X1C-Hardwarehandbuch](https://cdn1.bambulab.com/documentation/quick-start-2024-01-17/Quick-Start-X1%26X1C_en.pdf) bestätigt separat das Nennmaß 256³.

**Druck mit zwei Düsen:** „Dual“ verwendet den gemeinsamen rechteckigen Bereich beider Düsen. Die über beide Düsen erreichbare Vereinigungsfläche ist kein einheitlich nutzbarer Dual-Bauraum und wird nicht als Profil angeboten. Beim X2D ist der Bereich mit Hilfsdüse auch niedriger. Für H2C ist der hier belegte Einzeldüsenbereich 305 mm breit; eine weitere, nicht eindeutig belegte Links-/Rechts-Variante wird nicht angenommen. Die Auswahl des Profils verändert weder das STL noch die Düsenkonfiguration des Slicers.

## Prusa

| Modell | Hinterlegter Bauraum | Primärquelle |
| --- | --- | --- |
| MK4S | 250 × 210 × 220 | [Herstellerproduktseite](https://www.prusa3d.com/product/original-prusa-mk4s-3d-printer/) |
| CORE One | 250 × 220 × 270 | [Hersteller-FAQ](https://help.prusa3d.com/article/faq-frequently-asked-questions_1932?product=xl) |
| CORE One+ | 250 × 220 × 270 | [Herstellerproduktseite](https://www.prusa3d.com/product/prusa-core-one/) |
| MINI+ | 180 × 180 × 180 | [Herstellerproduktseite](https://www.prusa3d.com/en/product/original-prusa-mini-kit-6/) |
| XL | 360 × 360 × 360 | [Hersteller-FAQ](https://help.prusa3d.com/article/faq-frequently-asked-questions_1932?product=xl) |

CORE One und CORE One+ beziehen sich auf das jeweilige Standardgerät ohne INDX-Umbau. Für MINI+ wurde die eindeutige 180³-Angabe der Produktseite verwendet; eine abweichende Zeile in der allgemeinen FAQ wurde nicht übernommen.

## Creality

| Modell | Hinterlegter Bauraum | Primärquelle |
| --- | --- | --- |
| Ender-3 V3 | 220 × 220 × 250 | [Herstellervergleich der V3-Modelle](https://www.creality.com/compare/compare-ender-3-v3-series) |
| K1C | 220 × 220 × 250 | [Herstellerproduktseite](https://www.creality.com/products/k1c-carbon-3d-printer) |
| K1 Max | 300 × 300 × 300 | [Herstellerproduktseite](https://www.creality.com/products/creality-k1-max-3d-printer) |

Ender-3 V3 bezeichnet genau dieses Modell, nicht V3 KE, SE oder Plus. Deren Profile dürfen nicht aus dem Namen abgeleitet werden.

## Elegoo

| Modell | Hinterlegter Bauraum | Primärquelle |
| --- | --- | --- |
| Neptune 4 | 225 × 225 × 265 | [Herstellerproduktseite](https://www.elegoo.com/products/elegoo-neptune-4-fdm-3d-printer) |
| Neptune 4 Pro | 225 × 225 × 265 | [Herstellerproduktseite](https://www.elegoo.com/products/elegoo-neptune-4-pro-fdm-3d-printer) |
| Neptune 4 Plus | 320 × 320 × 385 | [Herstellerproduktseite](https://www.elegoo.com/products/neptune-4-plus-fdm-3d-printer) |
| Neptune 4 Max | 420 × 420 × 480 | [Herstellerproduktseite](https://us.elegoo.com/products/neptune-4-max-fdm-3d-printer) |
| Centauri Carbon | 256 × 256 × 256 | [Herstellerproduktseite](https://www.elegoo.com/pages/elegoo-centauri-carbon) |

Die teilweise größeren PEI-Druckplattenmaße der Neptune-Reihe wurden nicht als Bauraum übernommen.

## Anycubic

| Modell | Hinterlegter Bauraum | Primärquelle |
| --- | --- | --- |
| Kobra 3 | 250 × 250 × 260 | [Herstellerproduktseite](https://store.anycubic.com/products/anycubic-kobra-3-1) |
| Kobra 3 V2 | 255 × 255 × 260 | [Herstellerhandbuch V1.0](https://wiki.anycubic.com/k3-v2/Anycubic%20Kobra%203%20V2%20combo%20User%20Manual-EN-V1.0.pdf) |
| Kobra S1 | 250 × 250 × 250 | [Herstellerhandbuch V1.3](https://wiki.anycubic.com/kobra-s1/anycubic_kobra_s1_combo_user_manual-en-v1.3.pdf) |

Die Kobra-3-V2-Angabe ist der Nennbauraum des V2-Handbuchs. Firmware und installiertes Slicerprofil können den nutzbaren Bereich begrenzen oder ändern. Der ursprüngliche Kobra 3 bleibt ein eigenes Profil; spätere individuelle Erweiterungen gehören in „Benutzerdefiniert“.

## Grenzen der Prüfung

Die Profile beschreiben jeweils einen rechteckigen Bauraum. Slicerabhängige Sperrflächen, Reinigungslinien, Halteklammern, Spültürme, Brim und Stützen werden durch diese drei Zahlen nicht vollständig beschrieben. Randabstand lässt sich zusätzlich einplanen, ersetzt aber keine positionsabhängige Prüfung im Slicer. Besonders nahe an der maximalen Größe muss das konkrete Teil mit dem passenden Drucker- und Düsenprofil im Slicer geprüft werden.

„Benutzerdefiniert“ ist für andere Drucker und bewusst angepasste Bereiche vorgesehen. Die Anwendung erkennt Drucker, Firmware und Slicereinstellungen nicht automatisch. Die gespeicherten Herstellerangaben werden nicht im Hintergrund aktualisiert. Die Maßprüfung bestätigt nur den Platzbedarf; Materialeignung, Druckbarkeit, mechanische Passung und Kollisionsfreiheit aller Druckbewegungen sind davon getrennte Fragen.

Die Datensatztests prüfen eindeutige IDs und Namen, endliche positive Maße, hinterlegte Herstellerquellen sowie die Unterschiede zwischen Einzeldüsen- und Dual-Profilen. Sie ersetzen weder eine erneute Quellenprüfung nach Profiländerungen noch einen tatsächlichen Testdruck.
