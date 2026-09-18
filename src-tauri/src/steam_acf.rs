//! Steam's per-app Workshop manifest (`steamapps\workshop\appworkshop_779340.acf`).
//!
//! `WorkshopItemDetails.<id>` records, per subscribed item, `timeupdated` (the version installed
//! on disk) and `latest_timeupdated` (the newest version Steam knows of). When latest > installed,
//! Steam has an update it has not downloaded yet — detectable offline.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AcfItem {
    pub installed: Option<u64>,
    pub latest: Option<u64>,
}

/// `...\steamapps\workshop\content\779340` → `...\steamapps\workshop\appworkshop_779340.acf`.
pub fn acf_path(workshop_content_dir: &Path) -> Option<PathBuf> {
    let app_id = workshop_content_dir.file_name()?.to_string_lossy().into_owned();
    let workshop = workshop_content_dir.parent()?.parent()?;
    Some(workshop.join(format!("appworkshop_{app_id}.acf")))
}

pub fn workshop_items(workshop_content_dir: &Path) -> HashMap<String, AcfItem> {
    acf_path(workshop_content_dir)
        .and_then(|p| std::fs::read_to_string(p).ok())
        .map(|t| parse(&t))
        .unwrap_or_default()
}

/// Split a VDF line into its quoted tokens (`"key"  "value"` → [key, value]).
fn tokens(line: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut rest = line;
    while let Some(start) = rest.find('"') {
        let after = &rest[start + 1..];
        let Some(end) = after.find('"') else { break };
        out.push(&after[..end]);
        rest = &after[end + 1..];
    }
    out
}

/// Line-based VDF walk, tracking the block path (e.g. AppWorkshop / WorkshopItemDetails / <id>).
pub fn parse(text: &str) -> HashMap<String, AcfItem> {
    let mut items: HashMap<String, AcfItem> = HashMap::new();
    let mut stack: Vec<String> = Vec::new();
    let mut pending: Option<String> = None;
    for raw in text.lines() {
        let line = raw.trim();
        if line == "{" {
            if let Some(name) = pending.take() {
                stack.push(name);
            }
            continue;
        }
        if line == "}" {
            stack.pop();
            continue;
        }
        let t = tokens(line);
        match t.as_slice() {
            [name] => pending = Some(name.to_string()),
            [key, value] => {
                // Both blocks carry per-item data; Details is authoritative for "latest".
                let in_items = stack.len() == 3 && (stack[1] == "WorkshopItemDetails" || stack[1] == "WorkshopItemsInstalled");
                if in_items {
                    let id = stack[2].clone();
                    let n = value.parse::<u64>().ok().filter(|v| *v > 0);
                    let item = items.entry(id).or_default();
                    match *key {
                        "timeupdated" => item.installed = item.installed.or(n),
                        "latest_timeupdated" => item.latest = n.or(item.latest),
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
    items
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#""AppWorkshop"
{
	"appid"		"779340"
	"WorkshopItemsInstalled"
	{
		"111"
		{
			"size"		"10"
			"timeupdated"		"1000"
			"manifest"		"1"
		}
		"222"
		{
			"size"		"10"
			"timeupdated"		"2000"
		}
	}
	"WorkshopItemDetails"
	{
		"111"
		{
			"manifest"		"1"
			"timeupdated"		"1000"
			"timetouched"		"5"
			"latest_timeupdated"		"1500"
		}
		"222"
		{
			"timeupdated"		"2000"
			"latest_timeupdated"		"2000"
		}
	}
}
"#;

    #[test]
    fn reads_installed_and_latest() {
        let m = parse(SAMPLE);
        assert_eq!(m["111"], AcfItem { installed: Some(1000), latest: Some(1500) });
        assert_eq!(m["222"], AcfItem { installed: Some(2000), latest: Some(2000) });
        assert!(!m.contains_key("779340"));
    }

    #[test]
    fn acf_path_from_content_dir() {
        let p = acf_path(Path::new(r"C:\Steam\steamapps\workshop\content\779340")).unwrap();
        assert_eq!(p, PathBuf::from(r"C:\Steam\steamapps\workshop\appworkshop_779340.acf"));
    }
}
