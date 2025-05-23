use crate::packages::admin_cap::{AdminCap, PartialPerm};
use crate::packages::distribution_table::DistributionTable;
use crate::packages::encrypted_drive::EncryptedDrive;
use crate::packages::monster_part_registry::MonsterRegistry;
use crate::packages::{build_tx, find_objects, get_initial_shared_version, identifier, query};
use log::{debug, info};
use std::collections::BTreeMap;
use std::sync::Arc;
use sui_sdk::SuiClient;
use sui_sdk::rpc_types::SuiTransactionBlockResponse;
use sui_types::SUI_RANDOMNESS_STATE_OBJECT_ID;
use sui_types::base_types::{ObjectID, SuiAddress};
use sui_types::crypto::SuiKeyPair;
use sui_types::programmable_transaction_builder::ProgrammableTransactionBuilder;
use sui_types::transaction::{Argument, ObjectArg};

const PACKAGE: &str = "monster";

pub struct MonsterMinter {
    pub id: ObjectID,
    pub package: ObjectID,
    pub client: Arc<SuiClient>,
}

impl MonsterMinter {
    pub fn new(
        client: Arc<SuiClient>,
        res: &SuiTransactionBlockResponse,
    ) -> Result<Self, anyhow::Error> {
        let (id, package) = find_objects(&res, "MonsterMinter")?
            .first()
            .cloned()
            .unwrap();
        debug!("MonsterMinter: {}", id.to_string());
        Ok(Self {
            id,
            package,
            client,
        })
    }

    pub async fn minter_argument(
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

    pub async fn set_enabled(
        &self,
        admin_cap: &AdminCap,
        signer: &SuiKeyPair,
        enable: bool,
    ) -> Result<(), anyhow::Error> {
        if enable {
            debug!("Enabling MonsterMinter");
        } else {
            debug!("Disabling MonsterMinter");
        }

        let mut builder = ProgrammableTransactionBuilder::new();

        let enable = builder.pure(enable)?;

        let cap = admin_cap
            .partial_cap(PartialPerm::MonsterMinterPerm, &mut builder)
            .await?;

        let minter = self.minter_argument(&mut builder, true).await?;

        builder.programmable_move_call(
            self.package,
            identifier(PACKAGE),
            identifier("set_enabled"),
            vec![],
            vec![minter, cap, enable],
        );

        build_tx(&self.client, signer, builder.finish()).await?;

        Ok(())
    }

    pub async fn generate(
        &self,
        signer: &SuiKeyPair,
        registry: &MonsterRegistry,
        distrib: &DistributionTable,
        drive: EncryptedDrive,
    ) -> Result<Monster, anyhow::Error> {
        let mut builder = ProgrammableTransactionBuilder::new();

        let minter = self.minter_argument(&mut builder, false).await?;
        let distrib = distrib.argument(&mut builder, false).await?;
        let registry = registry.registry_argument(&mut builder, false).await?;
        let random = builder.obj(ObjectArg::SharedObject {
            id: SUI_RANDOMNESS_STATE_OBJECT_ID,
            initial_shared_version: get_initial_shared_version(
                &self.client,
                SUI_RANDOMNESS_STATE_OBJECT_ID,
            )
            .await?,
            mutable: false,
        })?;

        let builder_api = self.client.transaction_builder();

        let drive_ref = builder_api.get_object_ref(drive.id).await?;
        let drive = builder.obj(ObjectArg::ImmOrOwnedObject(drive_ref))?;

        builder.programmable_move_call(
            self.package,
            identifier(PACKAGE),
            identifier("generate"),
            vec![],
            vec![minter, drive, registry, distrib, random],
        );

        let res = build_tx(&self.client, signer, builder.finish()).await?;

        let owner = SuiAddress::from(&signer.public());

        let (id, package) = find_objects(&res, "Monster")?.first().cloned().unwrap();

        debug!("{} minted monster {}", owner.to_string(), id.to_string());

        Ok(Monster {
            id,
            package,
            owner,
            client: self.client.clone(),
        })
    }

    pub async fn monsters(
        &self,
        from: SuiAddress,
    ) -> Result<Vec<BTreeMap<String, String>>, anyhow::Error> {
        query(&self.client, from, self.package, PACKAGE, "Monster").await
    }
}

#[derive(Clone)]
pub struct Monster {
    pub id: ObjectID,
    pub package: ObjectID,
    pub client: Arc<SuiClient>,
    pub owner: SuiAddress,
}
