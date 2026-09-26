// Embed the icon and version info, so the exe says what it is.
fn main() {
    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("../app/icons/icon.ico");
        res.set("ProductName", "TK Mod Manager");
        res.set("FileDescription", "TK Mod Manager script-extender loader");
        res.set("CompanyName", "Ironictw2st");
        res.set("LegalCopyright", "MIT License, Ironictw2st");
        res.set("OriginalFilename", "tkmm-inject.exe");
        res.set("InternalName", "tkmm-inject");
        res.set("Comments", "Loads script_extender.dll into Three_Kingdoms.exe for TK Mod Manager. https://github.com/Ironictw2st/TKModManager");
        if let Err(e) = res.compile() {
            println!("cargo:warning=icon resource not embedded: {e}");
        }
    }
}
