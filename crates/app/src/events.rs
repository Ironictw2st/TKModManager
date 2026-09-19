//! Worker threads never touch Qt. They send `UiEvent`s over a channel that the GUI thread drains
//! from a short `QTimer` (see `App::pump`).

use std::collections::HashMap;
use std::sync::mpsc::{channel, Receiver, Sender};
use tkmm_core::cli::StartupArgs;
use tkmm_core::conflicts::ConflictReport;
use tkmm_core::dll::{CatalogEntry, DllStatus, InstalledDll, RemoteDll};
use tkmm_core::hash::{PackHash, Progress};
use tkmm_core::launch::{LaunchStatus, SaveGame};
use tkmm_core::packs::ModEntry;
use tkmm_core::paths::{GameInstall, GamePaths};
use tkmm_core::se_scan::SeInfo;
use tkmm_core::update::UpdateMeta;
use tkmm_core::workshop::{CollectionResult, WorkshopItem};

/// What a hashing run was for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HashPurpose {
    Export,
    Verify,
}

pub enum UiEvent {
    Scanned { paths: GamePaths, mods: Vec<ModEntry>, installs: Vec<GameInstall> },
    Workshop(HashMap<String, WorkshopItem>),
    SeScan(HashMap<String, SeInfo>),
    Dll(DllStatus),
    /// `true` when it came from the startup check (offer the install in a popup).
    DllRemote(Result<RemoteDll, String>, bool),
    DllCatalog(Result<Vec<CatalogEntry>, String>),
    DllProgress(f64),
    DllInstalled(Result<InstalledDll, String>),
    Launch(LaunchStatus),
    Saves(Vec<SaveGame>),
    HashProgress(Progress),
    Hashed(HashPurpose, Result<Vec<PackHash>, String>),
    Conflicts(ConflictReport),
    Collection(Result<CollectionResult, String>),
    AppUpdate(Result<Option<UpdateMeta>, String>, bool),
    UpdateProgress(f64),
    UpdateInstalled(Result<(), String>),
    Cli(StartupArgs),
}

#[derive(Clone)]
pub struct Bus(Sender<UiEvent>);

impl Bus {
    pub fn new() -> (Bus, Receiver<UiEvent>) {
        let (tx, rx) = channel();
        (Bus(tx), rx)
    }

    pub fn send(&self, e: UiEvent) {
        let _ = self.0.send(e);
    }

    /// Run `f` on a new thread and deliver its event.
    pub fn spawn(&self, f: impl FnOnce(&Bus) -> Option<UiEvent> + Send + 'static) {
        let bus = self.clone();
        std::thread::spawn(move || {
            if let Some(e) = f(&bus) {
                bus.send(e);
            }
        });
    }
}
