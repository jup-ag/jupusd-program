use anchor_lang::prelude::*;

use crate::{
    error::JupStableError,
    state::{
        benefactor::{Benefactor, BenefactorStatus, BENEFACTOR_PREFIX},
        operator::{Operator, OperatorRole},
    },
};

#[derive(Accounts)]
pub struct CreateBenefactor<'info> {
    pub operator_authority: Signer<'info>,
    #[account(
        has_one = operator_authority @ JupStableError::NotAuthorized,
    )]
    pub operator: AccountLoader<'info, Operator>,

    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK:
    pub benefactor_authority: UncheckedAccount<'info>,

    #[account(
        init,
        payer = payer,
        space = 8 + Benefactor::MAX_SIZE,
        seeds = [BENEFACTOR_PREFIX, benefactor_authority.key().as_ref()],
        bump
    )]
    pub benefactor: AccountLoader<'info, Benefactor>,

    pub system_program: Program<'info, System>,
}

pub fn create_benefactor(
    ctx: Context<CreateBenefactor>,
    mint_fee_rate: u16,
    redeem_fee_rate: u16,
) -> Result<()> {
    let operator = ctx.accounts.operator.load()?;
    operator.is(OperatorRole::BenefactorManager)?;

    require!(mint_fee_rate <= 10000, JupStableError::InvalidFeeRate);
    require!(redeem_fee_rate <= 10000, JupStableError::InvalidFeeRate);

    let mut benefactor = ctx.accounts.benefactor.load_init()?;

    *benefactor = Benefactor {
        authority: ctx.accounts.benefactor_authority.key(),
        status: BenefactorStatus::Disabled,
        mint_fee_rate,
        redeem_fee_rate,
        ..Default::default()
    };

    Ok(())
}

#[derive(Accounts)]
pub struct ManageBenefactor<'info> {
    #[account(mut)]
    pub operator_authority: Signer<'info>,
    #[account(
        has_one = operator_authority @ JupStableError::NotAuthorized,
    )]
    pub operator: AccountLoader<'info, Operator>,

    #[account(mut)]
    pub benefactor: AccountLoader<'info, Benefactor>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub enum BenefactorManagementAction {
    Disable,
    SetStatus {
        status: BenefactorStatus,
    },
    UpdateFeeRates {
        mint_fee_rate: u16,
        redeem_fee_rate: u16,
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
}

pub fn manage_benefactor(
    ctx: Context<ManageBenefactor>,
    action: BenefactorManagementAction,
) -> Result<()> {
    let mut benefactor = ctx.accounts.benefactor.load_mut()?;
    let operator = ctx.accounts.operator.load()?;

    match action {
        BenefactorManagementAction::Disable => {
            operator.is(OperatorRole::BenefactorDisabler)?;

            benefactor.is_active()?;
            benefactor.status = BenefactorStatus::Disabled;
        },
        BenefactorManagementAction::SetStatus { status } => {
            operator.is(OperatorRole::BenefactorManager)?;

            benefactor.set_status(status);
        },
        BenefactorManagementAction::UpdateFeeRates {
            mint_fee_rate,
            redeem_fee_rate,
        } => {
            operator.is(OperatorRole::BenefactorManager)?;

            require!(mint_fee_rate <= 10000, JupStableError::InvalidFeeRate);
            require!(redeem_fee_rate <= 10000, JupStableError::InvalidFeeRate);

            benefactor.mint_fee_rate = mint_fee_rate;
            benefactor.redeem_fee_rate = redeem_fee_rate;
        },
        BenefactorManagementAction::UpdatePeriodLimit {
            index,
            duration_seconds,
            max_mint_amount,
            max_redeem_amount,
        } => {
            operator.is(OperatorRole::PeriodManager)?;

            let current_time = Clock::get()?.unix_timestamp;
            benefactor.update_period_limit(
                index as usize,
                duration_seconds,
                max_mint_amount,
                max_redeem_amount,
                current_time,
            )?;
        },
        BenefactorManagementAction::ResetPeriodLimit { index } => {
            operator.is(OperatorRole::PeriodManager)?;

            benefactor.reset_period_limit(index.into())?;
        },
    }

    Ok(())
}

#[derive(Accounts)]
pub struct DeleteBenefactor<'info> {
    #[account(mut)]
    pub operator_authority: Signer<'info>,
    #[account(
        has_one = operator_authority @ JupStableError::NotAuthorized,
    )]
    pub operator: AccountLoader<'info, Operator>,

    #[account(mut)]
    /// CHECK: Will only receive rent
    pub receiver: UncheckedAccount<'info>,

    #[account(
        mut,
        close = receiver,
    )]
    pub benefactor: AccountLoader<'info, Benefactor>,
}

pub fn delete_benefactor(ctx: Context<DeleteBenefactor>) -> Result<()> {
    let operator = ctx.accounts.operator.load()?;
    operator.is(OperatorRole::BenefactorManager)?;
    Ok(())
}
