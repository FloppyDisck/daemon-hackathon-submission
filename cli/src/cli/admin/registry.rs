use crate::sui_client::Client;
use clap::Subcommand;

#[derive(Subcommand)]
pub enum RegistryCommands {
    #[command(about = "Register template, the template must be a json encoded string", long_about = None)]
    Register { template: String },
    #[command(about = "Unregister a template", long_about = None)]
    Unregister { template: String },
    #[command(about = "Reregister an unregistered template", long_about = None)]
    Reregister { template: String },
}

impl RegistryCommands {
    pub async fn execute(self, client: &mut Client<'_>) -> Result<(), anyhow::Error> {
        let admin = client.load_wallet("admin")?;
        let admin_cap = client.admin_cap()?;

        match self {
            RegistryCommands::Register { template } => {
                client
                    .registry()?
                    .register(&admin_cap, &admin, serde_json::from_str(&template)?)
                    .await?;
            }
            RegistryCommands::Unregister { template } => {
                client
                    .registry()?
                    .unregister(&admin_cap, &admin, &template)
                    .await?;
            }
            RegistryCommands::Reregister { template } => {
                client
                    .registry()?
                    .reregister(&admin_cap, &admin, &template)
                    .await?;
            }
        }

        Ok(())
    }
}
