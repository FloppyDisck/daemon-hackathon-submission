use crate::cli::NetType;
use crate::localnet_process::LocalnetProcess;
use crate::network_config::{Deployment, NetworkConfig};
use crate::packages::admin_cap::AdminCap;
use crate::packages::distribution_table::DistributionTable;
use crate::packages::encrypted_drive::{EncryptedDrive, EncryptedDriveMinter};
use crate::packages::monster_minter::{Monster, MonsterMinter};
use crate::packages::monster_part::{MonsterPartParam, MonsterPartTemplate};
use crate::packages::monster_part_registry::MonsterRegistry;
use crate::packages::rarity::Rarity;
use crate::packages::{build_tx, find_objects, sign_and_publish};
use anyhow::anyhow;
use fastcrypto::traits::EncodeDecodeBase64;
use keyring::Entry;
use log::{debug, info};
use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use sui_sdk::rpc_types::SuiTransactionBlockResponse;
use sui_sdk::types::base_types::SuiAddress;
use sui_sdk::types::crypto::{SuiKeyPair, get_key_pair_from_rng};
use sui_sdk::types::transaction::TransactionData;
use sui_sdk::{SuiClient, SuiClientBuilder};
use sui_types::base_types::ObjectID;
use sui_types::programmable_transaction_builder::ProgrammableTransactionBuilder;
use sui_types::transaction::ProgrammableTransaction;

pub const ADMIN: &str = "admin";

pub struct Client<'a> {
    service_name: String,
    client: Arc<SuiClient>,
    pub config: &'a mut NetworkConfig,
}

impl<'a> Client<'a> {
    pub async fn new(
        network: NetType,
        config: &'a mut NetworkConfig,
    ) -> Result<Self, anyhow::Error> {
        let sui_client = SuiClientBuilder::default().build(&config.rpc).await?;
        let service_name = format!(
            "daemon_cli_{}",
            match network {
                NetType::Localnet => "localnet",
                NetType::Testnet => "testnet",
                NetType::Mainnet => "mainnet",
            }
        );

        Ok(Self {
            service_name,
            client: Arc::new(sui_client),
            config,
        })
    }

    pub fn admin(&self) -> Result<SuiKeyPair, anyhow::Error> {
        self.load_wallet(ADMIN)
    }

    pub async fn publish_package(&mut self, minter_price: u64) -> Result<(), anyhow::Error> {
        debug!("Building package");
        let pkg = sui_move_build::BuildConfig {
            config: Default::default(),
            run_bytecode_verifier: true,
            print_diags_to_stderr: true,
            chain_id: None,
        }
        .build(&Path::new("./contracts"))?;

        let package_bytes = pkg.get_package_bytes(false);
        let deps = pkg.get_published_dependencies_ids();

        let builder = self.client.transaction_builder();

        let tx = builder
            .publish(
                self.sui_address(ADMIN)?,
                package_bytes,
                deps,
                None,
                150_000_000,
            )
            .await?;

        debug!("Publishing package");
        let res = self.sign_and_publish(ADMIN, tx).await?;

        let admin_cap = AdminCap::new(self.client.clone(), &res)?;
        let monster_registry = MonsterRegistry::new(self.client.clone(), &res)?;

        let signer = self.load_wallet(ADMIN)?;

        let monster_minter = MonsterMinter::new(self.client.clone(), &res)?;
        monster_minter
            .set_enabled(&admin_cap, &signer, true)
            .await?;

        let distrib = DistributionTable::new(self.client.clone(), &res)?;

        let drive_minter =
            EncryptedDriveMinter::create_new(&admin_cap, &signer, minter_price).await?;
        drive_minter.set_enabled(&admin_cap, &signer, true).await?;

        let _ = self.config.deployment.insert(Deployment {
            package: admin_cap.package.to_string(),
            admin_cap: admin_cap.id.to_string(),
            registry: monster_registry.id.to_string(),
            monster_minter: monster_minter.id.to_string(),
            drive_minter: drive_minter.id.to_string(),
            distribution_table: distrib.id.to_string(),
        });

        Ok(())
    }

    pub async fn admin_mint_drive(
        &self,
        sent_to: Option<&str>,
    ) -> Result<EncryptedDrive, anyhow::Error> {
        let mut builder = ProgrammableTransactionBuilder::new();

        let minter = self
            .drive_minter()?
            .admin_mint(&self.admin_cap()?, &mut builder)
            .await?;

        let owner = if let Some(send_to) = sent_to {
            builder.transfer_arg(self.sui_address(send_to)?, minter);
            self.sui_address(send_to)?
        } else {
            self.sui_address(ADMIN)?
        };

        let res = self.build_tx(ADMIN, builder.finish()).await?;

        let (id, package) = find_objects(&res, "EncryptedDrive")?
            .first()
            .cloned()
            .unwrap();

        Ok(EncryptedDrive {
            id,
            package,
            owner,
            client: self.client.clone(),
        })
    }

    pub async fn mint_drives(
        &self,
        signer: &str,
        amount: usize,
    ) -> Result<Vec<EncryptedDrive>, anyhow::Error> {
        let wallet = self.load_wallet(signer)?;

        let drive_minter = self.drive_minter()?;

        let mut drives = vec![];
        for _ in 0..amount {
            drives.push(drive_minter.mint(&wallet).await?);
        }

        Ok(drives)
    }

    pub async fn mint_drive(&self, signer: &str) -> Result<EncryptedDrive, anyhow::Error> {
        self.drive_minter()?.mint(&self.load_wallet(signer)?).await
    }

    pub async fn drives(&self, from: &str) -> Result<Vec<BTreeMap<String, String>>, anyhow::Error> {
        self.drive_minter()?.drives(self.sui_address(from)?).await
    }

    pub async fn mint_monsters(
        &self,
        drives: Vec<EncryptedDrive>,
    ) -> Result<Vec<Monster>, anyhow::Error> {
        let minter = self.monster_minter()?;
        let registry = self.registry()?;
        let distrib = self.distribution_table()?;
        let mut monsters = vec![];
        for drive in drives {
            let signer = drive.owner.to_string();
            let wallet = self.load_wallet(&signer)?;

            monsters.push(minter.generate(&wallet, &registry, &distrib, drive).await?)
        }

        Ok(monsters)
    }

    pub async fn mint_monster(&self, drive: EncryptedDrive) -> Result<Monster, anyhow::Error> {
        let wallet = self.load_wallet_from_addr(drive.owner)?;

        self.monster_minter()?
            .generate(
                &wallet,
                &self.registry()?,
                &self.distribution_table()?,
                drive,
            )
            .await
    }

    pub async fn monsters(
        &self,
        from: &str,
    ) -> Result<Vec<BTreeMap<String, String>>, anyhow::Error> {
        self.monster_minter()?
            .monsters(self.sui_address(from)?)
            .await
    }

    pub async fn set_rarity_distribution(
        &self,
        distribution: Vec<u16>,
    ) -> Result<(), anyhow::Error> {
        self.distribution_table()?
            .set_rarity_distribution(&self.admin_cap()?, &self.load_wallet(ADMIN)?, distribution)
            .await
    }

    pub async fn set_protocol_distribution(
        &self,
        distribution: Vec<u16>,
    ) -> Result<(), anyhow::Error> {
        self.distribution_table()?
            .set_protocol_distribution(&self.admin_cap()?, &self.load_wallet(ADMIN)?, distribution)
            .await
    }

    pub fn deployment(&self) -> Result<&Deployment, anyhow::Error> {
        self.config
            .deployment
            .as_ref()
            .ok_or(anyhow!("No deployment found"))
    }

    pub fn admin_cap(&self) -> Result<AdminCap, anyhow::Error> {
        let config = self.deployment()?;
        Ok(AdminCap {
            id: ObjectID::from_str(&config.admin_cap)?,
            package: ObjectID::from_str(&config.package)?,
            client: self.client.clone(),
        })
    }

    pub fn registry(&self) -> Result<MonsterRegistry, anyhow::Error> {
        let config = self.deployment()?;
        Ok(MonsterRegistry {
            id: ObjectID::from_str(&config.registry)?,
            package: ObjectID::from_str(&config.package)?,
            client: self.client.clone(),
        })
    }

    pub fn monster_minter(&self) -> Result<MonsterMinter, anyhow::Error> {
        let config = self.deployment()?;
        Ok(MonsterMinter {
            id: ObjectID::from_str(&config.monster_minter)?,
            package: ObjectID::from_str(&config.package)?,
            client: self.client.clone(),
        })
    }

    pub fn drive_minter(&self) -> Result<EncryptedDriveMinter, anyhow::Error> {
        let config = self.deployment()?;
        Ok(EncryptedDriveMinter {
            id: ObjectID::from_str(&config.drive_minter)?,
            package: ObjectID::from_str(&config.package)?,
            client: self.client.clone(),
            price: 10,
        })
    }

    pub fn distribution_table(&self) -> Result<DistributionTable, anyhow::Error> {
        let config = self.deployment()?;
        Ok(DistributionTable {
            id: ObjectID::from_str(&config.distribution_table)?,
            package: ObjectID::from_str(&config.package)?,
            client: self.client.clone(),
        })
    }

    pub fn wallet_exists(&self, name: &str) -> Result<bool, anyhow::Error> {
        let entry = Entry::new(&self.service_name, &name)?;
        Ok(entry.get_password().is_ok())
    }

    pub fn generate_wallet(&mut self, name: &str) -> Result<SuiKeyPair, anyhow::Error> {
        let key = generate_address();

        self.save_wallet(name, &key)?;

        Ok(key)
    }

    pub fn save_wallet(&mut self, name: &str, key: &SuiKeyPair) -> Result<(), anyhow::Error> {
        let secret = key.encode_base64();

        let name_entry = Entry::new(&self.service_name, &name)?;
        name_entry.set_password(&secret)?;

        let addr_entry = Entry::new(
            &self.service_name,
            &SuiAddress::from(&key.public()).to_string(),
        )?;
        addr_entry.set_password(&secret)?;

        if !self.config.keys.contains(&name.to_string()) {
            self.config.keys.push(name.to_string());
        }

        Ok(())
    }

    pub fn load_wallet_from_addr(&self, addr: SuiAddress) -> Result<SuiKeyPair, anyhow::Error> {
        self.load_wallet(&addr.to_string())
    }

    pub fn load_wallet(&self, name: &str) -> Result<SuiKeyPair, anyhow::Error> {
        let entry = Entry::new(&self.service_name, &name)?;
        let pass = entry.get_password()?;
        Ok(SuiKeyPair::decode_base64(&pass)?)
    }

    pub fn remove_wallet(&self, name: &str) -> Result<SuiKeyPair, anyhow::Error> {
        let entry = Entry::new(&self.service_name, &name)?;
        let key = self.load_wallet(name)?;
        entry.delete_credential()?;
        let addr_entry = Entry::new(
            &self.service_name,
            &SuiAddress::from(&key.public()).to_string(),
        )?;
        addr_entry.delete_credential()?;
        Ok(key)
    }

    pub fn sui_address(&self, name: &str) -> Result<SuiAddress, anyhow::Error> {
        Ok(SuiAddress::from(&self.load_wallet(name)?.public()))
    }

    pub async fn build_tx(
        &self,
        signer: &str,
        tx: ProgrammableTransaction,
    ) -> Result<SuiTransactionBlockResponse, anyhow::Error> {
        build_tx(&self.client, &self.load_wallet(signer)?, tx).await
    }

    pub async fn sign_and_publish(
        &self,
        signer: &str,
        tx: TransactionData,
    ) -> Result<SuiTransactionBlockResponse, anyhow::Error> {
        sign_and_publish(&self.client, &self.load_wallet(signer)?, tx).await
    }

    pub async fn init_registry(&self) -> Result<(), anyhow::Error> {
        let registry = self.registry()?;
        let admin_cap = self.admin_cap()?;
        let admin = self.load_wallet(ADMIN)?;

        info!("Initializing registry");

        let time = Instant::now();

        let mut part_types = HashMap::new();

        for (i, name) in [
            "torso",
            "head",
            "eye",
            "addon",
            "robot_leg_base",
            "robot_leg_tibia",
            "robot_leg_joint",
            "robot_leg_end",
        ]
        .iter()
        .enumerate()
        {
            part_types.insert(name.to_string(), i as u16);
            registry
                .register_part_type(&admin_cap, &admin, name)
                .await?;
        }

        // let mut batch = vec![];

        // Register torsos
        let parts = vec![part_types["head"], part_types["robot_leg_base"]];
        let ty = part_types["torso"];
        registry
            .batch_register(
                &admin_cap,
                &admin,
                vec![
                    MonsterPartTemplate::new(
                        &"BoxRobotBody",
                        ty,
                        Rarity::Common,
                        vec![],
                        parts.clone(),
                    ),
                    MonsterPartTemplate::new(
                        &"RoundRobotBody",
                        ty,
                        Rarity::Common,
                        vec![],
                        parts.clone(),
                    ),
                ],
            )
            .await?;

        let parts = vec![part_types["eye"], part_types["addon"]];
        let ty = part_types["head"];
        registry
            .batch_register(
                &admin_cap,
                &admin,
                vec![
                    MonsterPartTemplate::new(
                        &"BoxRobotHead",
                        ty,
                        Rarity::Common,
                        vec![],
                        parts.clone(),
                    ),
                    MonsterPartTemplate::new(
                        &"PyramidRobotHead",
                        ty,
                        Rarity::Common,
                        vec![],
                        parts.clone(),
                    ),
                    MonsterPartTemplate::new(
                        &"SphereRobotHead",
                        ty,
                        Rarity::Common,
                        vec![],
                        parts.clone(),
                    ),
                ],
            )
            .await?;

        let parts = vec![];
        let ty = part_types["eye"];
        registry
            .batch_register(
                &admin_cap,
                &admin,
                vec![
                    MonsterPartTemplate::new(
                        &"CameraRobotEye",
                        ty,
                        Rarity::Common,
                        vec![],
                        parts.clone(),
                    ),
                    MonsterPartTemplate::new(
                        &"FeelerRobotEye",
                        ty,
                        Rarity::Common,
                        vec![],
                        parts.clone(),
                    ),
                    MonsterPartTemplate::new(
                        &"SleekCameraRobotEye",
                        ty,
                        Rarity::Common,
                        vec![],
                        parts.clone(),
                    ),
                ],
            )
            .await?;

        let parts = vec![];
        let ty = part_types["addon"];
        registry
            .batch_register(
                &admin_cap,
                &admin,
                vec![
                    MonsterPartTemplate::new(
                        &"AntennaRobotAddon",
                        ty,
                        Rarity::Common,
                        vec![],
                        parts.clone(),
                    ),
                    MonsterPartTemplate::new(&"Empty", ty, Rarity::Common, vec![], parts.clone()),
                ],
            )
            .await?;

        let parts = vec![part_types["robot_leg_tibia"]];
        let ty = part_types["robot_leg_base"];
        registry
            .batch_register(
                &admin_cap,
                &admin,
                vec![
                    MonsterPartTemplate::new(
                        &"HexagonalRobotLegBase",
                        ty,
                        Rarity::Common,
                        vec![],
                        parts.clone(),
                    ),
                    MonsterPartTemplate::new(
                        &"CrownedRobotLegBase",
                        ty,
                        Rarity::Common,
                        vec![],
                        parts.clone(),
                    ),
                ],
            )
            .await?;

        let parts = vec![part_types["robot_leg_end"], part_types["robot_leg_joint"]];
        let ty = part_types["robot_leg_tibia"];
        registry
            .batch_register(
                &admin_cap,
                &admin,
                vec![
                    MonsterPartTemplate::new(
                        &"StraightRobotLegTibia",
                        ty,
                        Rarity::Common,
                        vec![],
                        parts.clone(),
                    ),
                    MonsterPartTemplate::new(
                        &"MiddleRobotLegTibia",
                        ty,
                        Rarity::Common,
                        vec![],
                        parts.clone(),
                    ),
                    MonsterPartTemplate::new(
                        &"SideRobotLegTibia",
                        ty,
                        Rarity::Common,
                        vec![],
                        parts.clone(),
                    ),
                ],
            )
            .await?;

        let parts = vec![];
        let ty = part_types["robot_leg_joint"];
        registry
            .batch_register(
                &admin_cap,
                &admin,
                vec![
                    MonsterPartTemplate::new(
                        &"HexagonalRobotLegJoint",
                        ty,
                        Rarity::Common,
                        vec![],
                        parts.clone(),
                    ),
                    MonsterPartTemplate::new(
                        &"CrossRobotLegJoint",
                        ty,
                        Rarity::Common,
                        vec![],
                        parts.clone(),
                    ),
                    MonsterPartTemplate::new(
                        &"StarRobotLegJoint",
                        ty,
                        Rarity::Common,
                        vec![],
                        parts.clone(),
                    ),
                ],
            )
            .await?;

        let parts = vec![];
        let ty = part_types["robot_leg_end"];
        registry
            .batch_register(
                &admin_cap,
                &admin,
                vec![MonsterPartTemplate::new(
                    &"SideRobotLegEnd",
                    ty,
                    Rarity::Common,
                    vec![],
                    parts.clone(),
                )],
            )
            .await?;

        debug!("Registry took {} seconds", time.elapsed().as_secs());

        Ok(())
    }
}

fn generate_address() -> SuiKeyPair {
    SuiKeyPair::Secp256r1(get_key_pair_from_rng(&mut rand::rngs::OsRng).1)
}

pub struct DaemonConfig {
    pub admin_cap: AdminCap,
    pub registry: MonsterRegistry,
    pub monster_minter: MonsterMinter,
    pub drive_minter: EncryptedDriveMinter,
    pub distribution_table: DistributionTable,
}
