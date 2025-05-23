use crate::packages::identifier;
use crate::packages::rarity::Rarity;
use move_core_types::account_address::AccountAddress;
use serde::{Deserialize, Serialize};
use sui_types::base_types::ObjectID;
use sui_types::programmable_transaction_builder::ProgrammableTransactionBuilder;
use sui_types::transaction::{Argument, Command};
use sui_types::type_input::{StructInput, TypeInput};

const PACKAGE: &str = "monster_part";

#[derive(Clone, Serialize, Deserialize)]
pub struct MonsterPartTemplate {
    pub name: String,
    part_type: u16,
    rarity: Rarity,
    params: Vec<MonsterPartParam>,
    parts: Vec<u16>,
}

impl MonsterPartTemplate {
    pub fn new(
        name: &str,
        part_type: u16,
        rarity: Rarity,
        params: Vec<MonsterPartParam>,
        parts: Vec<u16>,
    ) -> Self {
        Self {
            name: name.to_string(),
            part_type,
            rarity,
            params,
            parts,
        }
    }

    pub fn pure(
        &self,
        builder: &mut ProgrammableTransactionBuilder,
        package: ObjectID,
    ) -> Result<Argument, anyhow::Error> {
        let name = builder.pure(self.name.clone())?;
        let part_type = builder.pure(self.part_type)?;
        let rarity = self.rarity.pure(builder, package)?;
        let params_vec = self
            .params
            .iter()
            .map(|param| param.pure(builder, package))
            .collect::<Result<Vec<_>, _>>()?;

        let params = builder.command(Command::MakeMoveVec(
            Some(TypeInput::Struct(Box::new(StructInput {
                address: AccountAddress::from(package),
                module: "monster_part".to_string(),
                name: "MonsterPartParam".to_string(),
                type_params: vec![],
            }))),
            params_vec,
        ));

        let parts_vec = self
            .parts
            .iter()
            .map(|part| builder.pure(part))
            .collect::<Result<Vec<_>, _>>()?;

        let parts = builder.command(Command::MakeMoveVec(Some(TypeInput::U16), parts_vec));

        Ok(builder.programmable_move_call(
            package,
            identifier(PACKAGE),
            identifier("monster_part_template"),
            vec![],
            vec![name, part_type, rarity, params, parts],
        ))
    }
}

#[derive(Copy, Clone, Serialize, Deserialize)]
pub struct MonsterPartParam {
    min: u32,
    max: u32,
}

impl MonsterPartParam {
    pub fn new(min: u32, max: u32) -> Self {
        Self { min, max }
    }

    pub fn pure(
        self,
        builder: &mut ProgrammableTransactionBuilder,
        package: ObjectID,
    ) -> Result<Argument, anyhow::Error> {
        let min = builder.pure(self.min)?;
        let max = builder.pure(self.max)?;

        Ok(builder.programmable_move_call(
            package,
            identifier(PACKAGE),
            identifier("monster_part_param"),
            vec![],
            vec![min, max],
        ))
    }
}
