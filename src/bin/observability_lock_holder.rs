use std::fs::OpenOptions;
use std::io::{self, Read, Write};
use std::os::fd::AsRawFd;
use std::path::Path;

use clap::Parser;

#[derive(Debug, Parser)]
#[command(
    author,
    version,
    about = "Hold the observability service lock for tests and ops probes"
)]
struct Cli {
    #[arg(long, env = "PROXY_DB_PATH", default_value = "data/tavily_proxy.db")]
    db_path: String,
}

fn sqlite_sidecar_path(database_path: &str, file_name: &str) -> String {
    let path = Path::new(database_path);
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("sqlite");
    let sidecar_name = if let Some((base, ext)) = file_name.rsplit_once('.') {
        format!("{stem}-{base}.{ext}")
    } else {
        format!("{stem}-{file_name}")
    };
    parent.join(sidecar_name).to_string_lossy().to_string()
}

fn acquire_shared_lock(database_path: &str) -> Result<std::fs::File, io::Error> {
    let lock_path = sqlite_sidecar_path(database_path, "observability-migrate.lock");
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(lock_path)?;
    let rc = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_SH | libc::LOCK_NB) };
    if rc == 0 {
        Ok(file)
    } else {
        Err(io::Error::last_os_error())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let _lock = acquire_shared_lock(&cli.db_path)?;
    writeln!(io::stdout().lock(), "lock-held")?;
    io::stdout().lock().flush()?;
    let mut sink = String::new();
    let _ = io::stdin().read_to_string(&mut sink)?;
    Ok(())
}
