use crate::sui_client::Client;
use clap::Subcommand;
use log::info;

#[derive(Subcommand)]
pub enum DriveCommands {
    #[command(about = "Mint an encrypted drive", long_about = None)]
    Mint {},
    #[command(about = "Enable/Disable drive minter", long_about = None)]
    SetEnabled { enabled: bool },
    #[command(about = "Set drive minter price", long_about = None)]
    SetPrice { price: u64 },
}

impl DriveCommands {
    pub async fn execute(self, client: &mut Client<'_>) -> Result<(), anyhow::Error> {
        let admin = client.load_wallet("admin")?;

        match self {
            DriveCommands::Mint {} => {
                let drive = client.admin_mint_drive(None).await?;
                info!("Minted drive: {}", drive.id);
            }
            DriveCommands::SetEnabled { enabled } => {
                client
                    .drive_minter()?
                    .set_enabled(&client.admin_cap()?, &admin, enabled)
                    .await?;
            }
            DriveCommands::SetPrice { price } => {
                client
                    .drive_minter()?
                    .set_price(&client.admin_cap()?, &admin, price)
                    .await?;
            }
        }

        Ok(())
    }
}
