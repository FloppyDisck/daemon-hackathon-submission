mod admin;
mod query;
mod wallet;

use crate::localnet_process::LocalnetProcess;
use crate::network_config::NetworkConfigs;
use crate::sui_client::Client;
use clap::{Parser, Subcommand, ValueEnum};
use log::{debug, info};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use sui_types::base_types::SuiAddress;
use tokio::process::Command;
use tokio::time::sleep;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Default)]
pub enum NetType {
    Localnet,
    #[default]
    Testnet,
    Mainnet,
}

#[derive(Parser)]
#[command(name = "DaemonCli")]
#[command(version = "1.0")]
#[command(about = "Testing and management interface for the Daemon smart contracts", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(short, long)]
    net: Option<NetType>,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Smart contract automated deployment", long_about = None)]
    Deploy {
        #[arg(short, long)]
        script: Option<String>,
    },
    #[command(about = "Admin commands for managing the contracts", long_about = None)]
    Admin {
        #[command(subcommand)]
        command: admin::AdminCommands,
    },
    #[command(about = "Query commands", long_about = None)]
    Query {
        #[command(subcommand)]
        command: query::QueryCommands,
    },
    #[command(about = "Key management commands", long_about = None)]
    Wallet {
        #[command(subcommand)]
        command: wallet::WalletCommands,
    },
    #[command(about = "Fund address with localnet Sui", long_about = None)]
    Faucet { address: String },
}

impl Cli {
    pub async fn execute(self, config: &mut NetworkConfigs) -> Result<(), anyhow::Error> {
        let network = self.net.unwrap_or_default();
        let network_config = match network {
            NetType::Localnet => &mut config.localnet,
            NetType::Testnet => &mut config.testnet,
            NetType::Mainnet => &mut config.mainnet,
        };

        // Deploy the local network
        let mut localnet = None;
        if let Commands::Deploy { script: _ } = self.command {
            if network == NetType::Localnet {
                debug!("Initializing localnet");
                let net = LocalnetProcess::new().await?;
                net.register_local_env().await?;
                net.use_local_env().await?;

                localnet = Some(net);

                tokio::time::sleep(Duration::from_secs(5)).await;
                debug!("Localnet is ready");
            }
        }

        let mut client = Client::new(network, network_config).await?;

        match self.command {
            Commands::Wallet { command } => {
                command.execute(&mut client).await?;
            }
            Commands::Query { command } => command.execute(&client).await?,
            Commands::Admin { command } => command.execute(&mut client).await?,
            Commands::Deploy { script } => {
                if network == NetType::Localnet {
                    if !client.wallet_exists(crate::sui_client::ADMIN)? {
                        client.generate_wallet(crate::sui_client::ADMIN)?;
                    }

                    for key in client.config.keys.iter() {
                        let key = client.load_wallet(key)?;
                        faucet(&SuiAddress::from(&key.public()).to_string()).await?;
                    }
                }

                client.publish_package(10).await?;
                client.init_registry().await?;

                client
                    .set_rarity_distribution(vec![1, 0, 0, 0, 0, 0])
                    .await?;

                info!(
                    "\n\
                Admin: {}\n\
                Package: {}\n\
                ",
                    client.sui_address("admin")?.to_string(),
                    client.deployment()?.admin_cap.to_string()
                );

                if let Some(script) = script {
                    debug!("Running script");
                    let script: InitScript = serde_json::from_reader(std::fs::File::open(script)?)?;

                    for mint in script.mint {
                        if let Some(drives) = mint.drives {
                            debug!("Minting {} drives", drives);
                            client.mint_drives(&mint.name, drives).await?;
                        }

                        if let Some(monsters) = mint.monsters {
                            debug!("Minting {} monsters", monsters);
                            let drives = client.mint_drives(&mint.name, monsters).await?;
                            client.mint_monsters(drives).await?;
                        }
                    }
                }

                info!("Updating Config");
                config.update()?;
            }
            Commands::Faucet { address } => {
                faucet(&address).await?;
            }
        }

        // This is just to trick the compiler to not drop the localnet process
        if let Some(mut localnet) = localnet {
            info!("Server running!");
            loop {
                if !localnet.is_running() {
                    localnet.kill().await?;
                }
            }
        }

        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
pub struct InitScript {
    pub mint: Vec<ScriptMint>,
}

#[derive(Serialize, Deserialize)]
pub struct ScriptMint {
    pub name: String,
    pub monsters: Option<usize>,
    pub drives: Option<usize>,
}

async fn faucet(address: &str) -> Result<(), anyhow::Error> {
    debug!("Faucet: {:?}", address.to_string());
    let res = Command::new("sui")
        .args(["client", "faucet", "--address", &address.to_string()])
        .kill_on_drop(true)
        .output()
        .await?;
    println!("{:?}", res);
    sleep(Duration::from_secs(1)).await;
    Ok(())
}
