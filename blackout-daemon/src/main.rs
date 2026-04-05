mod daemon;
mod handle;

use crate::daemon::Daemon;
use blackout_core::storage::Wallet;

use std::sync::Arc;

#[tokio::main]
async fn main() {

    let daemon_path = "/tmp/blackout.sock";

    let daemon = Daemon::new(Arc::new(Wallet::init()));

    println!("Iniciando o daemon...");
    println!("Socket path: {}", daemon_path);

    daemon.run(daemon_path).await.unwrap();
}
