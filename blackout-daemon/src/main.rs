mod config;
mod daemon;

use crate::config::Config;
use crate::daemon::Daemon;
use blackout_core::storage::Wallet;

use std::sync::Arc;

#[tokio::main]
async fn main() {
    // Inicializa o storage
    let daemon = Daemon::new(Arc::new(Wallet::init()), Config::default());

    daemon.run("/tmp/blackout.sock").await.unwrap();
}
