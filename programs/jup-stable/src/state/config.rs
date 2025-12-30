use std::mem::size_of;

use anchor_lang::prelude::*;
use static_assertions::const_assert_eq;

use crate::{error::JupStableError, state::common::PeriodLimit};

const_assert_eq!(Config::MAX_SIZE, size_of::<Config>());
const_assert_eq!(size_of::<Config>() % 8, 0);

pub const CONFIG_PREFIX: &[u8; 6] = b"config";
pub const AUTHORITY_PREFIX: &[u8; 9] = b"authority";
pub const MAX_PERIOD_LIMIT: usize = 4;
pub const PEG_PRICE_DECIMALS: u32 = 4;

#[macro_export]
macro_rules! authority_seeds {
    ($bump:expr) => {
        &[AUTHORITY_PREFIX, &[$bump]]
    };
}

#[account(zero_copy)]
pub struct Config {
    pub mint: Pubkey,
    pub authority: Pubkey,
    pub token_program: Pubkey,
    pub period_limits: [PeriodLimit; MAX_PERIOD_LIMIT],
    pub peg_price_usd: u64,
    pub decimals: u8,
    pub is_mint_redeem_enabled: u8,
    pub authority_bump: u8,
    pub config_bump: u8,
    pub _padding: [u8; 4],
    pub reserved: [u8; 192],
}

impl Default for Config {
    fn default() -> Self {
        Config {
            mint: Pubkey::default(),
            authority: Pubkey::default(),
            token_program: Pubkey::default(),
            period_limits: [PeriodLimit::default(); MAX_PERIOD_LIMIT],
            peg_price_usd: 10000,
            decimals: 0,
            is_mint_redeem_enabled: 0,
            authority_bump: 0,
            config_bump: 0,
            _padding: [0; 4],
            reserved: [0; 192],
        }
    }
}
impl Config {
    pub const MAX_SIZE: usize =
        32 + 32 + 32 + PeriodLimit::MAX_SIZE * MAX_PERIOD_LIMIT + 8 + 1 + 1 + 1 + 1 + 4 + 192;

    pub fn is_mint_redeem_enabled(&self) -> bool { self.is_mint_redeem_enabled == 1 }

    pub fn set_peg_price_usd(&mut self, peg_price_usd: u64) { self.peg_price_usd = peg_price_usd; }

    pub fn update_mint_redeem_enabled(&mut self, is_mint_redeem_enabled: bool) {
        self.is_mint_redeem_enabled = if is_mint_redeem_enabled { 1 } else { 0 };
    }

    pub fn can_mint(&mut self, amount: u64, current_time: i64) -> Result<()> {
        if !self.is_mint_redeem_enabled() {
            return err!(JupStableError::ProtocolPaused);
        }

        for window in &mut self.period_limits {
            window.roll_window(current_time);
            window.check_mint_limit(amount)?;
        }

        Ok(())
    }

    pub fn can_redeem(&mut self, amount: u64, current_time: i64) -> Result<()> {
        if !self.is_mint_redeem_enabled() {
            return err!(JupStableError::ProtocolPaused);
        }

        for window in &mut self.period_limits {
            window.roll_window(current_time);
            window.check_redeem_limit(amount)?;
        }

        Ok(())
    }

    pub fn record_mint(&mut self, amount: u64) {
        for window in &mut self.period_limits {
            window.record_mint(amount);
        }
    }

    pub fn record_redeem(&mut self, amount: u64) {
        for window in &mut self.period_limits {
            window.record_redeem(amount);
        }
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
}
