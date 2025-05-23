use crate::sui_client::Client;
use clap::Subcommand;
use log::info;

#[derive(Subcommand)]
pub enum MonsterCommands {
    #[command(about = "Mint a monster", long_about = None)]
    Mint {},
    #[command(about = "Enable/Disable monster minter", long_about = None)]
    SetEnabled { enabled: bool },
}

impl MonsterCommands {
    pub async fn execute(self, client: &mut Client<'_>) -> Result<(), anyhow::Error> {
        match self {
            MonsterCommands::Mint {} => {
                let drive = client.admin_mint_drive(None).await?;
                let monster = client.mint_monster(drive).await?;
                info!("Minted monster: {}", monster.id);
            }
            MonsterCommands::SetEnabled { enabled } => {
                client
                    .monster_minter()?
                    .set_enabled(&client.admin_cap()?, &client.load_wallet("admin")?, enabled)
                    .await?;
            }
        }

        Ok(())
    }
}
