use std::mem::size_of;

use anchor_lang::prelude::*;
use bytemuck::{Pod, Zeroable};
use rust_decimal::{prelude::ToPrimitive, Decimal};
use static_assertions::const_assert_eq;

use crate::{error::JupStableError, oracle::OraclePrice, state::common::PeriodLimit};

const_assert_eq!(Vault::MAX_SIZE, size_of::<Vault>());

pub const MAX_ORACLES: usize = 5;
pub const MAX_PERIOD_LIMIT: usize = 4;
pub const VAULT_PREFIX: &[u8; 5] = b"vault";
pub const ORACLE_PRICE_DECIMALS: u32 = 4;

#[macro_export]
macro_rules! vault_seeds {
    ($mint:expr, $bump:expr) => {
        &[VAULT_PREFIX, $mint.as_ref(), &[$bump]]
    };
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, AnchorSerialize, AnchorDeserialize)]
pub enum VaultStatus {
    Enabled,
    Disabled,
}

unsafe impl Pod for VaultStatus {}
unsafe impl Zeroable for VaultStatus {}

#[repr(C)]
#[derive(Debug, Default, Clone, Copy, AnchorDeserialize, AnchorSerialize)]
pub struct PythV2Oracle {
    pub feed_id: [u8; 32],
    pub account: Pubkey,
    pub reserved1: [u8; 32],
    pub reserved2: [u8; 24],
}

#[repr(C)]
#[derive(Debug, Default, Clone, Copy, AnchorDeserialize, AnchorSerialize)]
pub struct SwitchboardOnDemandOracle {
    pub account: Pubkey,
    pub reserved: [u8; 32],
    pub reserved1: [u8; 32],
    pub reserved2: [u8; 24],
}

#[repr(C)]
#[derive(Debug, Default, Clone, Copy, AnchorDeserialize, AnchorSerialize)]
pub struct DovesOracle {
    pub account: Pubkey,
    pub reserved1: [u8; 32],
    pub reserved2: [u8; 32],
    pub reserved3: [u8; 24],
}

#[repr(C)]
#[derive(Debug, Default, Clone, Copy, AnchorDeserialize, AnchorSerialize)]
pub struct EmptyOracle {
    pub reserved: [u8; 32],
    pub reserved1: [u8; 32],
    pub reserved2: [u8; 32],
    pub reserved3: [u8; 24],
}

#[repr(C, u8)]
#[derive(Debug, Copy, Clone, AnchorDeserialize, AnchorSerialize)]
pub enum OracleType {
    Empty(EmptyOracle),
    Pyth(PythV2Oracle),
    Doves(DovesOracle),
    SwitchboardOnDemand(SwitchboardOnDemandOracle),
}

unsafe impl Pod for OracleType {}
unsafe impl Zeroable for OracleType {}

impl OracleType {
    pub const MAX_SIZE: usize = 1 + 120;
}

#[account(zero_copy)]
pub struct Vault {
    pub mint: Pubkey,
    pub custodian: Pubkey,
    pub token_account: Pubkey,
    pub token_program: Pubkey,

    pub stalesness_threshold: u64,

    pub min_oracle_price_usd: u64,
    pub max_oracle_price_usd: u64,

    pub status: VaultStatus,
    pub _padding1: [u8; 7],

    pub bump: u8,
    pub decimals: u8,
    pub _padding2: [u8; 6],

    pub oracles: [OracleType; MAX_ORACLES],
    pub _padding3: [u8; 3],

    pub period_limits: [PeriodLimit; MAX_PERIOD_LIMIT],

    pub reserved1: [u8; 32],

    pub total_minted: [u8; 16],
    pub total_redeemed: [u8; 16],

    pub reserved: [u8; 256],
}

impl Default for Vault {
    fn default() -> Self {
        Vault {
            mint: Pubkey::default(),
            custodian: Pubkey::default(),
            token_account: Pubkey::default(),
            token_program: Pubkey::default(),
            stalesness_threshold: 300,
            min_oracle_price_usd: 5000,
            max_oracle_price_usd: 10000,
            status: VaultStatus::Disabled,
            _padding1: [0; 7],
            bump: 0,
            decimals: 0,
            _padding2: [0; 6],
            reserved1: [0; 32],
            oracles: [OracleType::Empty(Default::default()); MAX_ORACLES],
            _padding3: [0; 3],
            period_limits: [PeriodLimit::default(); MAX_PERIOD_LIMIT],
            total_minted: [0; 16],
            total_redeemed: [0; 16],
            reserved: [0; 256],
        }
    }
}

impl Vault {
    pub const MAX_SIZE: usize = 32 + // stablecoin_mint
        32 + // custodian
        32 + // token_account
        32 + // token_program
        8 + // stalesness_threshold
        8 + 8 + // min_oracle_price and max_oracle_price
        1 + // status (enum)
        7 + // _padding1
        1 + // bump
        1 + // decimals
        6 + // _padding2
        OracleType::MAX_SIZE * MAX_ORACLES + // oracles array
        3 + // _padding3
        32 + // reserved
        PeriodLimit::MAX_SIZE * MAX_PERIOD_LIMIT + // rate limit windows
        16 + 16 + // total stats
        256;

    // reserved

    pub fn is_enabled(&self) -> Result<()> {
        require!(
            self.status == VaultStatus::Enabled,
            JupStableError::VaultDisabled
        );
        Ok(())
    }

    pub fn is_disabled(&self) -> Result<()> {
        require!(
            self.status == VaultStatus::Disabled,
            JupStableError::VaultEnabled
        );
        Ok(())
    }

    pub fn set_min_oracle_price_usd(&mut self, min_oracle_price_usd: u64) {
        self.min_oracle_price_usd = min_oracle_price_usd;
    }

    pub fn set_max_oracle_price_usd(&mut self, max_oracle_price_usd: u64) {
        self.max_oracle_price_usd = max_oracle_price_usd;
    }

    pub fn validate_oracle_price(&self, oracle_price: &OraclePrice, is_mint: bool) -> Result<()> {
        let oracle_price_usd = (oracle_price.0 * Decimal::from(10_i64.pow(ORACLE_PRICE_DECIMALS)))
            .to_u64()
            .ok_or(JupStableError::MathOverflow)?;
        if is_mint {
        require!(
                oracle_price_usd >= self.min_oracle_price_usd,
                JupStableError::BadOracle
            );
        } else {
            require!(
                oracle_price_usd <= self.max_oracle_price_usd,
                JupStableError::BadOracle
            );
        }
        Ok(())
    }

    pub fn set_stalesness_threshold(&mut self, stalesness_threshold: u64) {
        self.stalesness_threshold = stalesness_threshold;
    }

    pub fn set_status(&mut self, status: VaultStatus) { self.status = status; }

    pub fn update_oracle(&mut self, index: usize, oracle: &OracleType) -> Result<()> {
        if index >= MAX_ORACLES {
            return err!(JupStableError::BadInput);
        }

        self.oracles[index] = *oracle;

        Ok(())
    }

    pub fn update_period_limit(
        &mut self,
        index: usize,
        duration_seconds: u64,
        max_mint_amount: u64,
        max_redeem_amount: u64,
        current_time: i64,
    ) -> Result<()> {
        if index >= MAX_PERIOD_LIMIT {
            return err!(JupStableError::BadInput);
        }

        self.period_limits[index].update(
            duration_seconds,
            max_mint_amount,
            max_redeem_amount,
            current_time,
        )?;

        Ok(())
    }

    pub fn reset_period_limit(&mut self, index: usize) -> Result<()> {
        if index >= MAX_PERIOD_LIMIT {
            return err!(JupStableError::BadInput);
        }

        self.period_limits[index].reset();

        Ok(())
    }

    pub fn can_mint(&mut self, amount: u64, current_time: i64) -> Result<()> {
        self.is_enabled()?;

        for window in &mut self.period_limits {
            window.roll_window(current_time);
            window.check_mint_limit(amount)?;
        }

        Ok(())
    }

    pub fn can_redeem(&mut self, amount: u64, current_time: i64) -> Result<()> {
        self.is_enabled()?;

        for window in &mut self.period_limits {
            window.roll_window(current_time);
            window.check_redeem_limit(amount)?;
        }

        Ok(())
    }

    pub fn record_total_minted(&mut self, amount: u64) {
        let mut fake_u128 = u128::from_le_bytes(self.total_minted);
        fake_u128 += amount as u128;
        self.total_minted = fake_u128.to_le_bytes();
    }

    pub fn record_total_redeemed(&mut self, amount: u64) {
        let mut fake_u128 = u128::from_le_bytes(self.total_redeemed);
        fake_u128 += amount as u128;
        self.total_redeemed = fake_u128.to_le_bytes();
    }

    pub fn record_mint(&mut self, amount: u64) {
        self.record_total_minted(amount);

        for window in &mut self.period_limits {
            window.record_mint(amount);
        }
    }

    pub fn record_redeem(&mut self, amount: u64) {
        self.record_total_redeemed(amount);

        for window in &mut self.period_limits {
            window.record_redeem(amount);
        }
    }
}
