//! The mod list file the game reads (`tkmm_mods.txt`), and a parser for CA's `used_mods.txt`.
//!
//! Directives (all semicolon-terminated, forward slashes in paths):
//!   add_working_directory "<abs dir>";   make the folder's packs visible (movie packs in it load)
//!   mod "<file>.pack";                   load this mod pack, in list order (later overrides)
//!   exclude_pack_file "<file>.pack";     keep a data/ movie pack from auto-loading

use serde::Serialize;

/// One enabled pack, already resolved against the scan.
#[derive(Clone, Debug)]
pub struct ListMod {
    pub file: String,
    /// The Workshop item folder (None for packs in data/).
    pub dir: Option<String>,
    pub is_movie: bool,
}

#[derive(Clone, Debug, Default)]
pub struct ListInput {
    /// Enabled packs in profile order (movies may be mixed in; they get no `mod` line).
    pub enabled: Vec<ListMod>,
    /// Disabled movie packs that live in data/ (the engine would load them otherwise).
    pub excluded_data_movies: Vec<String>,
    /// Extra working directories to add last (e.g. the generated options pack folder).
    pub extra_dirs: Vec<String>,
    /// Extra `mod` lines to append after everything else (e.g. the options pack).
    pub extra_mods: Vec<String>,
}

fn slashes(p: &str) -> String {
    p.replace('\\', "/")
}

/// Render the list text. Folder lines first (deduped, first occurrence order), then `mod`
/// lines for mod-type packs in order, then exclusions, then extras.
pub fn build(input: &ListInput) -> String {
    let mut dirs: Vec<String> = Vec::new();
    let mut push_dir = |d: &str| {
        let d = slashes(d);
        if !dirs.iter().any(|x| x.eq_ignore_ascii_case(&d)) {
            dirs.push(d);
        }
    };
    for m in &input.enabled {
        if let Some(d) = &m.dir {
            push_dir(d);
        }
    }
    for d in &input.extra_dirs {
        push_dir(d);
    }
    let mut out = String::new();
    for d in &dirs {
        out.push_str(&format!("add_working_directory \"{d}\";\n"));
    }
    for m in input.enabled.iter().filter(|m| !m.is_movie) {
        out.push_str(&format!("mod \"{}\";\n", m.file));
    }
    for f in &input.excluded_data_movies {
        out.push_str(&format!("exclude_pack_file \"{f}\";\n"));
    }
    for f in &input.extra_mods {
        out.push_str(&format!("mod \"{f}\";\n"));
    }
    out
}

/// Parsed contents of a CA-style list file.
#[derive(Serialize, Clone, Debug, Default, PartialEq)]
pub struct ParsedList {
    pub dirs: Vec<String>,
    pub mods: Vec<String>,
    pub excludes: Vec<String>,
}

/// Parse `used_mods.txt` / `tkmm_mods.txt`. Tolerant: ignores unknown lines and blank lines.
pub fn parse(text: &str) -> ParsedList {
    let mut out = ParsedList::default();
    for raw in text.lines() {
        let line = raw.trim().trim_end_matches(';').trim();
        let Some((cmd, rest)) = line.split_once(char::is_whitespace) else { continue };
        let value = rest.trim().trim_matches('\u{22}').to_string();
        if value.is_empty() {
            continue;
        }
        match cmd {
            "add_working_directory" => out.dirs.push(value),
            "mod" => out.mods.push(value),
            "exclude_pack_file" => out.excludes.push(value),
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(file: &str, dir: Option<&str>, movie: bool) -> ListMod {
        ListMod { file: file.into(), dir: dir.map(str::to_owned), is_movie: movie }
    }

    #[test]
    fn builds_ca_shaped_list() {
        let input = ListInput {
            enabled: vec![
                m("!!a.pack", Some(r"C:\ws\1"), false),
                m("b.pack", None, false),
                m("mv.pack", Some(r"C:\ws\2"), true),
                m("c.pack", Some(r"C:\ws\1"), false),
            ],
            excluded_data_movies: vec!["off.pack".into()],
            extra_dirs: vec![r"C:\opt".into()],
            extra_mods: vec!["zzzz_opt.pack".into()],
        };
        let text = build(&input);
        let expected = [
            "add_working_directory \"C:/ws/1\";",
            "add_working_directory \"C:/ws/2\";",
            "add_working_directory \"C:/opt\";",
            "mod \"!!a.pack\";",
            "mod \"b.pack\";",
            "mod \"c.pack\";",
            "exclude_pack_file \"off.pack\";",
            "mod \"zzzz_opt.pack\";",
            "",
        ]
        .join("\n");
        assert_eq!(text, expected);
    }

    #[test]
    fn parses_used_mods() {
        let text = "add_working_directory \"C:/x/1\";\r\nmod \"a.pack\";\n\nmod \"b c.pack\";\nexclude_pack_file \"m.pack\";\njunk\n";
        let p = parse(text);
        assert_eq!(p.dirs, vec!["C:/x/1"]);
        assert_eq!(p.mods, vec!["a.pack", "b c.pack"]);
        assert_eq!(p.excludes, vec!["m.pack"]);
    }

    #[test]
    fn roundtrip() {
        let input = ListInput {
            enabled: vec![m("a.pack", Some("C:/ws/1"), false), m("b.pack", None, false)],
            ..Default::default()
        };
        let p = parse(&build(&input));
        assert_eq!(p.dirs, vec!["C:/ws/1"]);
        assert_eq!(p.mods, vec!["a.pack", "b.pack"]);
    }
}
