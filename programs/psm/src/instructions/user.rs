use std::cmp::Ordering;

use anchor_lang::prelude::*;
use anchor_spl::token_interface::{
    transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked,
};

use crate::{
    authority_seeds,
    error::PSmError,
    state::{
        config::{Config, AUTHORITY_PREFIX},
        pool::Pool,
    },
};

#[derive(Accounts)]
pub struct Redeem<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        mut,
        token::mint = redemption_mint,
        token::authority = user,
    )]
    pub user_redemption_token_account: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(
        mut,
        token::mint = settlement_mint,
        token::authority = user,
    )]
    pub user_settlement_token_account: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        has_one = authority
    )]
    pub config: AccountLoader<'info, Config>,
    /// CHECK: checked with constraint
    pub authority: UncheckedAccount<'info>,
    #[account(mut)]
    pub settlement_mint: Box<InterfaceAccount<'info, Mint>>,
    #[account(mut)]
    pub redemption_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        mut,
        has_one = redemption_mint,
        has_one = settlement_mint,
        has_one = redemption_token_account,
        has_one = settlement_token_account,
        has_one = redemption_token_program,
        has_one = settlement_token_program,
    )]
    pub pool: AccountLoader<'info, Pool>,
    #[account(mut)]
    pub redemption_token_account: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(mut)]
    pub settlement_token_account: Box<InterfaceAccount<'info, TokenAccount>>,

    pub redemption_token_program: Interface<'info, TokenInterface>,
    pub settlement_token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

pub fn redeem(ctx: Context<Redeem>, amount: u64) -> Result<()> {
    require!(amount > 0, PSmError::ZeroAmount);
    let normalized_amount: u64 = normalize_amount(
        amount.into(),
        ctx.accounts.settlement_mint.decimals,
        ctx.accounts.redemption_mint.decimals,
    )?
    .try_into()?;
    require!(normalized_amount > 0, PSmError::ZeroAmount);
    require!(
        ctx.accounts.redemption_token_account.amount >= normalized_amount,
        PSmError::InsufficientPoolBalance
    );

    let mut pool = ctx.accounts.pool.load_mut()?;
    let config = ctx.accounts.config.load()?;

    require!(!config.is_paused(), PSmError::ProtocolPaused);
    pool.can_redeem()?;
    pool.record_redeem(amount);

    transfer_checked(
        ctx.accounts.deposit_settlement_tokens(),
        amount,
        ctx.accounts.settlement_mint.decimals,
    )?;

    transfer_checked(
        ctx.accounts
            .claim_redemption_tokens()
            .with_signer(&[authority_seeds!(config.authority_bump)]),
        normalized_amount,
        ctx.accounts.redemption_mint.decimals,
    )?;

    Ok(())
}

impl<'info> Redeem<'info> {
    fn deposit_settlement_tokens(&self) -> CpiContext<'_, '_, '_, 'info, TransferChecked<'info>> {
        let cpi_accounts = TransferChecked {
            from: self.user_settlement_token_account.to_account_info(),
            mint: self.settlement_mint.to_account_info(),
            to: self.settlement_token_account.to_account_info(),
            authority: self.user.to_account_info(),
        };
        let cpi_program = self.settlement_token_program.to_account_info();
        CpiContext::new(cpi_program, cpi_accounts)
    }

    fn claim_redemption_tokens(&self) -> CpiContext<'_, '_, '_, 'info, TransferChecked<'info>> {
        let cpi_accounts = TransferChecked {
            from: self.redemption_token_account.to_account_info(),
            mint: self.redemption_mint.to_account_info(),
            to: self.user_redemption_token_account.to_account_info(),
            authority: self.authority.to_account_info(),
        };
        let cpi_program = self.redemption_token_program.to_account_info();
        CpiContext::new(cpi_program, cpi_accounts)
    }
}

fn normalize_amount(amount: u128, decimals: u8, target_decimals: u8) -> Result<u128> {
    match decimals.cmp(&target_decimals) {
        Ordering::Equal => Ok(amount),
        Ordering::Less => {
            let diff = target_decimals - decimals;
            require!(diff <= 19, PSmError::MathOverflow);
            Ok(amount * 10u128.pow(diff.into()))
        },
        Ordering::Greater => {
            let diff = decimals - target_decimals;
            require!(diff <= 19, PSmError::MathOverflow);
            Ok(amount / 10u128.pow(diff.into()))
        },
    }
}
