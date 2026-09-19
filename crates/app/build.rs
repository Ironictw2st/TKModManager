// Embed the app icon and version info into the Windows executable.
fn main() {
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
