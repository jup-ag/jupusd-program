#![allow(unexpected_cfgs)]

use anchor_lang::prelude::*;

pub mod error;
pub mod instructions;
pub mod oracle;
pub mod state;

declare_id!("JUPUSDecMzAVgztLe6eGhwUBj1Pn3j9WAXwmtHmfbRr");

use crate::{
    instructions::{
        BenefactorManagementAction, ConfigManagementAction, OperatorManagementAction,
        VaultManagementAction, *,
    },
    state::operator::OperatorRole,
};

#[cfg(not(feature = "no-entrypoint"))]
solana_security_txt::security_txt! {
    name: "Jupiter Stable",
`    project_url: "https://jupusd.money/",
    contacts: "https://security.raccoons.dev/submit/",
    policy: "https://security.raccoons.dev/",
    preferred_languages: "en",
    auditors: "Offside Labs, Guardian, Pashov"
}

#[program]
pub mod jup_stable {
    use super::*;

    pub fn init(
        ctx: Context<Init>,
        decimals: u8,
        name: String,
        symbol: String,
        uri: String,
    ) -> Result<()> {
        instructions::init(ctx, decimals, name, symbol, uri)
    }
    pub fn manage_config(ctx: Context<ManageConfig>, action: ConfigManagementAction) -> Result<()> {
        instructions::manage_config(ctx, action)
    }

    pub fn create_operator(ctx: Context<CreateOperator>, role: OperatorRole) -> Result<()> {
        instructions::create_operator(ctx, role)
    }

    pub fn manage_operator(
        ctx: Context<ManageOperator>,
        action: OperatorManagementAction,
    ) -> Result<()> {
        instructions::manage_operator(ctx, action)
    }
    pub fn create_vault(ctx: Context<CreateVault>) -> Result<()> { instructions::create_vault(ctx) }

    pub fn manage_vault(ctx: Context<ManageVault>, action: VaultManagementAction) -> Result<()> {
        instructions::manage_vault(ctx, action)
    }

    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        instructions::withdraw(ctx, amount)
    }

    pub fn create_benefactor(
        ctx: Context<CreateBenefactor>,
        mint_fee_rate: u16,
        redeem_fee_rate: u16,
    ) -> Result<()> {
        instructions::create_benefactor(ctx, mint_fee_rate, redeem_fee_rate)
    }

    pub fn delete_operator(ctx: Context<DeleteOperator>) -> Result<()> {
        instructions::delete_operator(ctx)
    }

    pub fn manage_benefactor(
        ctx: Context<ManageBenefactor>,
        action: BenefactorManagementAction,
    ) -> Result<()> {
        instructions::manage_benefactor(ctx, action)
    }

    pub fn delete_benefactor(ctx: Context<DeleteBenefactor>) -> Result<()> {
        instructions::delete_benefactor(ctx)
    }

    // User Instructions
    pub fn mint(ctx: Context<Mint>, amount: u64, min_amount_out: u64) -> Result<()> {
        instructions::mint(ctx, amount, min_amount_out)
    }

    pub fn redeem(ctx: Context<Redeem>, amount: u64, min_amount_out: u64) -> Result<()> {
        instructions::redeem(ctx, amount, min_amount_out)
    }
}
