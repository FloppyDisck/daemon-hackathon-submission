use crate::sui_client::Client;
use bip39::{Language, Mnemonic, MnemonicType, Seed};
use clap::{Subcommand, ValueEnum};
use fastcrypto::ed25519::{Ed25519KeyPair, Ed25519PrivateKey};
use fastcrypto::secp256k1::{Secp256k1KeyPair, Secp256k1PrivateKey};
use fastcrypto::secp256r1::{Secp256r1KeyPair, Secp256r1PrivateKey};
use fastcrypto::traits::{EncodeDecodeBase64, KeyPair};
use log::info;
use sui_keys::key_derive::derive_key_pair_from_path;
use sui_types::base_types::SuiAddress;
use sui_types::crypto::{SignatureScheme, SuiKeyPair};

#[derive(Subcommand)]

pub enum WalletCommands {
    #[command(about = "Import a private key with a given name", long_about = None)]
    Import {
        name: String,
        #[arg(short, long)]
        mnemonic: Option<String>,
        #[arg(short, long)]
        private_key: Option<String>,
        #[arg(short, long)]
        key_type: Option<KeyType>,
    },

    #[command(about = "Generate a new address", long_about = None)]
    Generate {
        name: String,
        #[arg(short, long)]
        key_type: Option<KeyType>,
    },

    #[command(about = "Exports a key's private key", long_about = None)]
    Export { name: String },

    #[command(about = "Lists all available keys", long_about = None)]
    List {},

    #[command(about = "Attempts to remove a key with a given name", long_about = None)]
    Remove { name: String },
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Default)]
pub enum KeyType {
    #[default]
    Ed25519,
    Secp256k1,
    Secp256r1,
}

impl WalletCommands {
    pub async fn execute(self, client: &mut Client<'_>) -> Result<(), anyhow::Error> {
        match self {
            WalletCommands::Import {
                name,
                mnemonic,
                private_key,
                key_type,
            } => {
                let key_type = key_type.unwrap_or_default();

                let key_pair = if let Some(mnemonic) = mnemonic {
                    // Parse the mnemonic
                    let mnemonic = Mnemonic::from_phrase(&mnemonic, Language::English)?;
                    from_mnemonic(mnemonic, key_type)?
                } else if let Some(private_key) = private_key {
                    match key_type {
                        KeyType::Ed25519 => SuiKeyPair::Ed25519(Ed25519KeyPair::from(
                            Ed25519PrivateKey::decode_base64(&private_key)?,
                        )),
                        KeyType::Secp256k1 => SuiKeyPair::Secp256k1(Secp256k1KeyPair::from(
                            Secp256k1PrivateKey::decode_base64(&private_key)?,
                        )),
                        KeyType::Secp256r1 => SuiKeyPair::Secp256r1(Secp256r1KeyPair::from(
                            Secp256r1PrivateKey::decode_base64(&private_key)?,
                        )),
                    }
                } else {
                    return Ok(());
                };

                client.save_wallet(&name, &key_pair)?;
                info!(
                    "Saved: {}",
                    SuiAddress::from(&key_pair.public()).to_string()
                );
            }
            WalletCommands::Export { name } => {
                let key = client.load_wallet(&name)?;

                let private_key = match key {
                    SuiKeyPair::Ed25519(key) => key.private().to_string(),
                    SuiKeyPair::Secp256k1(key) => key.private().to_string(),
                    SuiKeyPair::Secp256r1(key) => key.private().to_string(),
                };

                info!("{}", private_key);
            }
            WalletCommands::List {} => {
                for key in client.config.keys.iter() {
                    info!("{}: {}", key, client.sui_address(key)?);
                }
            }
            WalletCommands::Remove { name } => {
                let key = client.remove_wallet(&name)?;
                info!("Removed: {}", SuiAddress::from(&key.public()).to_string());
            }
            WalletCommands::Generate { name, key_type } => {
                let mnemonic = Mnemonic::new(MnemonicType::Words12, Language::English);
                info!("Mnemonic: {}", mnemonic.phrase());
                let key = from_mnemonic(mnemonic, key_type.unwrap_or_default())?;
                client.save_wallet(&name, &key)?;
                info!("Generated: {}", SuiAddress::from(&key.public()).to_string());
            }
        }

        Ok(())
    }
}

fn from_mnemonic(mnemonic: Mnemonic, key_type: KeyType) -> Result<SuiKeyPair, anyhow::Error> {
    // Generate seed from mnemonic
    let seed = Seed::new(&mnemonic, "");

    // Derive the key pair
    let scheme = match key_type {
        KeyType::Ed25519 => SignatureScheme::ED25519,
        KeyType::Secp256k1 => SignatureScheme::Secp256k1,
        KeyType::Secp256r1 => SignatureScheme::Secp256r1,
    };

    Ok(derive_key_pair_from_path(seed.as_bytes(), None, &scheme)?.1)
}
