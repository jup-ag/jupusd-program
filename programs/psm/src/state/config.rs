use std::mem::size_of;

use anchor_lang::prelude::*;
use static_assertions::const_assert_eq;

use crate::error::PSmError;

const_assert_eq!(Config::MAX_SIZE, size_of::<Config>());
const_assert_eq!(size_of::<Config>() % 8, 0);

pub const CONFIG_PREFIX: &[u8; 6] = b"config";
pub const AUTHORITY_PREFIX: &[u8; 9] = b"authority";
pub const MAX_ADMINS: usize = 10;
pub const MAX_PERIOD_LIMIT: usize = 4;

#[macro_export]
macro_rules! config_seeds {
    ($mint:expr, $bump:expr) => {
        &[CONFIG_PREFIX, &[$bump]]
    };
}

#[macro_export]
macro_rules! authority_seeds {
    ($bump:expr) => {
        &[AUTHORITY_PREFIX, &[$bump]]
    };
}

#[account(zero_copy)]
pub struct Config {
    pub admins: [Pubkey; MAX_ADMINS],
    pub authority: Pubkey,
    pub is_paused: u8,
    pub authority_bump: u8,
    pub config_bump: u8,
    pub _padding: [u8; 5],
    pub reserved: [u8; 192],
}

impl Config {
    pub const MAX_SIZE: usize = 32 * MAX_ADMINS + 32 + 1 + 1 + 1 + 5 + 192;

    pub fn is_admin(&self, pubkey: &Pubkey) -> bool {
        for i in 0..MAX_ADMINS {
            if &self.admins[i] == pubkey {
                return true;
            }
        }
        false
    }

    pub fn add_admin(&mut self, pubkey: &Pubkey) -> Result<()> {
        for i in 0..MAX_ADMINS {
            if self.admins[i] == Pubkey::default() {
                self.admins[i] = *pubkey;
                return Ok(());
            }
        }
        err!(PSmError::AdminArrayFull)
    }

    pub fn remove_admin(&mut self, pubkey: &Pubkey) -> Result<()> {
        for i in 0..MAX_ADMINS {
            if &self.admins[i] == pubkey {
                self.admins[i] = Pubkey::default();
                return Ok(());
            }
        }
        err!(PSmError::SomeError)
    }

    pub fn num_admins(&self) -> usize {
        let mut count = 0;
        for i in 0..MAX_ADMINS {
            if self.admins[i] != Pubkey::default() {
                count += 1;
            }
        }
        count
    }

    pub fn is_paused(&self) -> bool { self.is_paused == 1 }

    pub fn update_pause_flag(&mut self, is_paused: bool) -> Result<()> {
        self.is_paused = if is_paused { 1 } else { 0 };
        Ok(())
    }
}
