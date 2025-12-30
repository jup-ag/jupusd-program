use std::mem::size_of;

use anchor_lang::prelude::*;
use bytemuck::{Pod, Zeroable};
use static_assertions::const_assert_eq;

use crate::{error::JupStableError, state::common::PeriodLimit};

const_assert_eq!(Benefactor::MAX_SIZE, size_of::<Benefactor>());

pub const BENEFACTOR_PREFIX: &[u8; 10] = b"benefactor";
pub const MAX_PERIOD_LIMIT: usize = 4;

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, AnchorSerialize, AnchorDeserialize)]
pub enum BenefactorStatus {
    Active,
    Disabled,
}

unsafe impl Pod for BenefactorStatus {}
unsafe impl Zeroable for BenefactorStatus {}

#[account(zero_copy)]
pub struct Benefactor {
    pub authority: Pubkey,
    pub status: BenefactorStatus,
    pub _padding0: [u8; 7],

    pub mint_fee_rate: u16,
    pub redeem_fee_rate: u16,
    pub _padding1: [u8; 4],

    pub period_limits: [PeriodLimit; MAX_PERIOD_LIMIT],

    pub total_minted: [u8; 16],
    pub total_redeemed: [u8; 16],

    pub reserved: [u8; 256],
}

impl Default for Benefactor {
    fn default() -> Self {
        Benefactor {
            authority: Pubkey::default(),
            status: BenefactorStatus::Disabled,
            _padding0: [0; 7],
            mint_fee_rate: 0,
            redeem_fee_rate: 0,
            _padding1: [0; 4],
            period_limits: [PeriodLimit::default(); MAX_PERIOD_LIMIT],
            total_minted: [0; 16],
            total_redeemed: [0; 16],
            reserved: [0; 256],
        }
    }
}

impl Benefactor {
    pub const MAX_SIZE: usize = 32 + // authority
        1 + 7 + // status + padding
        2 + 2 + 4 + // fee rates (2 u16 fields) + padding
        PeriodLimit::MAX_SIZE * MAX_PERIOD_LIMIT + // rate limit windows
        16 + 16 + // total stats
        256;

    pub fn is_active(&self) -> Result<()> {
        require!(
            self.status == BenefactorStatus::Active,
            JupStableError::BenefactorDisabled
        );
        Ok(())
    }

    pub fn is_disabled(&self) -> Result<()> {
        require!(
            self.status == BenefactorStatus::Disabled,
            JupStableError::BenefactorActive
        );
        Ok(())
    }

    pub fn set_status(&mut self, status: BenefactorStatus) { self.status = status; }

    pub fn can_mint(&mut self, amount: u64, current_time: i64) -> Result<()> {
        self.is_active()?;

        for window in &mut self.period_limits {
            window.roll_window(current_time);
            window.check_mint_limit(amount)?;
        }

        Ok(())
    }

    pub fn can_redeem(&mut self, amount: u64, current_time: i64) -> Result<()> {
        self.is_active()?;

        for window in &mut self.period_limits {
            window.roll_window(current_time);
            window.check_redeem_limit(amount)?;
        }

        Ok(())
    }

    pub fn calculate_mint_fee(&self, amount: u64) -> u64 {
        (amount as u128 * self.mint_fee_rate as u128).div_ceil(10000) as u64
    }

    pub fn calculate_redeem_fee(&self, amount: u64) -> u64 {
        (amount as u128 * self.redeem_fee_rate as u128).div_ceil(10000) as u64
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
