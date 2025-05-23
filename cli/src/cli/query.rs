use crate::sui_client::Client;
use clap::Subcommand;

#[derive(Subcommand)]
pub enum QueryCommands {
    Drives { account: String },
    Monsters { account: String },
}

impl QueryCommands {
    pub async fn execute(self, client: &Client<'_>) -> Result<(), anyhow::Error> {
        let res = match self {
            QueryCommands::Drives { account } => client.drives(&account).await?,
            QueryCommands::Monsters { account } => client.monsters(&account).await?,
        };

        for item in res {
            println!("{:?}", item);
        }

        Ok(())
    }
}
