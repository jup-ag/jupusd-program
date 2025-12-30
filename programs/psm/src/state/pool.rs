use std::mem::size_of;

use anchor_lang::prelude::*;
use bytemuck::{Pod, Zeroable};
use static_assertions::const_assert_eq;

use crate::error::PSmError;

const_assert_eq!(Pool::MAX_SIZE, size_of::<Pool>());

pub const POOL_PREFIX: &[u8; 4] = b"pool";
pub const POOL_REDEMPTION_TOKEN_ACCOUNT_PREFIX: &[u8; 29] = b"pool_redemption_token_account";
pub const POOL_SETTLEMENT_TOKEN_ACCOUNT_PREFIX: &[u8; 29] = b"pool_settlement_token_account";

#[macro_export]
macro_rules! pool_seeds {
    ($mint:expr, $bump:expr) => {
        &[POOL_PREFIX, $mint.as_ref(), &[$bump]]
    };
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, AnchorSerialize, AnchorDeserialize)]
pub enum PoolStatus {
    Active,
    Paused,
    Disabled,
}

unsafe impl Pod for PoolStatus {}
unsafe impl Zeroable for PoolStatus {}

#[account(zero_copy)]
pub struct Pool {
    pub redemption_mint: Pubkey,
    pub settlement_mint: Pubkey,
    pub redemption_token_account: Pubkey,
    pub settlement_token_account: Pubkey,
    pub redemption_token_program: Pubkey,
    pub settlement_token_program: Pubkey,

    pub redemption_token_decimals: u8,
    pub settlement_token_decimals: u8,
    pub _padding1: [u8; 6],

    pub status: PoolStatus,
    pub _padding2: [u8; 7],

    pub bump: u8,
    pub _padding3: [u8; 7],

    pub total_redeemed: [u8; 16],
    pub total_supplied: [u8; 16],
    pub total_withdrawn: [u8; 16],

    pub reserved: [u8; 256],
}

impl Default for Pool {
    fn default() -> Self {
        Pool {
            redemption_mint: Pubkey::default(),
            settlement_mint: Pubkey::default(),
            redemption_token_account: Pubkey::default(),
            settlement_token_account: Pubkey::default(),
            redemption_token_program: Pubkey::default(),
            settlement_token_program: Pubkey::default(),
            redemption_token_decimals: 0,
            settlement_token_decimals: 0,
            _padding1: [0; 6],
            status: PoolStatus::Disabled,
            _padding2: [0; 7],
            bump: 0,
            _padding3: [0; 7],
            total_redeemed: [0; 16],
            total_supplied: [0; 16],
            total_withdrawn: [0; 16],
            reserved: [0; 256],
        }
    }
}

impl Pool {
    pub const MAX_SIZE: usize = 32 + // redemption_mint
        32 + // settlement_mint
        32 + // redemption_token_account
        32 + // settlement_token_account
        32 + // redemption_token_program
        32 + // settlement_token_program
        1 + // redemption_token_decimals
        1 + // settlement_token_decimals
        6 + // _padding1
        1 + // status (enum)
        7 + // _padding2
        1 + // bump
        7 + // _padding3
        16 + // total_redeemed
        16 + // total_supplied
        16 + // total_withdrawn
        256;

    pub fn is_active(&self) -> bool { self.status == PoolStatus::Active }

    pub fn set_status(&mut self, status: PoolStatus) { self.status = status; }

    pub fn record_total_redeemed(&mut self, amount: u64) {
        let mut fake_u128 = u128::from_le_bytes(self.total_redeemed);
        fake_u128 += amount as u128;
        self.total_redeemed = fake_u128.to_le_bytes();
    }

    pub fn record_total_supplied(&mut self, amount: u64) {
        let mut fake_u128 = u128::from_le_bytes(self.total_supplied);
        fake_u128 += amount as u128;
        self.total_supplied = fake_u128.to_le_bytes();
    }

    pub fn record_total_withdrawn(&mut self, amount: u64) {
        let mut fake_u128 = u128::from_le_bytes(self.total_withdrawn);
        fake_u128 += amount as u128;
        self.total_withdrawn = fake_u128.to_le_bytes();
    }

    pub fn record_withdraw(&mut self, amount: u64) { self.record_total_withdrawn(amount); }

    pub fn record_redeem(&mut self, amount: u64) { self.record_total_redeemed(amount); }

    pub fn record_supply(&mut self, amount: u64) { self.record_total_supplied(amount); }

    pub fn can_redeem(&mut self) -> Result<bool> {
        if !self.is_active() {
            return err!(PSmError::PoolNotActive);
        }

        Ok(true)
    }

    pub fn can_withdraw(&mut self) -> Result<bool> {
        if !self.is_active() {
            return err!(PSmError::PoolNotActive);
        }

        Ok(true)
    }

    pub fn can_supply(&mut self) -> Result<bool> {
        if !self.is_active() {
            return err!(PSmError::PoolNotActive);
        }

        Ok(true)
    }
}
