use crate::packages::{find_objects, identifier};
use log::debug;
use move_core_types::account_address::AccountAddress;
use move_core_types::language_storage::{StructTag, TypeTag};
use std::sync::Arc;
use sui_sdk::SuiClient;
use sui_sdk::rpc_types::SuiTransactionBlockResponse;
use sui_types::base_types::ObjectID;
use sui_types::programmable_transaction_builder::ProgrammableTransactionBuilder;
use sui_types::transaction::{Argument, ObjectArg};

const PACKAGE: &str = "admin_cap";

pub struct AdminCap {
    pub id: ObjectID,
    pub package: ObjectID,
    pub client: Arc<SuiClient>,
}

impl AdminCap {
    pub fn new(
        client: Arc<SuiClient>,
        res: &SuiTransactionBlockResponse,
    ) -> Result<Self, anyhow::Error> {
        let (id, package) = find_objects(&res, "AdminCap")?.first().cloned().unwrap();
        debug!("AdminCap: {}", id.to_string());
        Ok(Self {
            id,
            package,
            client,
        })
    }

    pub async fn partial_cap(
        &self,
        perm: PartialPerm,
        builder: &mut ProgrammableTransactionBuilder,
    ) -> Result<Argument, anyhow::Error> {
        let builder_api = self.client.transaction_builder();
        let admin_cap_ref = builder_api.get_object_ref(self.id).await?;

        let admin_cap = builder.obj(ObjectArg::ImmOrOwnedObject(admin_cap_ref))?;

        Ok(builder.programmable_move_call(
            self.package,
            identifier(PACKAGE),
            identifier("permit"),
            vec![perm.as_type_tag(AccountAddress::from(self.package))],
            vec![admin_cap],
        ))
    }
}

pub enum PartialPerm {
    EncryptedDriveMinterPerm,
    EncryptedDrivePerm,
    MonsterPartRegistryPerm,
    MonsterMinterPerm,
    MonsterPerm,
    DistributionTablePerm,
}

impl PartialPerm {
    pub fn as_type_tag(&self, package: AccountAddress) -> TypeTag {
        let name = match self {
            PartialPerm::EncryptedDriveMinterPerm => identifier("EncryptedDriveMinterPerm"),
            PartialPerm::EncryptedDrivePerm => identifier("EncryptedDrivePerm"),
            PartialPerm::MonsterPartRegistryPerm => identifier("MonsterPartRegistryPerm"),
            PartialPerm::MonsterMinterPerm => identifier("MonsterMinterPerm"),
            PartialPerm::MonsterPerm => identifier("MonsterPerm"),
            PartialPerm::DistributionTablePerm => identifier("DistributionTablePerm"),
        };

        TypeTag::Struct(Box::new(StructTag {
            address: package,
            module: identifier(PACKAGE),
            name,
            type_params: vec![],
        }))
    }
}
