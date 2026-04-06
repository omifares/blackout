mod daemon;
mod handle;

use tracing::{info, warn, error};
use std::env;

use crate::daemon::Daemon;
use blackout_core::storage::Wallet;

use std::sync::Arc;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let args: Vec<String> = env::args().collect();
    let use_mlock = args.contains(&"--mlock".to_string());

    if use_mlock {
        prevent_memory_swapping()
            .expect("Panic: Cannot lock memory with mlockall. This is critical for security.");
    } else {
        warn!("Running without mlock. This may lead to sensitive data being swapped to disk. Use --mlock flag to improve security.");
    }

    let daemon = Daemon::new(Arc::new(Wallet::init()));

    info!("Iniciando o daemon...");

    daemon.run().await.unwrap();
}

fn prevent_memory_swapping() -> std::io::Result<()> {
    let flags = libc::MCL_CURRENT | libc::MCL_FUTURE;
    let result = unsafe { libc::mlockall(flags) };

    if result != 0 {
        let err = std::io::Error::last_os_error();
        error!("Failed syscall mlockall: {}", err);
        return Err(err); 
    }

    info!("mlockall successful: Memory is locked and will not be swapped to disk.");
    Ok(())
}