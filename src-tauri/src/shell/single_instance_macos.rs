use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::time::Duration;

use tauri::{AppHandle, Runtime};

const MAX_MESSAGE_BYTES: u64 = 8 * 1024;
const SOCKET_NAME: &str = "app.userhop.desktop.sock";

/// macOS gives every user a private `$TMPDIR`, so the lock never spans other users' sessions.
/// The official plugin uses a shared `/tmp` path on macOS, which would let users collide.
fn socket_path() -> Option<PathBuf> {
    let dir = PathBuf::from(std::env::var_os("TMPDIR")?);
    dir.is_absolute().then(|| dir.join(SOCKET_NAME))
}

fn encode(args: &[String]) -> Vec<u8> {
    args.iter()
        .skip(1)
        .flat_map(|arg| arg.as_bytes().iter().copied().chain(std::iter::once(0)))
        .collect()
}

fn decode(bytes: &[u8]) -> Vec<String> {
    std::iter::once(String::new())
        .chain(
            bytes
                .split(|&b| b == 0)
                .filter(|part| !part.is_empty())
                .map(|part| String::from_utf8_lossy(part).into_owned()),
        )
        .collect()
}

/// Returns `true` when another instance of this user received the arguments and this one should exit.
pub fn forward_to_running_instance() -> bool {
    let Some(path) = socket_path() else {
        return false;
    };
    let Ok(mut stream) = UnixStream::connect(&path) else {
        return false;
    };
    let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));
    let args: Vec<String> = std::env::args().collect();
    stream.write_all(&encode(&args)).is_ok()
}

pub fn listen<R: Runtime>(app: &AppHandle<R>, on_args: fn(&AppHandle<R>, &[String])) {
    let Some(path) = socket_path() else { return };
    let _ = std::fs::remove_file(&path);
    let listener = match UnixListener::bind(&path) {
        Ok(listener) => listener,
        Err(error) => {
            tracing::warn!(%error, "single instance socket unavailable");
            return;
        }
    };
    let app = app.clone();
    let spawned = std::thread::Builder::new()
        .name("single-instance".into())
        .spawn(move || {
            for stream in listener.incoming().flatten() {
                let mut bytes = Vec::new();
                let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
                if stream
                    .take(MAX_MESSAGE_BYTES)
                    .read_to_end(&mut bytes)
                    .is_err()
                {
                    continue;
                }
                let args = decode(&bytes);
                let handle = app.clone();
                let _ = app.run_on_main_thread(move || on_args(&handle, &args));
            }
        });
    if let Err(error) = spawned {
        tracing::warn!(%error, "single instance listener failed");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arguments_round_trip_without_the_program_path() {
        let args = vec![
            "/Applications/App".to_owned(),
            "--toggle".to_owned(),
            "é".to_owned(),
        ];
        assert_eq!(decode(&encode(&args)), vec!["", "--toggle", "é"]);
        assert_eq!(decode(&[]), vec![""]);
    }
}
