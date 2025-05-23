use crate::packages::admin_cap::{AdminCap, PartialPerm};
use crate::packages::distribution_table::DistributionTable;
use crate::packages::{build_tx, find_objects, get_initial_shared_version, identifier, query, sui};
use log::debug;
use std::collections::BTreeMap;
use std::sync::Arc;
use sui_sdk::SuiClient;
use sui_sdk::rpc_types::{SuiObjectDataFilter, SuiObjectDataOptions, SuiObjectResponseQuery};
use sui_types::SUI_RANDOMNESS_STATE_OBJECT_ID;
use sui_types::base_types::{ObjectID, SuiAddress};
use sui_types::crypto::SuiKeyPair;
use sui_types::programmable_transaction_builder::ProgrammableTransactionBuilder;
use sui_types::transaction::{Argument, Command, ObjectArg};

const PACKAGE: &str = "encrypted_drive";

pub struct EncryptedDriveMinter {
    pub id: ObjectID,
    pub package: ObjectID,
    pub client: Arc<SuiClient>,
    pub price: u64,
}

impl EncryptedDriveMinter {
    pub async fn create_new(
        admin_cap: &AdminCap,
        signer: &SuiKeyPair,
        price: u64,
    ) -> Result<Self, anyhow::Error> {
        let mut builder = ProgrammableTransactionBuilder::new();

        let perms = admin_cap
            .partial_cap(PartialPerm::EncryptedDriveMinterPerm, &mut builder)
            .await?;

        let cap = admin_cap
            .partial_cap(PartialPerm::EncryptedDrivePerm, &mut builder)
            .await?;

        let price_arg = builder.pure(price)?;

        builder.programmable_move_call(
            admin_cap.package,
            identifier(PACKAGE),
            identifier("encrypted_drive_minter"),
            vec![sui()],
            vec![perms, cap, price_arg],
        );

        let res = build_tx(&admin_cap.client, signer, builder.finish()).await?;
        let (id, package) = find_objects(&res, "EncryptedDriveMinter")?
            .first()
            .cloned()
            .unwrap();
        debug!("EncryptedDriveMinter: {}", id.to_string());

        Ok(Self {
            id,
            package,
            client: admin_cap.client.clone(),
            price,
        })
    }

    pub async fn admin_mint(
        &self,
        admin_cap: &AdminCap,
        builder: &mut ProgrammableTransactionBuilder,
    ) -> Result<Argument, anyhow::Error> {
        let cap = admin_cap
            .partial_cap(PartialPerm::EncryptedDrivePerm, builder)
            .await?;

        Ok(builder.programmable_move_call(
            self.package,
            identifier(PACKAGE),
            identifier("encrypted_drive"),
            vec![],
            vec![cap],
        ))
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

    pub async fn withdraw(
        &self,
        admin_cap: AdminCap,
        builder: &mut ProgrammableTransactionBuilder,
    ) -> Result<Argument, anyhow::Error> {
        let cap = admin_cap
            .partial_cap(PartialPerm::EncryptedDriveMinterPerm, builder)
            .await?;

        let minter = self.argument(builder, true).await?;

        Ok(builder.programmable_move_call(
            self.package,
            identifier(PACKAGE),
            identifier("withdraw"),
            vec![sui()],
            vec![minter, cap],
        ))
    }

    pub async fn set_enabled(
        &self,
        admin_cap: &AdminCap,
        signer: &SuiKeyPair,
        enable: bool,
    ) -> Result<(), anyhow::Error> {
        if enable {
            debug!("Enabling EncryptedDriveMinter");
        } else {
            debug!("Disabling EncryptedDriveMinter");
        }

        let mut builder = ProgrammableTransactionBuilder::new();

        let enable = builder.pure(enable)?;

        let cap = admin_cap
            .partial_cap(PartialPerm::EncryptedDriveMinterPerm, &mut builder)
            .await?;

        let minter = self.argument(&mut builder, true).await?;

        builder.programmable_move_call(
            self.package,
            identifier(PACKAGE),
            identifier("set_enabled"),
            vec![sui()],
            vec![minter, cap, enable],
        );

        build_tx(&self.client, signer, builder.finish()).await?;

        Ok(())
    }

    pub async fn set_price(
        &self,
        admin_cap: &AdminCap,
        signer: &SuiKeyPair,
        price: u64,
    ) -> Result<(), anyhow::Error> {
        let mut builder = ProgrammableTransactionBuilder::new();

        let price = builder.pure(price)?;

        let cap = admin_cap
            .partial_cap(PartialPerm::EncryptedDriveMinterPerm, &mut builder)
            .await?;

        let minter = self.argument(&mut builder, true).await?;

        builder.programmable_move_call(
            self.package,
            identifier(PACKAGE),
            identifier("set_price"),
            vec![sui()],
            vec![minter, cap, price],
        );

        build_tx(&self.client, signer, builder.finish()).await?;

        Ok(())
    }

    pub async fn mint(&self, signer: &SuiKeyPair) -> Result<EncryptedDrive, anyhow::Error> {
        let mut builder = ProgrammableTransactionBuilder::new();
        let owner = SuiAddress::from(&signer.public());

        let minter = self.argument(&mut builder, true).await?;
        let price = builder.pure(self.price)?;

        let coin = builder.command(Command::SplitCoins(Argument::GasCoin, vec![price]));

        builder.programmable_move_call(
            self.package,
            identifier(PACKAGE),
            identifier("mint"),
            vec![sui()],
            vec![minter, coin],
        );

        let res = build_tx(&self.client, signer, builder.finish()).await?;

        let (id, package) = find_objects(&res, "EncryptedDrive")?
            .first()
            .cloned()
            .unwrap();

        debug!("{} minted drive {}", owner.to_string(), id.to_string());

        Ok(EncryptedDrive {
            id,
            package,
            owner,
            client: self.client.clone(),
        })
    }

    pub async fn drives(
        &self,
        from: SuiAddress,
    ) -> Result<Vec<BTreeMap<String, String>>, anyhow::Error> {
        query(&self.client, from, self.package, PACKAGE, "EncryptedDrive").await
    }
}

#[derive(Clone)]
pub struct EncryptedDrive {
    pub id: ObjectID,
    pub package: ObjectID,
    pub client: Arc<SuiClient>,
    pub owner: SuiAddress,
}
