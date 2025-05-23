use log::{debug, error};
use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use sui_sdk::types::base_types::SuiAddress;
use tokio::process::{Child, Command};
use tokio::time::sleep;

pub struct LocalnetProcess {
    process: Child,
    running: Arc<AtomicBool>,
}
impl LocalnetProcess {
    pub async fn new() -> Result<Self, anyhow::Error> {
        let process = Command::new("sui")
            .args(["start", "--with-faucet", "--force-regenesis"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()?;

        let running = Arc::new(AtomicBool::new(true));

        Ok(LocalnetProcess { process, running })
    }

    pub async fn register_local_env(&self) -> Result<(), anyhow::Error> {
        Command::new("sui")
            .args([
                "client",
                "new-env",
                "--alias",
                "local",
                "rpc",
                "http://127.0.0.1:9000",
            ])
            .kill_on_drop(true)
            .output()
            .await?;
        Ok(())
    }

    pub async fn use_local_env(&self) -> Result<(), anyhow::Error> {
        Command::new("sui")
            .args(["client", "switch", "--env", "local"])
            .kill_on_drop(true)
            .output()
            .await?;
        Ok(())
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    pub async fn kill(&mut self) -> Result<(), anyhow::Error> {
        if self.running.load(Ordering::SeqCst) {
            debug!("Stopping server...");
            self.running.store(false, Ordering::SeqCst);

            match self.process.kill().await {
                Ok(_) => {
                    self.process.wait().await?;
                    debug!("Server stopped");
                    Ok(())
                }
                Err(e) => Err(anyhow::anyhow!("Failed to stop server: {}", e)),
            }
        } else {
            error!("Server already stopped");
            Ok(())
        }
    }
}
