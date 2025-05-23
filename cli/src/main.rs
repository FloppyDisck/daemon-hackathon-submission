mod cli;
mod localnet_process;
mod network_config;
// mod old_cli;
mod packages;
mod sui_client;

use crate::cli::Cli;
use crate::network_config::NetworkConfigs;
use clap::Parser;
use log::LevelFilter;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    env_logger::builder()
        .filter_module("daemon_cli", LevelFilter::Debug)
        .init();

    let mut config = NetworkConfigs::load()?;

    let args = Cli::parse();

    args.execute(&mut config).await?;

    config.update()?;

    Ok(())
}
