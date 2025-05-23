mod drive;
mod monster;
mod registry;

use crate::cli::admin::drive::DriveCommands;
use crate::cli::admin::monster::MonsterCommands;
use crate::cli::admin::registry::RegistryCommands;
use crate::sui_client::Client;
use clap::Subcommand;

#[derive(Subcommand)]
pub enum AdminCommands {
    #[command(about = "MonsterPartRegistry admin commands", long_about = None)]
    Registry {
        #[command(subcommand)]
        command: RegistryCommands,
    },
    #[command(about = "EncryptedDriveMinter commands", long_about = None)]
    Drive {
        #[command(subcommand)]
        command: DriveCommands,
    },
    #[command(about = "MonsterMinter commands", long_about = None)]
    Monster {
        #[command(subcommand)]
        command: MonsterCommands,
    },
}

impl AdminCommands {
    pub async fn execute(self, client: &mut Client<'_>) -> Result<(), anyhow::Error> {
        match self {
            AdminCommands::Registry { command } => {
                command.execute(client).await?;
            }
            AdminCommands::Drive { command } => {
                command.execute(client).await?;
            }
            AdminCommands::Monster { command } => {
                command.execute(client).await?;
            }
        }

        Ok(())
    }
}
