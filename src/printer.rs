//! Local printer choice and rectangular build-volume checks.
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub profile_id: Option<String>,
    pub custom_size: [f64; 3],
    /// Clearance on each side of the XY bed; does not reduce the Z height.
    pub margin: f64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            profile_id: None,
            custom_size: [256.0; 3],
            margin: 0.0,
        }
    }
}

impl Settings {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.custom_size
                .iter()
                .all(|n| n.is_finite() && (1.0..=2000.0).contains(n)),
            "Die eigenen Druckermaße müssen zwischen 1 und 2000 mm liegen."
        );
        ensure!(
            self.margin.is_finite() && (0.0..=50.0).contains(&self.margin),
            "Der Druckrand muss zwischen 0 und 50 mm je Seite liegen."
        );
        if let Some(id) = &self.profile_id {
            ensure!(
                id == "custom" || crate::printer_profiles::PROFILES.iter().any(|p| p.id == id),
                "Das gespeicherte Druckerprofil ist unbekannt."
            );
        }
        if let Some(size) = self.volume() {
            ensure!(
                size.iter().all(|n| n.is_finite() && *n > 0.0)
                    && size[0] > 2.0 * self.margin
                    && size[1] > 2.0 * self.margin,
                "Der Druckrand lässt keinen nutzbaren Bauraum übrig."
            );
        }
        Ok(())
    }

    pub fn volume(&self) -> Option<[f64; 3]> {
        match self.profile_id.as_deref()? {
            "custom" => Some(self.custom_size),
            id => crate::printer_profiles::PROFILES
                .iter()
                .find(|p| p.id == id)
                .map(|p| p.size),
        }
    }

    pub fn name(&self) -> String {
        match self.profile_id.as_deref() {
            None => "Kein Drucker ausgewählt".into(),
            Some("custom") => "Eigener Drucker".into(),
            Some(id) => crate::printer_profiles::PROFILES
                .iter()
                .find(|p| p.id == id)
                .map(|p| format!("{} {}", p.brand, p.name))
                .unwrap_or_else(|| "Unbekanntes Druckerprofil".into()),
        }
    }

    pub fn usable_size(&self) -> Option<[f64; 3]> {
        self.validate().ok()?;
        self.volume()
            .map(|[x, y, z]| [x - 2.0 * self.margin, y - 2.0 * self.margin, z])
    }

    pub fn grid_capacity(&self) -> Option<[u32; 2]> {
        let [x, y, _] = self.usable_size()?;
        Some([(x / 42.0).floor() as u32, (y / 42.0).floor() as u32])
    }

    pub fn load() -> Result<Self> {
        Self::load_from(&settings_path())
    }

    pub fn save(&self) -> Result<()> {
        self.save_to(&settings_path())
    }

    fn load_from(path: &Path) -> Result<Self> {
        let bytes = match fs::read(path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(error) => {
                return Err(error)
                    .context("Die Druckereinstellungen konnten nicht gelesen werden.");
            }
        };
        let settings: Self = serde_json::from_slice(&bytes)
            .context("Die Druckereinstellungen enthalten ungültiges JSON.")?;
        settings.validate()?;
        Ok(settings)
    }

    fn save_to(&self, path: &Path) -> Result<()> {
        self.validate()?;
        let folder = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        fs::create_dir_all(folder)
            .context("Der Einstellungsordner konnte nicht angelegt werden.")?;
        match fs::symlink_metadata(path) {
            Ok(meta) => ensure!(
                meta.is_file() && !meta.file_type().is_symlink(),
                "Die Einstellungsdatei ist keine reguläre Datei."
            ),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(error).context("Die Einstellungsdatei konnte nicht geprüft werden.");
            }
        }
        let bytes = serde_json::to_vec_pretty(self)?;
        let (staged, mut file) = temporary(folder)?;
        file.write_all(&bytes)
            .context("Die Druckereinstellungen konnten nicht geschrieben werden.")?;
        file.sync_all()
            .context("Die Druckereinstellungen konnten nicht sicher gespeichert werden.")?;
        drop(file);
        // Same-directory replacement keeps the previous file until a complete,
        // synced successor is ready. A failed write never truncates the original.
        fs::rename(&staged.0, path)
            .context("Die Druckereinstellungen konnten nicht ersetzt werden. Die bisherige Datei bleibt erhalten.")?;
        Ok(())
    }
}

fn settings_path() -> PathBuf {
    if let Some(path) =
        std::env::var_os("GRIDFINITY_DESIGN2_SETTINGS_FILE").filter(|p| !p.is_empty())
    {
        return path.into();
    }
    std::env::var_os("LOCALAPPDATA")
        .filter(|p| !p.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::var_os("USERPROFILE")
                .map(PathBuf::from)
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
                .join("AppData")
                .join("Local")
        })
        .join("GridfinityStudioDesign2")
        .join("printer.json")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fit {
    Fits,
    Rotate90,
    TooLarge,
}

pub fn fit(size: [f64; 3], usable: [f64; 3]) -> Fit {
    // Small numerical slack accounts for the STL's f32 coordinates, not an
    // allowance to print outside the bed. Only an in-plane rotation is offered.
    const EPSILON: f64 = 0.0001;
    if size
        .iter()
        .chain(&usable)
        .any(|n| !n.is_finite() || *n <= 0.0)
        || size[2] > usable[2] + EPSILON
    {
        return Fit::TooLarge;
    }
    if size[0] <= usable[0] + EPSILON && size[1] <= usable[1] + EPSILON {
        Fit::Fits
    } else if size[1] <= usable[0] + EPSILON && size[0] <= usable[1] + EPSILON {
        Fit::Rotate90
    } else {
        Fit::TooLarge
    }
}

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);
struct Temporary(PathBuf);
impl Drop for Temporary {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}
fn temporary(folder: &Path) -> Result<(Temporary, fs::File)> {
    for _ in 0..100 {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let sequence = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = folder.join(format!(
            ".printer-{}-{nanos}-{sequence}.tmp",
            std::process::id()
        ));
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => return Ok((Temporary(path), file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(error)
                    .context("Temporäre Druckereinstellungen konnten nicht angelegt werden.");
            }
        }
    }
    bail!("Kein temporärer Dateiname für Druckereinstellungen verfügbar.")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn printer_settings_apply_margin_only_on_xy_and_allow_manual_sizes() {
        let default = Settings::default();
        assert_eq!(default.volume(), None);
        assert_eq!(default.grid_capacity(), None);
        assert!(default.validate().is_ok());
        let mut cfg = Settings {
            profile_id: Some("custom".into()),
            custom_size: [256.0, 252.0, 200.0],
            margin: 0.0,
        };
        assert_eq!(cfg.grid_capacity(), Some([6, 6]));
        cfg.margin = 3.0;
        assert_eq!(cfg.usable_size(), Some([250.0, 246.0, 200.0]));
        assert_eq!(cfg.grid_capacity(), Some([5, 5]));
        cfg.custom_size = [20.0, 84.0, 150.0];
        cfg.margin = 0.0;
        assert_eq!(cfg.grid_capacity(), Some([0, 2]));
        for profile in crate::printer_profiles::PROFILES {
            let selected = Settings {
                profile_id: Some(profile.id.into()),
                ..Default::default()
            };
            assert!(selected.validate().is_ok());
            assert_eq!(selected.volume(), Some(profile.size));
            assert!(selected.name().contains(profile.name));
        }
    }

    #[test]
    fn printer_fit_checks_both_xy_orientations_and_never_rotates_height() {
        assert_eq!(fit([252.0, 252.0, 200.0], [256.0, 256.0, 200.0]), Fit::Fits);
        assert_eq!(
            fit([250.0, 200.0, 100.0], [210.0, 260.0, 100.0]),
            Fit::Rotate90
        );
        assert_eq!(
            fit([250.0, 220.0, 100.0], [210.0, 260.0, 100.0]),
            Fit::TooLarge
        );
        assert_eq!(
            fit([20.0, 20.0, 101.0], [256.0, 256.0, 100.0]),
            Fit::TooLarge
        );
        assert_eq!(fit([42.0, 42.0, 7.0], [42.0, 42.0, 7.0]), Fit::Fits);
        for invalid in [f64::NAN, f64::INFINITY, -1.0, 0.0] {
            assert_eq!(fit([invalid, 1.0, 1.0], [10.0; 3]), Fit::TooLarge);
            assert_eq!(fit([1.0; 3], [10.0, invalid, 10.0]), Fit::TooLarge);
        }
    }

    #[test]
    fn printer_settings_reject_invalid_profiles_sizes_and_margins() {
        assert!(
            Settings {
                profile_id: Some("missing-printer".into()),
                ..Default::default()
            }
            .validate()
            .is_err()
        );
        for invalid in [f64::NAN, f64::INFINITY, -1.0, 0.0, 2001.0] {
            assert!(
                Settings {
                    custom_size: [invalid, 200.0, 200.0],
                    ..Default::default()
                }
                .validate()
                .is_err()
            );
        }
        for margin in [-1.0, f64::NAN, 51.0, 20.0] {
            let cfg = Settings {
                profile_id: Some("custom".into()),
                custom_size: [40.0, 40.0, 200.0],
                margin,
            };
            assert!(cfg.validate().is_err());
            assert_eq!(cfg.usable_size(), None);
        }
    }

    #[test]
    fn printer_settings_persist_and_preserve_previous_file_on_invalid_save() {
        let folder = std::env::temp_dir().join(format!(
            "gridfinity-printer-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path = folder.join("printer.json");
        assert_eq!(Settings::load_from(&path).unwrap(), Settings::default());
        let cfg = Settings {
            profile_id: Some("custom".into()),
            custom_size: [300.0, 220.0, 240.0],
            margin: 2.5,
        };
        cfg.save_to(&path).unwrap();
        assert_eq!(Settings::load_from(&path).unwrap(), cfg);
        let replacement = Settings {
            margin: 5.0,
            ..cfg.clone()
        };
        replacement.save_to(&path).unwrap();
        assert_eq!(Settings::load_from(&path).unwrap(), replacement);
        let before = fs::read(&path).unwrap();
        assert!(
            Settings {
                margin: f64::NAN,
                ..cfg
            }
            .save_to(&path)
            .is_err()
        );
        assert_eq!(fs::read(&path).unwrap(), before);
        assert_eq!(fs::read_dir(&folder).unwrap().count(), 1);
        fs::write(&path, b"invalid JSON").unwrap();
        assert!(Settings::load_from(&path).is_err());
        fs::remove_file(&path).unwrap();
        fs::remove_dir(&folder).unwrap();
    }
}
