# Entwicklung

- Desktop-Oberfläche und Anwendungslogik in Rust belassen; kein Python-Backend,
  Electron oder zusätzlicher Webserver als Laufzeitvoraussetzung.
- Deutsche Oberfläche und deutsche Dokumentation mit echten Umlauten und ß.
  Die README ist eine kurze englische Projektübersicht; ausführliche deutsche
  Bedienhinweise stehen in docs/USER_GUIDE.de.md.
- Zusätzliche Bibliotheken vor Integration anhand des tatsächlichen Quellcodes,
  ihrer Abhängigkeiten und Build-Skripte prüfen; Version und Umfang dokumentieren.
- Der öffentliche Produktname lautet GridfinityStudio, ohne Design-Zusatz.
  EXE und entpackter Ordner heißen GridfinityStudio; ZIP und Fenstertitel
  dürfen die Versionsnummer enthalten. Die bestehende interne Datenablage
  GridfinityStudioDesign2 bleibt für vorhandene Projekte kompatibel.
- Persönliche Entwürfe, lokale Einstellungen, Prüfdaten und Build-Ausgaben
  nicht einchecken. Portable Programme als Release-ZIP bereitstellen.
- Geometrieänderungen mit maßhaltigen Regressionstests sowie geschlossenen
  STL-Oberflächen prüfen. Eine rechnerische Passung ersetzt keinen Testdruck.
- Vor Auslieferung Formatierung, passende Rust-Tests, Clippy und Release-Build
  prüfen. Hinweise zum Windows-Build stehen in README.md.
