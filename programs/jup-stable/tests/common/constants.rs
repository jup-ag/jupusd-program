use hex_literal::hex;
use jup_stable::instructions::OracleConfig;
use solana_sdk::{pubkey, pubkey::Pubkey};

pub const USDC_MINT: Pubkey = pubkey!("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
pub const USDC_DECIMALS: u8 = 6;
pub const USDC_PRICE_ACCOUNT: Pubkey = pubkey!("Dpw1EAVrSB1ibxiDQyTAW6Zip3J4Btk2x4SgApQCeFbX");
pub const USDC_FEED_ID: [u8; 32] =
    hex!("eaa020c61cc479712813461ce153894a96a6c00b21ed0cfc2798d1f9a9e9c94a");

pub const JUPUSD_NAME: &str = "Jupiter USD";
pub const JUPUSD_SYMBOL: &str = "JUPUSD";
pub const JUPUSD_URI: &str = "https://jup.ag/jupusd";
pub const JUPUSD_DECIMALS: u8 = 6;

pub const USDC_ORACLE_CONFIG: OracleConfig = OracleConfig::Pyth(USDC_FEED_ID, USDC_PRICE_ACCOUNT);
