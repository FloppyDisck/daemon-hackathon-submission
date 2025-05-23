use crate::packages::identifier;
use serde::{Deserialize, Serialize};
use sui_types::base_types::ObjectID;
use sui_types::programmable_transaction_builder::ProgrammableTransactionBuilder;
use sui_types::transaction::Argument;

const PACKAGE: &str = "rarity";

#[repr(u8)]
#[derive(Copy, Clone, Serialize, Deserialize)]
pub enum Rarity {
    Common = 0,
    Uncommon = 1,
    Rare = 2,
    Legendary = 3,
    Epic = 4,
    Unique = 5,
}

impl Rarity {
    pub fn size() -> usize {
        6
    }
    pub fn pure(
        self,
        builder: &mut ProgrammableTransactionBuilder,
        package: ObjectID,
    ) -> Result<Argument, anyhow::Error> {
        let rarity = builder.pure(self as u8)?;

        Ok(builder.programmable_move_call(
            package,
            identifier(PACKAGE),
            identifier("from_u8"),
            vec![],
            vec![rarity],
        ))
    }
}

impl From<u8> for Rarity {
    fn from(rarity: u8) -> Self {
        match rarity {
            0 => Rarity::Common,
            1 => Rarity::Uncommon,
            2 => Rarity::Rare,
            3 => Rarity::Legendary,
            4 => Rarity::Epic,
            _ => Rarity::Unique,
        }
    }
}
