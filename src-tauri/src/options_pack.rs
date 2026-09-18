//! Launch-time options implemented as a generated mod pack (the technique Runcher and WH3 Mod
//! Manager use: there are no engine flags for these). The pack is named `zzzz_…` so it sorts
//! last, lives in our own folder (added as a working directory), and is rebuilt every launch.

use crate::empty_vp8::EMPTY_CA_VP8;
use crate::paths;
use rpfm_lib::files::pack::Pack;
use rpfm_lib::files::{Container, FileType, RFile};
use rpfm_lib::games::pfh_file_type::PFHFileType;
use rpfm_lib::games::pfh_version::PFHVersion;
use rpfm_lib::games::supported_games::{SupportedGames, KEY_THREE_KINGDOMS};
use std::path::PathBuf;

pub const PACK_NAME: &str = "zzzz_tkmm_options.pack";

/// The two startup videos Three Kingdoms plays before the menu.
const INTRO_MOVIES: [&str; 2] = ["movies/startup_movie_01.ca_vp8", "movies/startup_movie_02.ca_vp8"];

pub fn options_dir() -> PathBuf {
    paths::app_data_dir().join("options")
}

/// Rebuild the options pack for the requested features and return its folder + file name.
/// The folder is emptied first so a stale pack from an earlier launch can never linger.
pub fn build(skip_intro: bool) -> Result<(PathBuf, String), String> {
    let dir = options_dir();
    if dir.is_dir() {
        std::fs::remove_dir_all(&dir).map_err(|e| format!("clear {}: {e}", dir.display()))?;
    }
    std::fs::create_dir_all(&dir).map_err(|e| format!("create {}: {e}", dir.display()))?;

    let games = SupportedGames::default();
    let game = games.game(KEY_THREE_KINGDOMS).ok_or("rpfm: unknown game key")?;
    let mut pack = Pack::new_with_name_and_version(PACK_NAME, PFHVersion::PFH5);
    pack.set_pfh_file_type(PFHFileType::Mod);
    if skip_intro {
        for path in INTRO_MOVIES {
            let file = RFile::new_from_vec(&EMPTY_CA_VP8, FileType::Video, 0, path);
            pack.insert(file).map_err(|e| format!("insert {path}: {e}"))?;
        }
    }
    let out = dir.join(PACK_NAME);
    pack.save(Some(&out), game, &None).map_err(|e| format!("save {}: {e}", out.display()))?;
    Ok((dir, PACK_NAME.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_a_mod_pack_with_two_movies() {
        let (dir, name) = build(true).expect("build options pack");
        let path = dir.join(&name);
        assert_eq!(crate::packs::read_pack_type(&path).unwrap(), crate::packs::PackType::Mod);
        let games = SupportedGames::default();
        let game = games.game(KEY_THREE_KINGDOMS).unwrap();
        let pack = Pack::read_and_merge(&[path], game, true, false, false).unwrap();
        let mut paths = pack.paths_raw();
        paths.sort();
        assert_eq!(paths, INTRO_MOVIES.to_vec());
    }
}
