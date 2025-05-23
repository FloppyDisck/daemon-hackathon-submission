use crate::packages::admin_cap::{AdminCap, PartialPerm};
use crate::packages::{build_tx, find_objects, get_initial_shared_version, identifier};
use log::debug;
use std::sync::Arc;
use sui_sdk::SuiClient;
use sui_sdk::rpc_types::SuiTransactionBlockResponse;
use sui_types::base_types::ObjectID;
use sui_types::crypto::SuiKeyPair;
use sui_types::programmable_transaction_builder::ProgrammableTransactionBuilder;
use sui_types::transaction::{Argument, Command, ObjectArg};
use sui_types::type_input::TypeInput;

const PACKAGE: &str = "distribution_table";

pub struct DistributionTable {
    pub id: ObjectID,
    pub package: ObjectID,
    pub client: Arc<SuiClient>,
}

impl DistributionTable {
    pub fn new(
        client: Arc<SuiClient>,
        res: &SuiTransactionBlockResponse,
    ) -> Result<Self, anyhow::Error> {
        let (id, package) = find_objects(&res, "DistributionTable")?
            .first()
            .cloned()
            .unwrap();
        debug!("DistributionTable: {}", id.to_string());
        Ok(Self {
            id,
            package,
            client,
        })
    }

    pub async fn argument(
        &self,
        builder: &mut ProgrammableTransactionBuilder,
        mutable: bool,
    ) -> Result<Argument, anyhow::Error> {
        builder.obj(ObjectArg::SharedObject {
            id: self.id,
            initial_shared_version: get_initial_shared_version(&self.client, self.id).await?,
            mutable,
        })
    }

    pub async fn set_rarity_distribution(
        &self,
        admin_cap: &AdminCap,
        signer: &SuiKeyPair,
        distribution: Vec<u16>,
    ) -> Result<(), anyhow::Error> {
        self.set_distribution("set_rarity_distribution", admin_cap, signer, distribution)
            .await
    }

    pub async fn set_protocol_distribution(
        &self,
        admin_cap: &AdminCap,
        signer: &SuiKeyPair,
        distribution: Vec<u16>,
    ) -> Result<(), anyhow::Error> {
        self.set_distribution("set_protocol_distribution", admin_cap, signer, distribution)
            .await
    }

    async fn set_distribution(
        &self,
        func: &str,
        admin_cap: &AdminCap,
        signer: &SuiKeyPair,
        distribution: Vec<u16>,
    ) -> Result<(), anyhow::Error> {
        let mut builder = ProgrammableTransactionBuilder::new();

        let distribution_table = self.argument(&mut builder, true).await?;

        let cap = admin_cap
            .partial_cap(PartialPerm::DistributionTablePerm, &mut builder)
            .await?;

        let mut distrib_args = vec![];
        for d in distribution {
            distrib_args.push(builder.pure(d)?);
        }

        let distribution =
            builder.command(Command::MakeMoveVec(Some(TypeInput::U16), distrib_args));

        builder.programmable_move_call(
            self.package,
            identifier(PACKAGE),
            identifier(func),
            vec![],
            vec![distribution_table, cap, distribution],
        );

        build_tx(&self.client, signer, builder.finish()).await?;

        Ok(())
    }
}
