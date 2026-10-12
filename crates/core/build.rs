// Compile every translation in locales/<language>/*.ftl into the library. Adding a language is
// adding a folder: this lists the folders at build time, so no code change is needed.
use std::fmt::Write as _;
use std::path::Path;

fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("locales");
    println!("cargo:rerun-if-changed={}", root.display());
    let mut langs: Vec<_> = std::fs::read_dir(&root)
        .map(|rd| rd.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect())
        .unwrap_or_default();
    langs.sort();
    let mut out = String::from("pub(crate) static LOCALES: &[(&str, &[(&str, &str)])] = &[\n");
    for dir in langs {
        let code = dir.file_name().unwrap().to_string_lossy().into_owned();
        let mut files: Vec<_> = std::fs::read_dir(&dir)
            .map(|rd| rd.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "ftl")).collect())
            .unwrap_or_default();
        files.sort();
        let _ = write!(out, "    ({code:?}, &[");
        for f in files {
            println!("cargo:rerun-if-changed={}", f.display());
            let name = f.file_name().unwrap().to_string_lossy().into_owned();
            let _ = write!(out, "({name:?}, include_str!({:?})), ", f.display().to_string());
        }
        out.push_str("]),\n");
    }
    out.push_str("];\n");
    let dest = Path::new(&std::env::var("OUT_DIR").unwrap()).join("locales.rs");
    std::fs::write(dest, out).unwrap();
}
