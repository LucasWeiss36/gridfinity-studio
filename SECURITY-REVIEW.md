# Prüfung eingebundener Geometriebibliotheken

Stand: 13. Oktober 2026. Vor Integration wurden tatsächlicher Quellcode und
Build-Skripte der zusätzlichen Geometriebibliotheken geprüft.

| Komponente | Festgelegter Stand | Herkunft |
|---|---|---|
| manifold-csg | 0.4.1 | crates.io, veröffentlichter Rust-Quellcode |
| manifold-csg-sys | 3.5.105 | crates.io, FFI und Build-Skripte |
| Manifold | v3.5.3 | https://github.com/elalish/manifold |
| Clipper2 | 46f639177fe418f9689e8ddb74f08a870c71f5b4 | https://github.com/AngusJohnson/Clipper2 |

SHA256 der heruntergeladenen Crate-Archive:

```text
manifold-csg-0.4.1:
5ba4dd58c980405d5a23657a59106e2844a36d3c25b745ee7a0a3e0524e6bc4d
manifold-csg-sys-3.5.105:
58f034b8af43be927e7651da0c18a0205e8942acf275550acc4a8300042dee51
```

## Umfang und Ergebnis

Geprüft wurden Rust-Manifeste, Bindings, FFI-Aufbau und Mesh-Übertragung,
relevante vollständige Build-, Download- und Patch-Skripte, Manifold-CMake und
C-API sowie die festgelegten Clipper2-CMake-Dateien und relevante Quellen.
In nativen Laufzeitquellen wurde zusätzlich nach Shell- und Netzwerkaufrufen
gesucht. In den geprüften Pfaden wurden keine verdächtigen Aktionen gefunden.

Der Build verwendet Git/CMake, um die oben festgelegten nativen Quellen zu laden.
Optionale TBB-Parallelisierung, Python- und JavaScript-Bindings sowie native
Beispiel-/Test-Builds sind deaktiviert. Der App-Betrieb benötigt keine Netzwerkverbindung.
`Cargo.lock` hält die aufgelösten Rust-Versionen fest; Builds verwenden `--locked`.
Lokale Prüfauszüge liegen unter `reviews/` und werden nicht in Git aufgenommen.

Das ist eine dokumentierte Quellcodeprüfung der zusätzlichen Geometriekette,
kein vollständiges Sicherheits-Audit sämtlicher transitiver UI-, Treiber- und
Compilerkomponenten und keine Garantie für Schadcodefreiheit. Bei Updates sind
neue Quellen und Build-Skripte erneut zu prüfen.

## Abgleich der Clip-Konstruktion

Für die neue Verbindung wurden die öffentlich ausgelieferten CAD-Profile und
die tatsächlich exportierten STL-Dateien von gridfinitygenerator.com geprüft
(42-mm-Raster, offene 2×2-Grundplatte, Connectable aktiv, Clip-Toleranz 0).
Die Konstruktion wird eigenständig durch Rust-Querschnitte erzeugt. JavaScript,
Webworker, CAD-WASM und Netzwerkzugriff der Website sind keine App-Abhängigkeiten.
Die gelesenen Webdateien und STL-Vergleichsdaten liegen ausschließlich in den
lokalen Prüfauszügen unter `reviews/reference-site/`.

## Schrift und Boxreferenz
InterVariable.ttf wurde aus dem offiziellen rsms/inter-Repository (docs/font-files) bezogen. Die SIL-OFL-Lizenz ist beigefügt; die Schrift enthält keine App- oder Build-Skripte. Boxprofile wurden anhand der bereitgestellten STL und öffentlich ausgelieferter Maßprofile eigenständig in Rust konstruiert. Die Referenzdateien werden nicht zur Laufzeit ausgeführt.
