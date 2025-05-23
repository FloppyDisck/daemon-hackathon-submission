use anyhow::anyhow;
use log::{debug, error};
use move_core_types::identifier::Identifier;
use move_core_types::language_storage::{StructTag, TypeTag};
use shared_crypto::intent::{Intent, IntentMessage};
use std::collections::BTreeMap;
use std::str::FromStr;
use sui_sdk::SuiClient;
use sui_sdk::rpc_types::{
    Coin, ObjectChange, SuiObjectDataFilter, SuiObjectDataOptions, SuiObjectResponseQuery,
    SuiTransactionBlockResponse, SuiTransactionBlockResponseOptions,
};
use sui_types::SUI_FRAMEWORK_ADDRESS;
use sui_types::base_types::{ObjectID, ObjectType, SequenceNumber, SuiAddress};
use sui_types::crypto::{Signature, SuiKeyPair};
use sui_types::object::Owner;
use sui_types::quorum_driver_types::ExecuteTransactionRequestType;
use sui_types::transaction::{ProgrammableTransaction, Transaction, TransactionData};

pub mod admin_cap;
pub mod distribution_table;
pub mod encrypted_drive;
pub mod monster;
pub mod rarity;

pub use monster::*;

pub fn sui() -> TypeTag {
    TypeTag::Struct(Box::new(StructTag {
        address: SUI_FRAMEWORK_ADDRESS,
        module: identifier("sui"),
        name: identifier("SUI"),
        type_params: vec![],
    }))
}

pub fn assert_success(tx: &SuiTransactionBlockResponse) -> Result<(), anyhow::Error> {
    if !tx.errors.is_empty() {
        error!("Transaction: {:?}", tx.transaction);
        error!("Transaction error");
        Err(anyhow!("Found: {:?}", tx.errors))
    } else if !tx.status_ok().unwrap_or(true) {
        error!("Transaction: {:?}", tx.transaction);
        error!("Effects: {:?}", tx.effects);
        Err(anyhow!("Status error"))
    } else {
        Ok(())
    }
}

pub fn identifier(s: &str) -> Identifier {
    Identifier::from_str(s).unwrap()
}

pub fn find_objects(
    res: &SuiTransactionBlockResponse,
    name: &str,
) -> Result<Vec<(ObjectID, ObjectID)>, anyhow::Error> {
    let mut objects = vec![];
    if let Some(object_changes) = &res.object_changes {
        for object_change in object_changes {
            match object_change {
                ObjectChange::Created {
                    object_id,
                    object_type,
                    ..
                } => {
                    if object_type.name == Identifier::from_str(name)? {
                        objects.push((object_id.clone(), ObjectID::from(object_type.address)));
                    }
                }
                _ => {}
            }
        }
    }
    Ok(objects)
}

pub async fn gas_coin(client: &SuiClient, from: SuiAddress) -> Result<Coin, anyhow::Error> {
    let coins = client
        .coin_read_api()
        .get_coins(from, None, None, None)
        .await?;
    Ok(coins.data.into_iter().next().unwrap())
}

pub async fn query(
    client: &SuiClient,
    from: SuiAddress,
    package: ObjectID,
    module: &str,
    name: &str,
) -> Result<Vec<BTreeMap<String, String>>, anyhow::Error> {
    Ok(client
        .read_api()
        .get_owned_objects(
            from,
            Some(SuiObjectResponseQuery {
                filter: Some(SuiObjectDataFilter::MoveModule {
                    package,
                    module: identifier(module),
                }),
                options: Some(SuiObjectDataOptions {
                    show_type: true,
                    show_owner: false,
                    show_previous_transaction: false,
                    show_display: true,
                    show_content: false,
                    show_bcs: false,
                    show_storage_rebate: false,
                }),
            }),
            None,
            None,
        )
        .await?
        .data
        .iter()
        .filter(|item| {
            let data = item.data.as_ref().unwrap().type_.as_ref().unwrap();
            if let ObjectType::Struct(struct_type) = data {
                return struct_type.name().as_str() == name;
            }

            false
        })
        .map(|item| {
            item.data
                .as_ref()
                .unwrap()
                .display
                .as_ref()
                .unwrap()
                .data
                .as_ref()
                .unwrap()
                .clone()
        })
        .collect())
}

pub async fn build_tx(
    client: &SuiClient,
    signer: &SuiKeyPair,
    tx: ProgrammableTransaction,
) -> Result<SuiTransactionBlockResponse, anyhow::Error> {
    let gas_budget = 50_000_000;
    let gas_price = client.read_api().get_reference_gas_price().await?;
    let addr = SuiAddress::from(&signer.public());

    let coin = gas_coin(client, addr).await?;

    // create the transaction data that will be sent to the network.
    let tx_data =
        TransactionData::new_programmable(addr, vec![coin.object_ref()], tx, gas_budget, gas_price);

    let tx = sign_and_publish(client, signer, tx_data).await?;
    assert_success(&tx)?;
    Ok(tx)
}

pub async fn sign_and_publish(
    client: &SuiClient,
    signer: &SuiKeyPair,
    tx_data: TransactionData,
) -> Result<SuiTransactionBlockResponse, anyhow::Error> {
    let signature = Signature::new_secure(
        &IntentMessage::new(Intent::sui_transaction(), &tx_data),
        signer,
    );

    let transaction_response = client
        .quorum_driver_api()
        .execute_transaction_block(
            Transaction::from_data(tx_data, vec![signature]),
            SuiTransactionBlockResponseOptions::full_content(),
            Some(ExecuteTransactionRequestType::WaitForLocalExecution),
        )
        .await?;

    Ok(transaction_response)
}

pub async fn get_initial_shared_version(
    client: &SuiClient,
    id: ObjectID,
) -> Result<SequenceNumber, anyhow::Error> {
    match client
        .read_api()
        .get_object_with_options(
            id,
            SuiObjectDataOptions {
                show_type: false,
                show_owner: true,
                show_previous_transaction: false,
                show_display: false,
                show_content: false,
                show_bcs: false,
                show_storage_rebate: false,
            },
        )
        .await?
        .owner()
        .unwrap()
    {
        Owner::Shared {
            initial_shared_version,
        } => Ok(initial_shared_version),
        _ => Err(anyhow!("Now shared owner.")),
    }
}
