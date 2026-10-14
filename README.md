# Gridfinity Studio

A native Rust desktop app for designing Gridfinity storage and exporting printable STL files. Models are generated locally, with an interactive 3D preview and light/dark themes.

**Windows x64 · German interface · MIT license**

[Download the app](https://github.com/LucasWeiss36/gridfinity-studio/releases) · [User guide (German)](docs/USER_GUIDE.de.md)

## Features

- **Bins:** adjustable size, stacking rims, magnet pockets and screw holes.
- **Dividers:** equal compartments or custom walls with adjustable height and thickness, snapping, axis locking and clickable dimensions.
- **Baseplates and clips:** open or solid baseplates, optional magnet/screw mounts and separately printed connection clips.
- **Tool inserts:** circular, rectangular, hexagonal and imported STL-outline cutouts.
- **Labels:** raised or engraved text, recessed sticker areas and separate slide-in nameplates.
- **Drawer boxes:** individual drawers and an optional experimental printed push-to-open mechanism.
- **Printer sizing:** built-in printer profiles or custom build-volume dimensions, with a print-bed preview and per-part fit checks.
- **Projects:** folders, saved designs and individual or batch STL export.

## Download and run

1. Download the **Windows-x64 ZIP** from [Releases](https://github.com/LucasWeiss36/gridfinity-studio/releases).
2. Extract the entire archive.
3. Run `GridfinityStudioDesign2.exe` or `Start.cmd`.

Requires **Windows 10/11 x64** and a **DirectX 12-capable graphics driver**. The ZIP includes the required Visual C++ runtime files. Rust is only needed to build from source.

Choose a model, edit its properties, select your printer or enter custom dimensions, then export the desired parts. Example designs are available under **Datei → Beispiele**.

GitHub's **Code → Download ZIP** contains source code, not the ready-to-run app. This repository contains the standalone **Design 2** version.

## Printing notes

- STL coordinates use **millimeters**. The default grid pitch is **42 mm**, with **7 mm** height units.
- Clips, nameplates, drawers and mechanism components export as separate parts; the assembled preview does not fuse them.
- Printer checks cover each part's dimensions, not automatic plate packing, printer exclusion zones or slicer settings. Any suggested 90° rotation must also be applied in your slicer.
- Imported tool STLs produce an extruded **XY silhouette**, not a fully conforming 3D cavity.
- Fits are checked computationally; physical test prints are still needed. The printed push-to-open mechanism is an **unvalidated prototype**.

See the [user guide](docs/USER_GUIDE.de.md) for tolerances, assembly and detailed controls, and [printer profiles](PRINTERS.md) for dimensions and sources. Both documents are currently in German.

## Build from source

Install **Git**, **Rust for `x86_64-pc-windows-msvc`**, and **Visual Studio C++ Build Tools** with the *Desktop development with C++* workload, Windows SDK and CMake. Tested with Rust **1.99.0**.

```powershell
git clone https://github.com/LucasWeiss36/gridfinity-studio.git
cd gridfinity-studio
.\scripts\build.ps1
```

The script finds CMake and copies the resulting EXE into the project directory. The first build downloads locked Rust dependencies and pinned native Manifold/Clipper2 sources. Manifold is statically linked through Rust bindings.

For validation, make CMake available in `PATH` before running Cargo directly:

```powershell
cargo fmt --check
cargo test --locked --release
cargo clippy --locked --release -- -D warnings
```

One optional comparison test requires external reference STLs and is ignored by default.

Create a portable package:

```powershell
.\scripts\package.ps1 -OutputPath "$PWD\GridfinityStudio-Design2-Windows-x64.zip"
```

Packaging respects `CARGO_TARGET_DIR`. Use `-BuildExe` for a specific EXE or `-CrtDir` for a custom x64 MSVC runtime directory. Existing ZIP files are not overwritten.

## Project files and license

Designs and settings are stored under `%LOCALAPPDATA%\GridfinityStudioDesign2`. Back up the `Projekte` folder to transfer saved designs. Imported tool STLs must be copied separately and reselected if their paths change. Release ZIPs do not include personal projects.

Licensed under [MIT](LICENSE). See [third-party notices](THIRD-PARTY-NOTICES.md) and the [dependency source review](SECURITY-REVIEW.md).
