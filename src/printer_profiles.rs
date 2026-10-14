//! Geprüfte rechteckige Bauraumprofile, Stand 13.10.2026.
//! Quellen und Grenzen der Maßprüfung stehen in `PRINTERS.md`.
//! Slicer-Sperrflächen und Platz für Brim/Stützen sind nicht in `size` enthalten.

#[derive(Debug, Clone, Copy)]
pub struct Profile {
    pub id: &'static str,
    pub brand: &'static str,
    pub name: &'static str,
    /// X (Breite), Y (Tiefe), Z (Höhe) in Millimetern.
    pub size: [f64; 3],
    pub note: &'static str,
    pub source: &'static str,
}

const NOMINAL: &str =
    "Herstellerbauraum; Slicer-Sperrflächen und Druckhilfen zusätzlich berücksichtigen.";
const BAMBU_250: &str = "Standard-Slicerhöhe 250 mm; nominell 256 mm. Slicer-Sperrfläche beachten. Angepasste Maße über Benutzerdefiniert.";

pub const PROFILES: &[Profile] = &[
    Profile {
        id: "bambu-a1-mini",
        brand: "Bambu Lab",
        name: "A1 mini",
        size: [180.0, 180.0, 180.0],
        note: NOMINAL,
        source: "https://cdn1.bambulab.com/documentation/quick-start-f507128172bdf/Quick%20start%20guide%20-%20A1%20mini-EN.pdf",
    },
    Profile {
        id: "bambu-a1",
        brand: "Bambu Lab",
        name: "A1",
        size: [256.0, 256.0, 256.0],
        note: NOMINAL,
        source: "https://cdn1.bambulab.com/documentation/quick-start-a75adcb1d5d5e/Quick%20Start%20Guide%20for%20A1.pdf",
    },
    Profile {
        id: "bambu-a2l",
        brand: "Bambu Lab",
        name: "A2L",
        size: [330.0, 320.0, 325.0],
        note: NOMINAL,
        source: "https://blog.bambulab.com/bambu-lab-a2l-creative-playground-extra-large/",
    },
    Profile {
        id: "bambu-p1p",
        brand: "Bambu Lab",
        name: "P1P",
        size: [256.0, 256.0, 250.0],
        note: BAMBU_250,
        source: "https://us.store.bambulab.com/products/p1s?skr=yes",
    },
    Profile {
        id: "bambu-p1s",
        brand: "Bambu Lab",
        name: "P1S",
        size: [256.0, 256.0, 250.0],
        note: BAMBU_250,
        source: "https://us.store.bambulab.com/products/p1s?skr=yes",
    },
    Profile {
        id: "bambu-p2s",
        brand: "Bambu Lab",
        name: "P2S",
        size: [256.0, 256.0, 256.0],
        note: NOMINAL,
        source: "https://blog.bambulab.com/the-icon-redefined-meet-the-p2s-a-completely-reengineered-version-of-the-ultra-productive-p1-series/",
    },
    Profile {
        id: "bambu-x1",
        brand: "Bambu Lab",
        name: "X1",
        size: [256.0, 256.0, 250.0],
        note: BAMBU_250,
        source: "https://raw.githubusercontent.com/bambulab/BambuStudio/master/resources/profiles/BBL/machine/Bambu%20Lab%20X1%200.4%20nozzle.json",
    },
    Profile {
        id: "bambu-x1-carbon",
        brand: "Bambu Lab",
        name: "X1 Carbon",
        size: [256.0, 256.0, 250.0],
        note: BAMBU_250,
        source: "https://us.store.bambulab.com/products/x1-carbon?variant=42698346037384",
    },
    Profile {
        id: "bambu-x1e",
        brand: "Bambu Lab",
        name: "X1E",
        size: [256.0, 256.0, 250.0],
        note: BAMBU_250,
        source: "https://raw.githubusercontent.com/bambulab/BambuStudio/master/resources/profiles/BBL/machine/Bambu%20Lab%20X1E%200.4%20nozzle.json",
    },
    Profile {
        id: "bambu-h2s",
        brand: "Bambu Lab",
        name: "H2S",
        size: [340.0, 320.0, 340.0],
        note: NOMINAL,
        source: "https://blog.bambulab.com/h2s-the-ultimate-single-nozzle-3d-printer-now-bigger-than-ever/",
    },
    Profile {
        id: "bambu-h2d-single",
        brand: "Bambu Lab",
        name: "H2D · Einzeldüse",
        size: [325.0, 320.0, 325.0],
        note: "Rechteckiger Bereich einer Düse; für beide Düsen das Dual-Profil wählen.",
        source: "https://cdn1.bambulab.com/documentation/h2d/en/H2D_Laser_Full_Combo_20250305.pdf",
    },
    Profile {
        id: "bambu-h2d-dual",
        brand: "Bambu Lab",
        name: "H2D · Dual",
        size: [300.0, 320.0, 325.0],
        note: "Gemeinsamer Druckbereich beider Düsen.",
        source: "https://cdn1.bambulab.com/documentation/h2d/en/H2D_Laser_Full_Combo_20250305.pdf",
    },
    Profile {
        id: "bambu-h2d-pro-single",
        brand: "Bambu Lab",
        name: "H2D Pro · Einzeldüse",
        size: [325.0, 320.0, 325.0],
        note: "Rechteckiger Bereich einer Düse; für beide Düsen das Dual-Profil wählen.",
        source: "https://bambulab.com/ko-kr/h2d-pro/tech-specs",
    },
    Profile {
        id: "bambu-h2d-pro-dual",
        brand: "Bambu Lab",
        name: "H2D Pro · Dual",
        size: [300.0, 320.0, 325.0],
        note: "Gemeinsamer Druckbereich beider Düsen.",
        source: "https://bambulab.com/ko-kr/h2d-pro/tech-specs",
    },
    Profile {
        id: "bambu-h2c-single",
        brand: "Bambu Lab",
        name: "H2C · Einzeldüse",
        size: [305.0, 320.0, 325.0],
        note: "Verifizierter Einzeldüsenbereich 305 mm breit; für beide Düsen das Dual-Profil wählen.",
        source: "https://blog.bambulab.com/bambu-lab-h2c-where-multi-material-vortek-system-meets-engineering-precision/",
    },
    Profile {
        id: "bambu-h2c-dual",
        brand: "Bambu Lab",
        name: "H2C · Dual",
        size: [300.0, 320.0, 325.0],
        note: "Gemeinsamer Druckbereich beider Düsen.",
        source: "https://blog.bambulab.com/bambu-lab-h2c-where-multi-material-vortek-system-meets-engineering-precision/",
    },
    Profile {
        id: "bambu-x2d-main",
        brand: "Bambu Lab",
        name: "X2D · Hauptdüse",
        size: [256.0, 256.0, 260.0],
        note: "Nur linke Hauptdüse. Mit rechter Hilfsdüse ist der gemeinsame Bereich kleiner.",
        source: "https://blog.bambulab.com/two-extruders-one-purpose-what-is-x2d-direct-drive-extrusion-and-auxiliary-extrusion/",
    },
    Profile {
        id: "bambu-x2d-dual",
        brand: "Bambu Lab",
        name: "X2D · Dual / Hilfsdüse",
        size: [235.5, 256.0, 256.0],
        note: "Gemeinsamer Bereich mit rechter Hilfsdüse; gegenüber der Hauptdüse 4 mm weniger Höhe.",
        source: "https://blog.bambulab.com/two-extruders-one-purpose-what-is-x2d-direct-drive-extrusion-and-auxiliary-extrusion/",
    },
    Profile {
        id: "prusa-mk4s",
        brand: "Prusa",
        name: "MK4S",
        size: [250.0, 210.0, 220.0],
        note: NOMINAL,
        source: "https://www.prusa3d.com/product/original-prusa-mk4s-3d-printer/",
    },
    Profile {
        id: "prusa-core-one",
        brand: "Prusa",
        name: "CORE One",
        size: [250.0, 220.0, 270.0],
        note: "Standardgerät ohne INDX-Umbau; Slicer-Druckbereich beachten.",
        source: "https://help.prusa3d.com/article/faq-frequently-asked-questions_1932?product=xl",
    },
    Profile {
        id: "prusa-core-one-plus",
        brand: "Prusa",
        name: "CORE One+",
        size: [250.0, 220.0, 270.0],
        note: "Standardgerät ohne INDX-Umbau; Slicer-Druckbereich beachten.",
        source: "https://www.prusa3d.com/product/prusa-core-one/",
    },
    Profile {
        id: "prusa-mini-plus",
        brand: "Prusa",
        name: "MINI+",
        size: [180.0, 180.0, 180.0],
        note: NOMINAL,
        source: "https://www.prusa3d.com/en/product/original-prusa-mini-kit-6/",
    },
    Profile {
        id: "prusa-xl",
        brand: "Prusa",
        name: "XL",
        size: [360.0, 360.0, 360.0],
        note: NOMINAL,
        source: "https://help.prusa3d.com/article/faq-frequently-asked-questions_1932?product=xl",
    },
    Profile {
        id: "creality-ender-3-v3",
        brand: "Creality",
        name: "Ender-3 V3",
        size: [220.0, 220.0, 250.0],
        note: "Ender-3 V3; nicht V3 KE, SE oder Plus.",
        source: "https://www.creality.com/compare/compare-ender-3-v3-series",
    },
    Profile {
        id: "creality-k1c",
        brand: "Creality",
        name: "K1C",
        size: [220.0, 220.0, 250.0],
        note: NOMINAL,
        source: "https://www.creality.com/products/k1c-carbon-3d-printer",
    },
    Profile {
        id: "creality-k1-max",
        brand: "Creality",
        name: "K1 Max",
        size: [300.0, 300.0, 300.0],
        note: NOMINAL,
        source: "https://www.creality.com/products/creality-k1-max-3d-printer",
    },
    Profile {
        id: "elegoo-neptune-4",
        brand: "Elegoo",
        name: "Neptune 4",
        size: [225.0, 225.0, 265.0],
        note: NOMINAL,
        source: "https://www.elegoo.com/products/elegoo-neptune-4-fdm-3d-printer",
    },
    Profile {
        id: "elegoo-neptune-4-pro",
        brand: "Elegoo",
        name: "Neptune 4 Pro",
        size: [225.0, 225.0, 265.0],
        note: NOMINAL,
        source: "https://www.elegoo.com/products/elegoo-neptune-4-pro-fdm-3d-printer",
    },
    Profile {
        id: "elegoo-neptune-4-plus",
        brand: "Elegoo",
        name: "Neptune 4 Plus",
        size: [320.0, 320.0, 385.0],
        note: NOMINAL,
        source: "https://www.elegoo.com/products/neptune-4-plus-fdm-3d-printer",
    },
    Profile {
        id: "elegoo-neptune-4-max",
        brand: "Elegoo",
        name: "Neptune 4 Max",
        size: [420.0, 420.0, 480.0],
        note: NOMINAL,
        source: "https://us.elegoo.com/products/neptune-4-max-fdm-3d-printer",
    },
    Profile {
        id: "elegoo-centauri-carbon",
        brand: "Elegoo",
        name: "Centauri Carbon",
        size: [256.0, 256.0, 256.0],
        note: NOMINAL,
        source: "https://www.elegoo.com/pages/elegoo-centauri-carbon",
    },
    Profile {
        id: "anycubic-kobra-3",
        brand: "Anycubic",
        name: "Kobra 3",
        size: [250.0, 250.0, 260.0],
        note: "Original-Kobra-3-Bauraum; bei geändertem Firmware-/Slicerprofil Benutzerdefiniert nutzen.",
        source: "https://store.anycubic.com/products/anycubic-kobra-3-1",
    },
    Profile {
        id: "anycubic-kobra-3-v2",
        brand: "Anycubic",
        name: "Kobra 3 V2",
        size: [255.0, 255.0, 260.0],
        note: "Nennbauraum laut V2-Handbuch; installierter Slicer und Firmware können den Bereich begrenzen.",
        source: "https://wiki.anycubic.com/k3-v2/Anycubic%20Kobra%203%20V2%20combo%20User%20Manual-EN-V1.0.pdf",
    },
    Profile {
        id: "anycubic-kobra-s1",
        brand: "Anycubic",
        name: "Kobra S1",
        size: [250.0, 250.0, 250.0],
        note: NOMINAL,
        source: "https://wiki.anycubic.com/kobra-s1/anycubic_kobra_s1_combo_user_manual-en-v1.3.pdf",
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn profiles_have_unique_ids_valid_dimensions_and_primary_sources() {
        let mut ids = HashSet::new();
        let mut names = HashSet::new();
        for profile in PROFILES {
            assert!(ids.insert(profile.id), "Doppelte Profil-ID: {}", profile.id);
            assert!(
                names.insert((profile.brand, profile.name)),
                "Doppeltes Profil: {}",
                profile.name
            );
            assert!(!profile.id.is_empty());
            assert!(
                profile
                    .id
                    .bytes()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
            );
            assert!(
                !profile.brand.is_empty() && !profile.name.is_empty() && !profile.note.is_empty()
            );
            assert!(
                profile
                    .size
                    .iter()
                    .all(|v| v.is_finite() && *v > 0.0 && *v <= 2000.0),
                "Ungültiger Bauraum: {}",
                profile.id
            );
            let host = profile
                .source
                .strip_prefix("https://")
                .unwrap()
                .split('/')
                .next()
                .unwrap();
            assert!(
                [
                    "cdn1.bambulab.com",
                    "blog.bambulab.com",
                    "us.store.bambulab.com",
                    "bambulab.com",
                    "raw.githubusercontent.com",
                    "www.prusa3d.com",
                    "help.prusa3d.com",
                    "www.creality.com",
                    "www.elegoo.com",
                    "us.elegoo.com",
                    "store.anycubic.com",
                    "wiki.anycubic.com",
                ]
                .contains(&host),
                "Quelle außerhalb der geprüften Herstellerdomains: {}",
                profile.source
            );
            if host == "raw.githubusercontent.com" {
                assert!(
                    profile
                        .source
                        .starts_with("https://raw.githubusercontent.com/bambulab/BambuStudio/")
                );
            }
        }
    }

    #[test]
    fn mode_profiles_preserve_individual_and_shared_print_areas() {
        let size = |id: &str| {
            PROFILES
                .iter()
                .find(|profile| profile.id == id)
                .unwrap()
                .size
        };
        for stem in ["bambu-h2d", "bambu-h2d-pro"] {
            assert_eq!(size(&format!("{stem}-single")), [325.0, 320.0, 325.0]);
            assert_eq!(size(&format!("{stem}-dual")), [300.0, 320.0, 325.0]);
        }
        assert_eq!(size("bambu-h2c-single"), [305.0, 320.0, 325.0]);
        assert_eq!(size("bambu-h2c-dual"), [300.0, 320.0, 325.0]);
        assert_eq!(size("bambu-x2d-main"), [256.0, 256.0, 260.0]);
        assert_eq!(size("bambu-x2d-dual"), [235.5, 256.0, 256.0]);
        for id in [
            "bambu-p1p",
            "bambu-p1s",
            "bambu-x1",
            "bambu-x1-carbon",
            "bambu-x1e",
        ] {
            assert_eq!(size(id), [256.0, 256.0, 250.0]);
        }
    }
}
