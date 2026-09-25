// Embed the app icon and version info into the Windows executable, and put Valve's
// steam_api64.dll (used by "Force update", loaded at run time) next to it for dev runs.
fn main() {
    println!("cargo:rerun-if-changed=../../redist/steam_api64.dll");
    // Compiled in with option_env! for "Send report" (set by scripts/build.ps1 and release.ps1).
    println!("cargo:rerun-if-env-changed=TKMM_REPORT_WEBHOOK");
    if let Ok(out) = std::env::var("OUT_DIR") {
        // OUT_DIR = target/<profile>/build/tkmm-<hash>/out
        if let Some(profile_dir) = std::path::Path::new(&out).ancestors().nth(3) {
            let _ = std::fs::copy("../../redist/steam_api64.dll", profile_dir.join("steam_api64.dll"));
        }
    }
    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("icons/icon.ico");
        res.set("ProductName", "TK Mod Manager");
        res.set("FileDescription", "TK Mod Manager");
        if let Err(e) = res.compile() {
            println!("cargo:warning=icon resource not embedded: {e}");
        }
    }
}
