use crate::packages::admin_cap::{AdminCap, PartialPerm};
use crate::packages::monster_part::MonsterPartTemplate;
use crate::packages::{build_tx, find_objects, get_initial_shared_version, identifier};
use log::debug;
use std::sync::Arc;
use sui_sdk::SuiClient;
use sui_sdk::rpc_types::SuiTransactionBlockResponse;
use sui_types::base_types::ObjectID;
use sui_types::crypto::SuiKeyPair;
use sui_types::programmable_transaction_builder::ProgrammableTransactionBuilder;
use sui_types::transaction::{Argument, ObjectArg};

const PACKAGE: &str = "monster_part_registry";

pub struct MonsterRegistry {
    pub id: ObjectID,
    pub package: ObjectID,
    pub client: Arc<SuiClient>,
}

impl MonsterRegistry {
    pub fn new(
        client: Arc<SuiClient>,
        res: &SuiTransactionBlockResponse,
    ) -> Result<Self, anyhow::Error> {
        let (id, package) = find_objects(&res, "MonsterPartRegistry")?
            .first()
            .cloned()
            .unwrap();
        debug!("MonsterRegistry: {}", id.to_string());
        Ok(Self {
            id,
            package,
            client,
        })
    }

    pub async fn registry_argument(
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

    pub async fn register(
        &self,
        admin_cap: &AdminCap,
        signer: &SuiKeyPair,
        template: MonsterPartTemplate,
    ) -> Result<(), anyhow::Error> {
        self.batch_register(admin_cap, signer, vec![template])
            .await?;

        Ok(())
    }

    pub async fn register_part_type(
        &self,
        admin_cap: &AdminCap,
        signer: &SuiKeyPair,
        name: &str,
    ) -> Result<(), anyhow::Error> {
        let mut builder = ProgrammableTransactionBuilder::new();

        let registry = self.registry_argument(&mut builder, true).await?;

        let cap = admin_cap
            .partial_cap(PartialPerm::MonsterPartRegistryPerm, &mut builder)
            .await?;

        let name = builder.pure(name.to_string())?;

        builder.programmable_move_call(
            self.package,
            identifier(PACKAGE),
            identifier("register_part_type"),
            vec![],
            vec![registry, cap, name],
        );

        build_tx(&self.client, signer, builder.finish()).await?;

        Ok(())
    }

    pub async fn batch_register(
        &self,
        admin_cap: &AdminCap,
        signer: &SuiKeyPair,
        templates: Vec<MonsterPartTemplate>,
    ) -> Result<(), anyhow::Error> {
        let mut builder = ProgrammableTransactionBuilder::new();

        let registry = self.registry_argument(&mut builder, true).await?;

        let cap = admin_cap
            .partial_cap(PartialPerm::MonsterPartRegistryPerm, &mut builder)
            .await?;

        for template in templates {
            debug!("Registering template: {}", template.name.clone());

            let template = template.pure(&mut builder, self.package)?;

            builder.programmable_move_call(
                self.package,
                identifier(PACKAGE),
                identifier("monster_part_registry_register"),
                vec![],
                vec![registry, cap, template],
            );
        }

        build_tx(&self.client, signer, builder.finish()).await?;

        Ok(())
    }

    pub async fn unregister(
        &self,
        admin_cap: &AdminCap,
        signer: &SuiKeyPair,
        name: &str,
    ) -> Result<(), anyhow::Error> {
        let mut builder = ProgrammableTransactionBuilder::new();

        let registry = self.registry_argument(&mut builder, true).await?;

        let cap = admin_cap
            .partial_cap(PartialPerm::MonsterPartRegistryPerm, &mut builder)
            .await?;

        let name = builder.pure(name.to_string())?;

        builder.programmable_move_call(
            self.package,
            identifier(PACKAGE),
            identifier("unregister"),
            vec![],
            vec![registry, cap, name],
        );

        build_tx(&self.client, signer, builder.finish()).await?;

        Ok(())
    }

    pub async fn reregister(
        &self,
        admin_cap: &AdminCap,
        signer: &SuiKeyPair,
        name: &str,
    ) -> Result<(), anyhow::Error> {
        let mut builder = ProgrammableTransactionBuilder::new();

        let registry = self.registry_argument(&mut builder, true).await?;

        let cap = admin_cap
            .partial_cap(PartialPerm::MonsterPartRegistryPerm, &mut builder)
            .await?;

        let name = builder.pure(name.to_string())?;

        builder.programmable_move_call(
            self.package,
            identifier(PACKAGE),
            identifier("reregister"),
            vec![],
            vec![registry, cap, name],
        );

        build_tx(&self.client, signer, builder.finish()).await?;

        Ok(())
    }
}
