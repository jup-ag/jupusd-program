use anchor_lang::prelude::*;

use crate::{
    error::JupStableError,
    state::{
        config::{Config, PEG_PRICE_DECIMALS},
        operator::{Operator, OperatorRole},
    },
};

#[derive(Accounts)]
pub struct ManageConfig<'info> {
    #[account(mut)]
    pub operator_authority: Signer<'info>,
    #[account(
        has_one = operator_authority @ JupStableError::NotAuthorized,
    )]
    pub operator: AccountLoader<'info, Operator>,
    #[account(mut)]
    pub config: AccountLoader<'info, Config>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub enum ConfigManagementAction {
    Pause,
    UpdatePauseFlag {
        is_mint_redeem_enabled: bool,
    },
    UpdatePeriodLimit {
        index: u8,
        duration_seconds: u64,
        max_mint_amount: u64,
        max_redeem_amount: u64,
    },
    ResetPeriodLimit {
        index: u8,
    },
    SetPegPriceUSD {
        peg_price_usd: u64,
    },
}

pub fn manage_config(ctx: Context<ManageConfig>, action: ConfigManagementAction) -> Result<()> {
    let mut config = ctx.accounts.config.load_mut()?;
    let operator = ctx.accounts.operator.load()?;

    match action {
        ConfigManagementAction::Pause => {
            operator.is(OperatorRole::GlobalDisabler)?;
            require!(
                config.is_mint_redeem_enabled(),
                JupStableError::ProtocolPaused
            );

            config.update_mint_redeem_enabled(false);
        },
        ConfigManagementAction::UpdatePauseFlag {
            is_mint_redeem_enabled,
        } => {
            operator.is(OperatorRole::Admin)?;

            config.update_mint_redeem_enabled(is_mint_redeem_enabled);
        },
        ConfigManagementAction::UpdatePeriodLimit {
            index,
            duration_seconds,
            max_mint_amount,
            max_redeem_amount,
        } => {
            operator.is(OperatorRole::PeriodManager)?;

            let current_time = Clock::get()?.unix_timestamp;
            config.update_period_limit(
                index as usize,
                duration_seconds,
                max_mint_amount,
                max_redeem_amount,
                current_time,
            )?;
        },
        ConfigManagementAction::ResetPeriodLimit { index } => {
            operator.is(OperatorRole::PeriodManager)?;

            config.reset_period_limit(index.into())?;
        },
        ConfigManagementAction::SetPegPriceUSD { peg_price_usd } => {
            operator.is(OperatorRole::PegManager)?;

            require!(peg_price_usd > 0, JupStableError::InvalidPegPriceUSD);
            require!(
                peg_price_usd < 2 * 10_u64.pow(PEG_PRICE_DECIMALS),
                JupStableError::InvalidPegPriceUSD
            );

            config.set_peg_price_usd(peg_price_usd);
        },
    }

    Ok(())
}
