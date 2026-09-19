//! Command-line options (used by desktop shortcuts and forwarded by the single-instance plugin):
//!   --profile "<name>"   make this profile active
//!   --launch             start the game right away
//!   --minimized          start hidden in the tray
//! plus creating a desktop shortcut that carries them.

use serde::Serialize;

#[derive(Serialize, Clone, Debug, Default, PartialEq)]
pub struct StartupArgs {
    pub profile: Option<String>,
    pub launch: bool,
    pub minimized: bool,
}

/// Parse an argv (first element = exe path is skipped). Unknown arguments are ignored.
pub fn parse<I: IntoIterator<Item = String>>(argv: I) -> StartupArgs {
    let mut out = StartupArgs::default();
    let mut it = argv.into_iter().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--launch" => out.launch = true,
            "--minimized" => out.minimized = true,
            "--profile" => out.profile = it.next().filter(|s| !s.is_empty()),
            other => {
                if let Some(v) = other.strip_prefix("--profile=") {
                    out.profile = Some(v.to_string()).filter(|s| !s.is_empty());
                }
            }
        }
    }
    out
}

/// Create `Desktop\TK3K - <profile>.lnk` pointing at this exe with `--profile "<name>" --launch`.
pub fn create_profile_shortcut(profile: &str) -> Result<String, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let desktop = directories::UserDirs::new()
        .and_then(|u| u.desktop_dir().map(|d| d.to_path_buf()))
        .ok_or("desktop folder not found")?;
    let safe: String = profile.chars().map(|c| if r#"\/:*?"<>|"#.contains(c) { '_' } else { c }).collect();
    let lnk = desktop.join(format!("TK3K - {safe}.lnk"));
    let mut link = mslnk::ShellLink::new(&exe).map_err(|e| format!("shortcut: {e}"))?;
    link.set_arguments(Some(format!("--profile \"{}\" --launch", profile.replace('"', ""))));
    link.set_name(Some(format!("Play Three Kingdoms with the {profile} profile")));
    if let Some(dir) = exe.parent() {
        link.set_working_dir(Some(dir.to_string_lossy().into_owned()));
    }
    link.create_lnk(&lnk).map_err(|e| format!("shortcut: {e}"))?;
    Ok(lnk.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(args: &[&str]) -> StartupArgs {
        parse(args.iter().map(|s| s.to_string()))
    }

    #[test]
    fn parses_flags() {
        assert_eq!(p(&["app.exe"]), StartupArgs::default());
        assert_eq!(
            p(&["app.exe", "--profile", "190E MP", "--launch"]),
            StartupArgs { profile: Some("190E MP".into()), launch: true, minimized: false }
        );
        assert_eq!(p(&["app.exe", "--profile=Vanilla+", "--minimized", "--bogus"]).profile.as_deref(), Some("Vanilla+"));
        assert!(p(&["app.exe", "--minimized"]).minimized);
        assert_eq!(p(&["app.exe", "--profile"]).profile, None);
    }
}
