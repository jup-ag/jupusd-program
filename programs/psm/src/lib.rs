#![allow(unexpected_cfgs)]

use anchor_lang::prelude::*;

pub mod error;
pub mod instructions;
pub mod state;

declare_id!("GFU42W56UJ4ZyJL8beMWjtiz3LhbxXMBbHinft6Jc5SC");

use crate::instructions::{ConfigManagementAction, *};

#[program]
pub mod psm {
    use super::*;

    pub fn init(ctx: Context<Init>) -> Result<()> {
        instructions::init(ctx)?;
        Ok(())
    }

    pub fn manage_config(ctx: Context<ManageConfig>, action: ConfigManagementAction) -> Result<()> {
        instructions::manage_config(ctx, action)?;
        Ok(())
    }

    pub fn create_pool(ctx: Context<CreatePool>) -> Result<()> {
        instructions::create_pool(ctx)?;
        Ok(())
    }

    pub fn manage_pool(ctx: Context<ManagePool>, action: PoolManagementAction) -> Result<()> {
        instructions::manage_pool(ctx, action)?;
        Ok(())
    }

    pub fn supply(ctx: Context<Supply>, amount: u64) -> Result<()> {
        instructions::supply(ctx, amount)?;
        Ok(())
    }

    pub fn redeem(ctx: Context<Redeem>, amount: u64) -> Result<()> {
        instructions::redeem(ctx, amount)?;
        Ok(())
    }

    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        instructions::withdraw(ctx, amount)?;
        Ok(())
    }
}
