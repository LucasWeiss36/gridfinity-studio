//! Local project folders. Imported tool STLs remain external file references.
use crate::design::Design;
use anyhow::{Context, Result, bail, ensure};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Component, Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

const SUFFIX: &str = ".gridfinity.json";
const MAX_BYTES: u64 = 8 * 1024 * 1024;
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

pub struct Library {
    pub root: PathBuf,
}

#[derive(Clone, Debug)]
pub struct Entry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
}

pub fn default_root() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::var_os("USERPROFILE")
                .map(PathBuf::from)
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
                .join("AppData")
                .join("Local")
        })
        .join("GridfinityStudioDesign2")
        .join("Projekte")
}

impl Library {
    pub fn new(root: PathBuf) -> Result<Self> {
        let root = absolute(&root)?;
        reject_links(&root)?;
        fs::create_dir_all(&root).context("Der Projektordner konnte nicht angelegt werden.")?;
        reject_links(&root)?;
        let root = root
            .canonicalize()
            .context("Projektordner nicht erreichbar.")?;
        ensure!(root.is_dir(), "Die Projektbibliothek ist kein Ordner.");
        Ok(Self { root })
    }

    pub fn entries(&self, dir: &Path) -> Result<Vec<Entry>> {
        let dir = self.folder(dir)?;
        let mut entries = Vec::new();
        for entry in fs::read_dir(&dir).context("Der Projektordner konnte nicht gelesen werden.")? {
            let entry = entry.context("Ein Eintrag konnte nicht gelesen werden.")?;
            let metadata = fs::symlink_metadata(entry.path())?;
            if is_link(&metadata) {
                continue;
            }
            let name = entry.file_name();
            let Some(name) = name.to_str() else { continue };
            let is_dir = metadata.is_dir();
            let display_name = if is_dir {
                name.to_string()
            } else if metadata.is_file() {
                let Some(name) = design_name(name) else {
                    continue;
                };
                name.to_string()
            } else {
                continue;
            };
            entries.push(Entry {
                name: display_name,
                path: self.contained(&entry.path())?,
                is_dir,
            });
        }
        entries.sort_by(|a, b| {
            b.is_dir
                .cmp(&a.is_dir)
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
                .then_with(|| a.name.cmp(&b.name))
        });
        Ok(entries)
    }

    pub fn create_folder(&self, parent: &Path, name: &str) -> Result<PathBuf> {
        validate_name(name)?;
        let path = self.folder(parent)?.join(name);
        ensure_absent(&path)?;
        fs::create_dir(&path).context("Der Ordner konnte nicht angelegt werden.")?;
        self.folder(&path)
    }

    pub fn save_design(
        &self,
        folder: &Path,
        name: &str,
        cfg: &Design,
        overwrite: bool,
    ) -> Result<PathBuf> {
        validate_name(name)?;
        let folder = self.folder(folder)?;
        let path = folder.join(format!("{name}{SUFFIX}"));
        write_design(&path, cfg, overwrite)?;
        self.contained(&path)
    }

    pub fn load_design(&self, path: &Path) -> Result<Design> {
        let path = self.design_file(path)?;
        let file = File::open(&path).context("Das Modell konnte nicht geöffnet werden.")?;
        ensure!(
            file.metadata()?.len() <= MAX_BYTES,
            "Die Projektdatei ist größer als 8 MB."
        );
        let mut bytes = Vec::new();
        file.take(MAX_BYTES + 1)
            .read_to_end(&mut bytes)
            .context("Die Projektdatei konnte nicht gelesen werden.")?;
        ensure!(
            bytes.len() as u64 <= MAX_BYTES,
            "Die Projektdatei ist größer als 8 MB."
        );
        Design::from_json(&bytes)
            .context("Die Projektdatei enthält ungültige oder beschädigte Modelldaten.")
    }

    pub fn rename(&self, path: &Path, name: &str) -> Result<PathBuf> {
        validate_name(name)?;
        let source = self.contained(path)?;
        ensure!(
            source != self.root,
            "Der Hauptordner kann nicht umbenannt werden."
        );
        let metadata = fs::metadata(&source)?;
        let new_name = if metadata.is_dir() {
            name.to_string()
        } else {
            self.design_file(&source)?;
            format!("{name}{SUFFIX}")
        };
        if source
            .file_name()
            .is_some_and(|old| old == new_name.as_str())
        {
            return Ok(source);
        }
        let parent = source.parent().context("Der übergeordnete Ordner fehlt.")?;
        let target = parent.join(new_name);
        #[cfg(windows)]
        if fs::symlink_metadata(&target).is_ok() && self.contained(&target)? == source {
            // Windows resolves a spelling-only change to the existing entry.
            // Permit changing its case, while all different destinations below
            // still use the no-overwrite move.
            fs::rename(&source, &target)
                .context("Die Schreibweise des Namens konnte nicht geändert werden.")?;
            return self.contained(&target);
        }
        ensure_absent(&target)?;
        rename_no_replace(&source, &target)
            .context("Der Eintrag konnte nicht umbenannt werden.")?;
        self.contained(&target)
    }

    /// Permanently removes the entry after the UI has confirmed its name and kind.
    /// Check the complete tree before removing anything; do not follow links or
    /// recurse during removal, since new unreviewed children must not be deleted.
    pub fn delete(&self, entry: &Entry) -> Result<()> {
        let source = self.contained(&entry.path)?;
        ensure!(
            source != self.root,
            "Der Hauptordner der Projektbibliothek kann nicht gelöscht werden."
        );
        let metadata = fs::symlink_metadata(&source)?;
        ensure!(
            metadata.is_dir() == entry.is_dir,
            "Der Eintrag wurde inzwischen verändert. Bitte aktualisieren und erneut auswählen."
        );
        if !entry.is_dir {
            self.design_file(&source)?;
        }

        let mut pending = vec![source.clone()];
        let mut plan = Vec::new();
        while let Some(path) = pending.pop() {
            let path = self.contained(&path)?;
            ensure!(
                path.starts_with(&source) && path != self.root,
                "Ein Untereintrag liegt außerhalb des ausgewählten Ordners."
            );
            let metadata = fs::symlink_metadata(&path)?;
            ensure!(
                !is_link(&metadata) && (metadata.is_dir() || metadata.is_file()),
                "Der Eintrag enthält Verknüpfungen oder nicht unterstützte Dateien. Es wurde nichts gelöscht."
            );
            if metadata.is_dir() {
                for child in fs::read_dir(&path).context(
                    "Der Ordnerinhalt konnte vor dem Löschen nicht vollständig geprüft werden.",
                )? {
                    pending.push(child?.path());
                }
            }
            plan.push((path, metadata.is_dir()));
        }

        // Parents precede descendants in the plan, so reversing it removes only
        // empty directories. Recheck each final absolute target before mutation.
        for (path, is_dir) in plan.into_iter().rev() {
            let checked = self.contained(&path)?;
            ensure!(
                checked == path && checked.starts_with(&source) && checked != self.root,
                "Der Pfad wurde während des Löschens verändert. Der Vorgang wurde abgebrochen."
            );
            let metadata = fs::symlink_metadata(&checked)?;
            ensure!(
                !is_link(&metadata)
                    && metadata.is_dir() == is_dir
                    && (metadata.is_dir() || metadata.is_file()),
                "Der Eintrag wurde während des Löschens verändert. Der Vorgang wurde abgebrochen."
            );
            let result = if is_dir {
                fs::remove_dir(&checked)
            } else {
                fs::remove_file(&checked)
            };
            result.with_context(|| {
                format!(
                    "„{}“ konnte nicht gelöscht werden. Zuvor entfernte Unterinhalte bleiben gelöscht.",
                    checked.file_name().unwrap_or_default().to_string_lossy()
                )
            })?;
        }
        Ok(())
    }

    fn contained(&self, path: &Path) -> Result<PathBuf> {
        reject_links(&self.root)?;
        let root = self
            .root
            .canonicalize()
            .context("Die Projektbibliothek ist nicht erreichbar.")?;
        let path = absolute(path)?;
        reject_links(&path)?;
        let path = path
            .canonicalize()
            .context("Der Eintrag ist nicht erreichbar.")?;
        ensure!(
            path.starts_with(&root),
            "Der Pfad liegt außerhalb der Projektbibliothek."
        );
        Ok(path)
    }

    fn folder(&self, path: &Path) -> Result<PathBuf> {
        let path = self.contained(path)?;
        ensure!(path.is_dir(), "Der ausgewählte Pfad ist kein Ordner.");
        Ok(path)
    }

    fn design_file(&self, path: &Path) -> Result<PathBuf> {
        let path = self.contained(path)?;
        ensure!(
            path.is_file()
                && path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .and_then(design_name)
                    .is_some(),
            "Bitte eine Gridfinity-Projektdatei auswählen."
        );
        Ok(path)
    }
}

/// Save a file chosen explicitly through "Save as". The native file dialog
/// confirms replacement; validation and staged writes still happen here.
pub fn save_design_file(path: &Path, cfg: &Design) -> Result<()> {
    let path = absolute(path)?;
    reject_links(&path)?;
    let parent = path.parent().context("Bitte einen Dateinamen wählen.")?;
    let parent = parent
        .canonicalize()
        .context("Der Zielordner ist nicht erreichbar.")?;
    ensure!(parent.is_dir(), "Der Zielordner ist kein Ordner.");
    let filename = path.file_name().context("Bitte einen Dateinamen wählen.")?;
    write_design(&parent.join(filename), cfg, true)
}

fn write_design(path: &Path, cfg: &Design, overwrite: bool) -> Result<()> {
    cfg.validate()
        .context("Das Modell enthält ungültige Einstellungen.")?;
    reject_links(path)?;
    let folder = path.parent().context("Der Zielordner fehlt.")?;
    let exists = match fs::symlink_metadata(path) {
        Ok(metadata) => {
            ensure!(
                !is_link(&metadata),
                "Die Zieldatei darf keine Verknüpfung sein."
            );
            ensure!(
                metadata.is_file(),
                "Dieser Name ist bereits durch einen Ordner belegt."
            );
            ensure!(
                overwrite,
                "Ein Modell mit diesem Namen ist bereits vorhanden."
            );
            true
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => false,
        Err(error) => return Err(error).context("Das Speicherziel ist nicht erreichbar."),
    };
    let bytes =
        serde_json::to_vec_pretty(cfg).context("Das Modell konnte nicht gespeichert werden.")?;
    ensure!(
        bytes.len() as u64 <= MAX_BYTES,
        "Die Projektdatei ist größer als 8 MB."
    );
    let (staged, mut file) = temporary(folder, "save")?;
    file.write_all(&bytes)
        .context("Die Projektdatei konnte nicht geschrieben werden.")?;
    file.sync_all()
        .context("Die Projektdatei konnte nicht sicher gespeichert werden.")?;
    drop(file);

    if exists {
        // Keep the original until the fully written replacement is ready. A
        // synced backup remains available if replacement and recovery both fail.
        let (mut backup, mut file) = temporary(folder, "backup")?;
        let mut original =
            File::open(path).context("Das bisherige Modell konnte nicht gesichert werden.")?;
        io::copy(&mut original, &mut file)
            .context("Das bisherige Modell konnte nicht gesichert werden.")?;
        file.sync_all()
            .context("Die Sicherung konnte nicht geschrieben werden.")?;
        drop(original);
        drop(file);
        if let Err(error) = fs::rename(&staged.path, path) {
            if fs::symlink_metadata(path).is_err()
                && let Err(recovery) = rename_no_replace(&backup.path, path)
            {
                backup.keep = true;
                bail!(
                    "Speichern fehlgeschlagen: {error}. Die Sicherung liegt unter '{}'. Wiederherstellung: {recovery}",
                    backup.path.display()
                );
            }
            return Err(error).context(
                "Das Modell konnte nicht ersetzt werden. Die bisherige Datei bleibt erhalten.",
            );
        }
    } else {
        rename_no_replace(&staged.path, path).context("Das Modell konnte nicht gespeichert werden. Der Name ist möglicherweise bereits vergeben.")?;
    }
    Ok(())
}

pub(crate) fn design_name(filename: &str) -> Option<&str> {
    let split = filename.len().checked_sub(SUFFIX.len())?;
    let suffix = filename.get(split..)?;
    (split > 0 && suffix.eq_ignore_ascii_case(SUFFIX)).then(|| &filename[..split])
}

fn validate_name(name: &str) -> Result<()> {
    ensure!(!name.trim().is_empty(), "Bitte einen Namen eingeben.");
    ensure!(
        name != "." && name != "..",
        "Dieser Name ist nicht erlaubt."
    );
    ensure!(
        name.encode_utf16().count() <= 120,
        "Der Name darf höchstens 120 Zeichen enthalten."
    );
    ensure!(
        !name.ends_with(['.', ' ']),
        "Der Name darf nicht mit einem Punkt oder Leerzeichen enden."
    );
    ensure!(
        !name
            .chars()
            .any(|c| c.is_control() || "<>:\"/\\|?*".contains(c)),
        "Der Name enthält ein unzulässiges Zeichen: < > : \" / \\ | ? *"
    );
    let stem = name
        .split('.')
        .next()
        .unwrap_or(name)
        .trim_end()
        .to_uppercase();
    let numbered_device = ["COM", "LPT"].iter().any(|prefix| {
        stem.strip_prefix(prefix).is_some_and(|rest| {
            matches!(
                rest,
                "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
            )
        })
    });
    ensure!(
        !numbered_device
            && !matches!(
                stem.as_str(),
                "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
            ),
        "Dieser Name ist von Windows reserviert."
    );
    Ok(())
}

fn absolute(path: &Path) -> Result<PathBuf> {
    ensure!(
        !path.components().any(|c| matches!(c, Component::ParentDir)),
        "Übergeordnete Pfadverweise sind hier nicht erlaubt."
    );
    Ok(if path.is_absolute() {
        path.to_owned()
    } else {
        std::env::current_dir()?.join(path)
    })
}

fn reject_links(path: &Path) -> Result<()> {
    for ancestor in path.ancestors().collect::<Vec<_>>().into_iter().rev() {
        if ancestor.as_os_str().is_empty() {
            continue;
        }
        match fs::symlink_metadata(ancestor) {
            Ok(metadata) => ensure!(
                !is_link(&metadata),
                "Verknüpfungen sind in der Bibliothek nicht erlaubt."
            ),
            Err(error) if error.kind() == io::ErrorKind::NotFound => break,
            Err(error) => {
                return Err(error).context("Der Projektpfad konnte nicht geprüft werden.");
            }
        }
    }
    Ok(())
}

fn is_link(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x400 != 0 // FILE_ATTRIBUTE_REPARSE_POINT
    }
    #[cfg(not(windows))]
    {
        metadata.file_type().is_symlink()
    }
}

fn ensure_absent(path: &Path) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(_) => bail!("Ein Eintrag mit diesem Namen ist bereits vorhanden."),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).context("Der Zielname konnte nicht geprüft werden."),
    }
}

struct Temporary {
    path: PathBuf,
    keep: bool,
}
impl Drop for Temporary {
    fn drop(&mut self) {
        if !self.keep {
            let _ = fs::remove_file(&self.path);
        }
    }
}

fn temporary(folder: &Path, purpose: &str) -> Result<(Temporary, File)> {
    for _ in 0..100 {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let sequence = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = folder.join(format!(
            ".gridfinity-{purpose}-{}-{nanos}-{sequence}.tmp",
            std::process::id()
        ));
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => return Ok((Temporary { path, keep: false }, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(error).context("Temporäre Projektdatei konnte nicht angelegt werden.");
            }
        }
    }
    bail!("Kein freier Name für die temporäre Projektdatei gefunden.")
}

#[cfg(windows)]
fn rename_no_replace(source: &Path, target: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn MoveFileW(existing: *const u16, new: *const u16) -> i32;
    }
    let source: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let target: Vec<u16> = target.as_os_str().encode_wide().chain(Some(0)).collect();
    // MoveFileW refuses an existing destination, including a file created after our check.
    // Both buffers are NUL-terminated and remain alive throughout the synchronous call.
    if unsafe { MoveFileW(source.as_ptr(), target.as_ptr()) } != 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(not(windows))]
fn rename_no_replace(source: &Path, target: &Path) -> io::Result<()> {
    if source.is_file() {
        fs::hard_link(source, target)?;
        fs::remove_file(source)
    } else {
        if fs::symlink_metadata(target).is_ok() {
            return Err(io::ErrorKind::AlreadyExists.into());
        }
        fs::rename(source, target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::design::{CutShape, Cutout, LabelArea, Product, Wall};

    struct Sandbox(PathBuf);
    impl Sandbox {
        fn new() -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "gridfinity-library-test-{}-{nonce}-{}",
                std::process::id(),
                TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path.canonicalize().unwrap())
        }
        fn library(&self) -> Library {
            Library::new(self.0.join("Projekte")).unwrap()
        }
    }
    impl Drop for Sandbox {
        fn drop(&mut self) {
            // Only remove our uniquely created test directory, never an application library.
            if self.0.parent() == std::env::temp_dir().canonicalize().ok().as_deref()
                && self
                    .0
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("gridfinity-library-test-")
            {
                let _ = fs::remove_dir_all(&self.0);
            }
        }
    }

    #[test]
    fn folders_and_models_roundtrip_without_losing_parameters() {
        let sandbox = Sandbox::new();
        let library = sandbox.library();
        let folder = library.create_folder(&library.root, "Werkstatt").unwrap();
        let nested = library.create_folder(&folder, "Schrauben").unwrap();
        let mut cfg = Design {
            title: "Vier-Fach-Box".into(),
            label: "M3".into(),
            label_area: LabelArea::Recess,
            label_width: 28.0,
            label_height: 7.0,
            label_depth: 0.2,
            grid_x: 3,
            ..Design::default()
        };
        cfg.walls.push(Wall {
            x1: 0.0,
            y1: -40.0,
            x2: 0.0,
            y2: 40.0,
            thickness: 0.8,
            height: 10.0,
        });
        let path = library.save_design(&nested, "Box M3", &cfg, false).unwrap();
        let loaded = library.load_design(&path).unwrap();
        assert_eq!(
            serde_json::to_value(&loaded).unwrap(),
            serde_json::to_value(&cfg).unwrap()
        );
        assert_eq!(library.entries(&nested).unwrap()[0].name, "Box M3");
        let model = library.rename(&path, "Sortierbox").unwrap();
        assert_eq!(model.file_name().unwrap(), "Sortierbox.gridfinity.json");
        assert!(!path.exists());
        let moved_folder = library.rename(&folder, "Garage").unwrap();
        assert!(
            moved_folder
                .join("Schrauben/Sortierbox.gridfinity.json")
                .is_file()
        );

        let insert = Design {
            product: Product::Insert,
            cutouts: vec![Cutout {
                shape: CutShape::Stl,
                stl_path: Some("C:\\Werkzeuge\\bohrer.stl".into()),
                ..Cutout::default()
            }],
            ..Design::default()
        };
        let path = library
            .save_design(&library.root, "Werkzeugeinsatz", &insert, false)
            .unwrap();
        assert_eq!(
            library.load_design(&path).unwrap().cutouts[0].stl_path,
            insert.cutouts[0].stl_path
        );
    }

    #[test]
    fn legacy_label_projects_migrate_on_load_without_rewriting_the_original() {
        let sandbox = Sandbox::new();
        let library = sandbox.library();
        let original = br#"{"title":"Kabelbox","product":"Bin","reference_profile":true,"wall":1.2,"floor":1.21,"label_area":"Clip","label_width":40.0,"label_height":20.0,"label_clip_clearance":0.3,"label":"KABEL"}"#;
        let path = library.root.join("Kabelbox.gridfinity.json");
        fs::write(&path, original).unwrap();
        let loaded = library.load_design(&path).unwrap();
        assert_eq!(loaded.title, "Kabelbox");
        assert_eq!(loaded.label, "KABEL");
        assert_eq!(loaded.label_area, LabelArea::SlideIn);
        assert_eq!(loaded.label_insert_clearance, 0.3);
        assert_eq!(loaded.label_height, loaded.label_height_limit());
        assert_eq!(fs::read(&path).unwrap(), original);
        let saved = library
            .save_design(&library.root, "Kabelbox neu", &loaded, false)
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&fs::read(saved).unwrap()).unwrap();
        assert_eq!(json["label_area"], "SlideIn");
        assert!(json.get("label_clip_clearance").is_none());
    }

    #[test]
    fn listing_sorts_folders_first_and_ignores_unrelated_files() {
        let sandbox = Sandbox::new();
        let library = sandbox.library();
        for name in ["Zubehör", "ablage"] {
            library.create_folder(&library.root, name).unwrap();
        }
        for name in ["Zeta", "Alpha"] {
            library
                .save_design(&library.root, name, &Design::default(), false)
                .unwrap();
        }
        fs::write(library.root.join("notes.txt"), "Notiz").unwrap();
        fs::write(library.root.join(".gridfinity-save.tmp"), "temporär").unwrap();
        fs::write(library.root.join(".gridfinity.json"), "{}").unwrap();
        let entries = library.entries(&library.root).unwrap();
        assert_eq!(
            entries.iter().map(|e| e.name.as_str()).collect::<Vec<_>>(),
            ["ablage", "Zubehör", "Alpha", "Zeta"]
        );
        assert!(entries[..2].iter().all(|e| e.is_dir));
        assert!(entries[2..].iter().all(|e| !e.is_dir));
    }

    #[test]
    fn names_and_paths_cannot_escape_the_library() {
        let sandbox = Sandbox::new();
        let library = sandbox.library();
        let cfg = Design::default();
        for name in [
            "",
            " ",
            ".",
            "..",
            "../fremd",
            "a/b",
            "a\\b",
            "C:\\fremd",
            "a:b",
            "a?",
            "a*",
            "a|b",
            "a<",
            "a>",
            "a\"",
            "a\n",
            "Ende.",
            "Ende ",
            "CON",
            "nul.txt",
            "LPT9",
            "COM¹",
            "PRN",
            "CONOUT$",
        ] {
            assert!(
                library.create_folder(&library.root, name).is_err(),
                "folder: {name:?}"
            );
            assert!(
                library
                    .save_design(&library.root, name, &cfg, false)
                    .is_err(),
                "model: {name:?}"
            );
        }
        assert!(
            library
                .create_folder(&library.root, &"a".repeat(121))
                .is_err()
        );
        assert!(library.entries(&sandbox.0).is_err());
        assert!(library.create_folder(&sandbox.0, "Draußen").is_err());
        assert!(
            library
                .save_design(&sandbox.0, "Draußen", &cfg, false)
                .is_err()
        );
        assert!(library.entries(&library.root.join("..")).is_err());
        assert!(library.rename(&library.root, "Andere Bibliothek").is_err());
        let outside = sandbox.0.join("außerhalb.gridfinity.json");
        fs::write(&outside, serde_json::to_vec(&cfg).unwrap()).unwrap();
        assert!(library.load_design(&outside).is_err());
        assert!(library.rename(&outside, "Geändert").is_err());
        assert!(outside.is_file());
    }

    #[test]
    fn conflicts_never_overwrite_and_explicit_save_replaces_safely() {
        let sandbox = Sandbox::new();
        let library = sandbox.library();
        let mut cfg = Design::default();
        let first = library
            .save_design(&library.root, "Box", &cfg, false)
            .unwrap();
        let original = fs::read(&first).unwrap();
        cfg.grid_x = 4;
        assert!(
            library
                .save_design(&library.root, "Box", &cfg, false)
                .is_err()
        );
        assert_eq!(fs::read(&first).unwrap(), original);
        let other = library
            .save_design(&library.root, "Andere Box", &cfg, false)
            .unwrap();
        assert!(library.rename(&other, "Box").is_err());
        assert!(other.is_file());
        assert_eq!(fs::read(&first).unwrap(), original);
        library
            .save_design(&library.root, "Box", &cfg, true)
            .unwrap();
        assert_eq!(library.load_design(&first).unwrap().grid_x, 4);
        assert_eq!(library.entries(&library.root).unwrap().len(), 2);
        assert_eq!(fs::read_dir(&library.root).unwrap().count(), 2);
        library.create_folder(&library.root, "Ablage").unwrap();
        assert!(library.create_folder(&library.root, "Ablage").is_err());
        let folder = library
            .create_folder(&library.root, "Andere Ablage")
            .unwrap();
        assert!(library.rename(&folder, "Ablage").is_err());
        assert!(folder.is_dir());
    }

    #[cfg(windows)]
    #[test]
    fn renaming_only_letter_case_keeps_the_contents_and_other_entries() {
        let sandbox = Sandbox::new();
        let library = sandbox.library();
        let folder = library.create_folder(&library.root, "schrank").unwrap();
        let first = library
            .save_design(&folder, "box", &Design::default(), false)
            .unwrap();
        let before = fs::read(&first).unwrap();
        let renamed = library.rename(&first, "Box").unwrap();
        assert_eq!(renamed.file_name().unwrap(), "Box.gridfinity.json");
        assert_eq!(fs::read(&renamed).unwrap(), before);
        let folder = library.rename(&folder, "Schrank").unwrap();
        assert_eq!(folder.file_name().unwrap(), "Schrank");
        assert_eq!(
            fs::read(folder.join("Box.gridfinity.json")).unwrap(),
            before
        );
        assert_eq!(library.entries(&folder).unwrap().len(), 1);
        let other = library
            .save_design(&folder, "Andere", &Design::default(), false)
            .unwrap();
        assert!(library.rename(&other, "BOX").is_err());
        assert_eq!(
            fs::read(folder.join("Box.gridfinity.json")).unwrap(),
            before
        );
        assert!(other.is_file());
    }

    #[test]
    fn malformed_or_oversized_models_are_rejected() {
        let sandbox = Sandbox::new();
        let library = sandbox.library();
        let path = library.root.join("Ungültig.gridfinity.json");
        for data in ["{", "null", "[]", "{\"grid_x\":0}", "{\"version\":999}"] {
            fs::write(&path, data).unwrap();
            assert!(library.load_design(&path).is_err(), "{data}");
        }
        let file = File::create(&path).unwrap();
        file.set_len(MAX_BYTES + 1).unwrap();
        drop(file);
        assert!(library.load_design(&path).is_err());
        let cfg = Design {
            grid_x: 0,
            ..Design::default()
        };
        assert!(
            library
                .save_design(&library.root, "Neu", &cfg, false)
                .is_err()
        );
        assert!(!library.root.join("Neu.gridfinity.json").exists());
    }

    #[test]
    fn standalone_save_validates_before_replacing_and_cleans_staging_files() {
        let sandbox = Sandbox::new();
        let path = sandbox.0.join("Entwurf.json");
        save_design_file(&path, &Design::default()).unwrap();
        let original = fs::read(&path).unwrap();
        let invalid = Design {
            grid_x: 0,
            ..Design::default()
        };
        assert!(save_design_file(&path, &invalid).is_err());
        assert_eq!(fs::read(&path).unwrap(), original);
        let changed = Design {
            grid_x: 4,
            ..Design::default()
        };
        save_design_file(&path, &changed).unwrap();
        assert_eq!(
            Design::from_json(&fs::read(&path).unwrap()).unwrap().grid_x,
            4
        );
        assert_eq!(fs::read_dir(&sandbox.0).unwrap().count(), 1);
        assert!(save_design_file(&sandbox.0, &changed).is_err());
        assert!(save_design_file(&sandbox.0.join("Fehlt/Datei.json"), &changed).is_err());
        assert!(path.is_file());
    }

    #[cfg(windows)]
    #[test]
    fn standalone_save_preserves_the_original_when_windows_blocks_replacement() {
        use std::os::windows::fs::OpenOptionsExt;
        let sandbox = Sandbox::new();
        let path = sandbox.0.join("Entwurf.json");
        save_design_file(&path, &Design::default()).unwrap();
        let original = fs::read(&path).unwrap();
        let locked = OpenOptions::new()
            .read(true)
            .share_mode(1)
            .open(&path)
            .unwrap();
        assert!(
            save_design_file(
                &path,
                &Design {
                    grid_x: 4,
                    ..Design::default()
                }
            )
            .is_err()
        );
        assert_eq!(fs::read(&path).unwrap(), original);
        assert_eq!(fs::read_dir(&sandbox.0).unwrap().count(), 1);
        drop(locked);
    }

    #[test]
    fn mixed_case_design_suffix_is_stripped_consistently() {
        assert_eq!(design_name("Box.GRIDFINITY.JSON"), Some("Box"));
        assert_eq!(design_name("Kästen.gridfinity.json"), Some("Kästen"));
        assert_eq!(design_name(".gridfinity.json"), None);
        assert_eq!(design_name("Box.json"), None);
    }

    fn entry(path: &Path, is_dir: bool) -> Entry {
        Entry {
            name: path.file_name().unwrap().to_string_lossy().into_owned(),
            path: path.to_owned(),
            is_dir,
        }
    }

    #[test]
    fn deleting_a_design_preserves_other_files_and_allows_corrupt_designs() {
        let sandbox = Sandbox::new();
        let library = sandbox.library();
        let model = library
            .save_design(&library.root, "Box", &Design::default(), false)
            .unwrap();
        let other = library
            .save_design(&library.root, "Behalten", &Design::default(), false)
            .unwrap();
        library.delete(&entry(&model, false)).unwrap();
        assert!(!model.exists());
        assert!(other.is_file());
        fs::write(&model, "{ beschädigt").unwrap();
        library.delete(&entry(&model, false)).unwrap();
        assert!(!model.exists());
        assert!(library.load_design(&other).is_ok());
    }

    #[test]
    fn deleting_a_project_removes_all_nested_contents_but_not_siblings() {
        let sandbox = Sandbox::new();
        let library = sandbox.library();
        let project = library.create_folder(&library.root, "Schrank").unwrap();
        let folder = library.create_folder(&project, "Kabel").unwrap();
        let nested = library.create_folder(&folder, "USB").unwrap();
        library
            .save_design(&nested, "Box", &Design::default(), false)
            .unwrap();
        fs::write(folder.join("Notizen.txt"), "Zum Projekt gehörende Notizen").unwrap();
        let sibling = library.create_folder(&library.root, "Schrank 2").unwrap();
        library.delete(&entry(&project, true)).unwrap();
        assert!(!project.exists());
        assert!(sibling.is_dir());
        assert!(library.root.is_dir());
        library.delete(&entry(&sibling, true)).unwrap();
        assert!(library.entries(&library.root).unwrap().is_empty());
    }

    #[test]
    fn deletion_rejects_root_escape_unknown_files_and_changed_entry_kinds() {
        let sandbox = Sandbox::new();
        let library = sandbox.library();
        let folder = library.create_folder(&library.root, "Bleibt").unwrap();
        let outside = sandbox.0.join("Projekte daneben");
        fs::create_dir(&outside).unwrap();
        let note = library.root.join("Notizen.txt");
        fs::write(&note, "Keine Entwurfsdatei").unwrap();
        assert!(library.delete(&entry(&library.root, true)).is_err());
        assert!(library.delete(&entry(&outside, true)).is_err());
        assert!(
            library
                .delete(&entry(&library.root.join(".."), true))
                .is_err()
        );
        assert!(library.delete(&entry(&note, false)).is_err());
        assert!(library.delete(&entry(&folder, false)).is_err());
        assert!(
            library
                .delete(&entry(&library.root.join("Fehlt.gridfinity.json"), false))
                .is_err()
        );
        assert!(folder.is_dir());
        assert!(outside.is_dir());
        assert!(note.is_file());
        let model = library
            .save_design(&folder, "Bleibt", &Design::default(), false)
            .unwrap();
        assert!(library.delete(&entry(&model, true)).is_err());
        assert!(model.is_file());
    }

    #[test]
    fn deletion_checks_the_entire_tree_before_removing_anything() {
        let sandbox = Sandbox::new();
        let library = sandbox.library();
        let project = library.create_folder(&library.root, "Schrank").unwrap();
        let model = library
            .save_design(&project, "Box", &Design::default(), false)
            .unwrap();
        let outside = sandbox.0.join("Außerhalb");
        fs::create_dir(&outside).unwrap();
        let protected = outside.join("Notizen.txt");
        fs::write(&protected, "Bleibt unangetastet").unwrap();
        let nested = library.create_folder(&project, "Unterordner").unwrap();
        let linked = nested.join("Verknüpfung");
        #[cfg(windows)]
        {
            if let Err(error) = std::os::windows::fs::symlink_dir(&outside, &linked) {
                assert!(
                    error.kind() == io::ErrorKind::PermissionDenied
                        || error.raw_os_error() == Some(1314),
                    "{error}"
                );
                // Directory junctions work without the symlink privilege. Both
                // paths are newly created descendants of this test's sandbox.
                let result = std::process::Command::new("powershell.exe")
                    .args(["-NoProfile", "-NonInteractive", "-Command",
                        "$ErrorActionPreference='Stop'; New-Item -ItemType Junction -Path $env:GRIDFINITY_TEST_LINK -Target $env:GRIDFINITY_TEST_TARGET | Out-Null"])
                    .env("GRIDFINITY_TEST_LINK", &linked)
                    .env("GRIDFINITY_TEST_TARGET", &outside)
                    .output().unwrap();
                assert!(
                    result.status.success(),
                    "{}",
                    String::from_utf8_lossy(&result.stderr)
                );
            }
        }
        #[cfg(not(windows))]
        std::os::unix::fs::symlink(&outside, &linked).unwrap();
        assert!(library.delete(&entry(&project, true)).is_err());
        assert!(library.delete(&entry(&linked, true)).is_err());
        assert!(model.is_file());
        assert_eq!(
            fs::read_to_string(&protected).unwrap(),
            "Bleibt unangetastet"
        );
        #[cfg(windows)]
        fs::remove_dir(&linked).unwrap();
        #[cfg(not(windows))]
        fs::remove_file(&linked).unwrap();
        library.delete(&entry(&project, true)).unwrap();
        assert!(protected.is_file());
    }

    #[cfg(windows)]
    #[test]
    fn failed_delete_reports_error_and_keeps_a_locked_design() {
        use std::os::windows::fs::OpenOptionsExt;
        let sandbox = Sandbox::new();
        let library = sandbox.library();
        let path = library
            .save_design(&library.root, "Gesperrt", &Design::default(), false)
            .unwrap();
        let locked = OpenOptions::new()
            .read(true)
            .share_mode(1)
            .open(&path)
            .unwrap();
        assert!(library.delete(&entry(&path, false)).is_err());
        assert!(path.is_file());
        drop(locked);
        library.delete(&entry(&path, false)).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn failed_overwrite_preserves_original_and_removes_staging_files() {
        use std::os::windows::fs::OpenOptionsExt;
        let sandbox = Sandbox::new();
        let library = sandbox.library();
        let path = library
            .save_design(&library.root, "Box", &Design::default(), false)
            .unwrap();
        let original = fs::read(&path).unwrap();
        // Allow reads (including the backup), but make Windows reject replacing the file.
        let locked = OpenOptions::new()
            .read(true)
            .share_mode(1)
            .open(&path)
            .unwrap();
        let changed = Design {
            grid_x: 4,
            ..Design::default()
        };
        assert!(
            library
                .save_design(&library.root, "Box", &changed, true)
                .is_err()
        );
        assert_eq!(fs::read(&path).unwrap(), original);
        assert_eq!(fs::read_dir(&library.root).unwrap().count(), 1);
        drop(locked);
        assert_eq!(library.load_design(&path).unwrap().grid_x, 2);
    }

    #[test]
    fn linked_folders_and_files_are_neither_listed_nor_followed() {
        let sandbox = Sandbox::new();
        let library = sandbox.library();
        let outside = sandbox.0.join("Außerhalb");
        fs::create_dir(&outside).unwrap();
        let project = outside.join("Projekt.gridfinity.json");
        fs::write(&project, serde_json::to_vec(&Design::default()).unwrap()).unwrap();
        let folder_link = library.root.join("Verknüpfter Ordner");
        let file_link = library.root.join("Verknüpft.gridfinity.json");
        #[cfg(windows)]
        let linked = std::os::windows::fs::symlink_dir(&outside, &folder_link);
        #[cfg(not(windows))]
        let linked = std::os::unix::fs::symlink(&outside, &folder_link);
        if let Err(error) = linked {
            if error.kind() == io::ErrorKind::PermissionDenied || error.raw_os_error() == Some(1314)
            {
                eprintln!(
                    "Symlink-Prüfung übersprungen: Dieses Testkonto darf keine Verknüpfungen anlegen."
                );
                return;
            }
            panic!("Testverknüpfung konnte nicht angelegt werden: {error}");
        }
        #[cfg(windows)]
        std::os::windows::fs::symlink_file(&project, &file_link).unwrap();
        #[cfg(not(windows))]
        std::os::unix::fs::symlink(&project, &file_link).unwrap();
        assert!(library.entries(&library.root).unwrap().is_empty());
        assert!(library.entries(&folder_link).is_err());
        assert!(library.create_folder(&folder_link, "Neu").is_err());
        assert!(
            library
                .save_design(&folder_link, "Neu", &Design::default(), false)
                .is_err()
        );
        assert!(
            library
                .load_design(&folder_link.join("Projekt.gridfinity.json"))
                .is_err()
        );
        assert!(library.load_design(&file_link).is_err());
        assert!(save_design_file(&file_link, &Design::default()).is_err());
        assert!(save_design_file(&folder_link.join("Neu.json"), &Design::default()).is_err());
        assert!(
            library
                .save_design(&library.root, "Verknüpft", &Design::default(), true)
                .is_err()
        );
        assert!(library.rename(&folder_link, "Verschoben").is_err());
        assert!(library.delete(&entry(&folder_link, true)).is_err());
        assert!(library.delete(&entry(&file_link, false)).is_err());
        assert!(Library::new(folder_link).is_err());
        assert!(project.is_file());
    }
}
