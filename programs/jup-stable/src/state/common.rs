use anchor_lang::prelude::*;
use bytemuck::{Pod, Zeroable};

use crate::error::JupStableError;

pub const MAX_DURATION_SECONDS: u64 = 86400 * 30; // 30 days
pub const MIN_DURATION_SECONDS: u64 = 30; // 30 seconds

#[repr(C)]
#[derive(Default, Clone, Copy, AnchorDeserialize, AnchorSerialize)]
pub struct PeriodLimit {
    /// Window duration in seconds (0 = disabled)
    pub duration_seconds: u64,
    /// Maximum mint amount in this window
    pub max_mint_amount: u64,
    /// Maximum redeem amount in this window
    pub max_redeem_amount: u64,
    /// Amount minted in current window
    pub minted_amount: u64,
    /// Amount redeemed in current window
    pub redeemed_amount: u64,
    /// Window start timestamp
    pub window_start: i64,
}

unsafe impl Pod for PeriodLimit {}
unsafe impl Zeroable for PeriodLimit {}

impl PeriodLimit {
    pub const MAX_SIZE: usize = 8 + 8 + 8 + 8 + 8 + 8;

    pub fn is_valid(&self) -> bool {
        self.duration_seconds >= MIN_DURATION_SECONDS
            && self.duration_seconds <= MAX_DURATION_SECONDS
            && self.max_mint_amount > 0
            && self.max_redeem_amount > 0
    }

    pub fn update(
        &mut self,
        duration_seconds: u64,
        max_mint_amount: u64,
        max_redeem_amount: u64,
        current_time: i64,
    ) -> Result<()> {
        self.duration_seconds = duration_seconds;
        self.max_mint_amount = max_mint_amount;
        self.max_redeem_amount = max_redeem_amount;
        self.minted_amount = 0;
        self.redeemed_amount = 0;
        self.window_start = current_time;

        require!(self.is_valid(), JupStableError::InvalidPeriodLimit);

        Ok(())
    }

    pub fn roll_window(&mut self, current_time: i64) {
        if self.duration_seconds == 0 {
            return;
        }

        let window_elapsed = current_time - self.window_start;
        if window_elapsed >= self.duration_seconds as i64 {
            self.minted_amount = 0;
            self.redeemed_amount = 0;
            self.window_start = current_time;
        }
    }

    pub fn check_mint_limit(&mut self, amount: u64) -> Result<()> {
        if self.duration_seconds == 0 {
            return Ok(());
        }

        if self.minted_amount + amount > self.max_mint_amount {
            return err!(JupStableError::MintLimitExceeded);
        }

        Ok(())
    }

    pub fn check_redeem_limit(&mut self, amount: u64) -> Result<()> {
        if self.duration_seconds == 0 {
            return Ok(());
        }

        if self.redeemed_amount + amount > self.max_redeem_amount {
            return err!(JupStableError::RedeemLimitExceeded);
        }

        Ok(())
    }

    pub fn record_mint(&mut self, amount: u64) {
        if self.duration_seconds == 0 {
            return;
        }

        self.minted_amount += amount;
    }

    pub fn record_redeem(&mut self, amount: u64) {
        if self.duration_seconds == 0 {
            return;
        }

        self.redeemed_amount += amount;
    }

    pub fn reset(&mut self) { *self = Self::default(); }
}
