//! `tkmm-inject.exe --pid <pid> --dll <path>`: load the script extender into a running
//! Three_Kingdoms.exe (LoadLibraryW on a remote thread) and report the result as one JSON line
//! `{"ok": bool, "message": "..."}` on stdout. Exit code 0 on success.
//!
//! This is the only part of TK Mod Manager that writes into another process. It lives in its
//! own small exe so the manager itself does no process injection, and so an antivirus that
//! blocks it stops only the script extender, not the manager. The manager runs it only when a
//! profile uses the script extender.

// No console window: the manager reads stdout through a pipe.
#![windows_subsystem = "windows"]

use std::path::PathBuf;

const GAME_EXE: &str = "Three_Kingdoms.exe";

fn reply(ok: bool, message: &str) -> i32 {
    println!("{}", serde_json::json!({ "ok": ok, "message": message }));
    if ok { 0 } else { 1 }
}

fn run() -> i32 {
    let mut args = std::env::args().skip(1);
    let (mut pid, mut dll): (Option<u32>, Option<PathBuf>) = (None, None);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--pid" => pid = args.next().and_then(|v| v.parse().ok()),
            "--dll" => dll = args.next().map(PathBuf::from),
            _ => return reply(false, &format!("unknown argument {a}")),
        }
    }
    let (Some(pid), Some(dll)) = (pid, dll) else {
        return reply(false, "usage: tkmm-inject --pid <pid> --dll <path>");
    };
    // Only ever the game, and only a DLL that exists.
    if !inject_core::find_pids(GAME_EXE).contains(&pid) {
        return reply(false, &format!("process {pid} is not {GAME_EXE}"));
    }
    if !dll.is_file() {
        return reply(false, &format!("{} does not exist", dll.display()));
    }
    match inject_core::inject(pid, &dll) {
        Ok(_) => reply(true, "DLL loaded"),
        Err(e) => reply(false, &e),
    }
}

fn main() {
    std::process::exit(run());
}
