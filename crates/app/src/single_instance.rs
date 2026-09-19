//! One running copy: the first instance listens on a loopback port; a second instance (e.g. a
//! desktop shortcut) sends its command line there and exits.

use crate::events::{Bus, UiEvent};
use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::time::Duration;
use tkmm_core::cli::{self, StartupArgs};

const PORT: u16 = 47_811;
const MAGIC: &str = "TKMM1";

/// Ok(listener) when we are the first instance; Err(()) after forwarding `argv` to the running one.
pub fn acquire(argv: &[String]) -> Result<TcpListener, ()> {
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, PORT));
    match TcpListener::bind(addr) {
        Ok(l) => Ok(l),
        Err(_) => {
            if let Ok(mut s) = TcpStream::connect_timeout(&addr, Duration::from_millis(800)) {
                let payload = format!("{MAGIC}\n{}", serde_json::to_string(argv).unwrap_or_default());
                let _ = s.write_all(payload.as_bytes());
                return Err(());
            }
            // Port taken by something else: run without single-instance support.
            TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, 0))).map_err(|_| ())
        }
    }
}

/// Accept forwarded command lines for the life of the app.
pub fn listen(listener: TcpListener, bus: Bus) {
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let mut stream = stream;
            let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
            let mut text = String::new();
            if stream.read_to_string(&mut text).is_err() {
                continue;
            }
            let Some(rest) = text.strip_prefix(MAGIC) else { continue };
            let argv: Vec<String> = serde_json::from_str(rest.trim()).unwrap_or_default();
            let args: StartupArgs = cli::parse(argv);
            bus.send(UiEvent::Cli(args));
        }
    });
}
