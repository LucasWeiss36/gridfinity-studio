use std::{env, fs, path::PathBuf, process::Command};

fn resource_compiler() -> PathBuf {
    if let Some(path) = env::var_os("PATH")
        .into_iter()
        .flat_map(|paths| env::split_paths(&paths).collect::<Vec<_>>())
        .map(|dir| dir.join("rc.exe"))
        .find(|path| path.is_file())
    {
        return path;
    }
    let mut roots = Vec::new();
    if let Some(root) = env::var_os("WindowsSdkDir") {
        roots.push(PathBuf::from(root));
    }
    if let Some(root) = env::var_os("ProgramFiles(x86)") {
        roots.push(PathBuf::from(root).join("Windows Kits/10"));
    }
    for root in roots {
        let bin = root.join("bin");
        let mut versions: Vec<_> = fs::read_dir(&bin)
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .filter_map(|entry| {
                let name = entry.file_name();
                let version: Option<Vec<u32>> = name
                    .to_string_lossy()
                    .split('.')
                    .map(|part| part.parse().ok())
                    .collect();
                version.map(|version| (version, entry.path().join("x64/rc.exe")))
            })
            .filter(|(_, path)| path.is_file())
            .collect();
        versions.sort_by(|a, b| b.0.cmp(&a.0));
        if let Some((_, path)) = versions.into_iter().next() {
            return path;
        }
        let path = bin.join("x64/rc.exe");
        if path.is_file() {
            return path;
        }
    }
    panic!(
        "Windows SDK resource compiler rc.exe not found. Install the Windows SDK with Visual Studio C++ Build Tools."
    );
}

fn main() {
    println!("cargo:rerun-if-changed=assets/GridfinityStudio.ico");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows")
        || env::var("CARGO_CFG_TARGET_ENV").as_deref() != Ok("msvc")
    {
        return;
    }
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let icon = root
        .join("assets/GridfinityStudio.ico")
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('"', "\\\"");
    let version = env::var("CARGO_PKG_VERSION").unwrap();
    let numeric = format!("{},0", version.replace('.', ","));
    let source = format!(
        r#"#pragma code_page(65001)
101 ICON "{icon}"
1 VERSIONINFO
FILEVERSION {numeric}
PRODUCTVERSION {numeric}
FILEFLAGSMASK 0x3fL
FILEFLAGS 0
FILEOS 0x40004L
FILETYPE 1
FILESUBTYPE 0
BEGIN
  BLOCK "StringFileInfo"
  BEGIN
    BLOCK "040904B0"
    BEGIN
      VALUE "FileDescription", "GridfinityStudio\0"
      VALUE "FileVersion", "{version}\0"
      VALUE "InternalName", "GridfinityStudio\0"
      VALUE "OriginalFilename", "GridfinityStudio.exe\0"
      VALUE "ProductName", "GridfinityStudio\0"
      VALUE "ProductVersion", "{version}\0"
    END
  END
  BLOCK "VarFileInfo"
  BEGIN
    VALUE "Translation", 0x0409, 1200
  END
END
"#
    );
    let script = out.join("branding.rc");
    let resource = out.join("branding.res");
    fs::write(&script, source).expect("Could not write Windows resources");
    let result = Command::new(resource_compiler())
        .args(["/nologo", "/fo"])
        .arg(&resource)
        .arg(&script)
        .output()
        .expect("Could not start Windows resource compiler");
    assert!(
        result.status.success(),
        "Windows resources failed: {} {}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    println!(
        "cargo:rustc-link-arg-bin=gridfinitystudio={}",
        resource.display()
    );
}
