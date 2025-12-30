use anchor_lang::prelude::*;
use anchor_spl::token_interface::{
    transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked,
};

use crate::{
    authority_seeds,
    error::PSmError,
    state::{
        config::{Config, AUTHORITY_PREFIX},
        pool::{
            Pool, PoolStatus, POOL_PREFIX, POOL_REDEMPTION_TOKEN_ACCOUNT_PREFIX,
            POOL_SETTLEMENT_TOKEN_ACCOUNT_PREFIX,
        },
    },
};

#[derive(Accounts)]
pub struct CreatePool<'info> {
    pub admin: Signer<'info>,
    #[account(mut)]
    pub payer: Signer<'info>,

    pub redemption_mint: Box<InterfaceAccount<'info, Mint>>,
    pub settlement_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        constraint = config.load()?.is_admin(admin.key) @ PSmError::NotAuthorized,
        constraint = config.load()?.authority == authority.key() @ PSmError::InvalidAuthority,
    )]
    pub config: AccountLoader<'info, Config>,
    /// CHECK: checked with constraint
    pub authority: UncheckedAccount<'info>,

    #[account(
        init,
        payer = payer,
        space = 8 + Pool::MAX_SIZE,
        seeds = [POOL_PREFIX, redemption_mint.key().as_ref(), settlement_mint.key().as_ref()],
        bump
    )]
    pub pool: AccountLoader<'info, Pool>,

    #[account(
        init,
        payer = payer,
        seeds = [POOL_REDEMPTION_TOKEN_ACCOUNT_PREFIX, pool.key().as_ref()],
        token::authority = authority,
        token::mint = redemption_mint,
        token::token_program = redemption_token_program,
        bump
    )]
    pub redemption_token_account: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        init,
        payer = payer,
        seeds = [POOL_SETTLEMENT_TOKEN_ACCOUNT_PREFIX, pool.key().as_ref()],
        token::authority = authority,
        token::mint = settlement_mint,
        token::token_program = settlement_token_program,
        bump
    )]
    pub settlement_token_account: Box<InterfaceAccount<'info, TokenAccount>>,

    pub redemption_token_program: Interface<'info, TokenInterface>,
    pub settlement_token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

pub fn create_pool(ctx: Context<CreatePool>) -> Result<()> {
    let mut pool = ctx.accounts.pool.load_init()?;

    pool.redemption_mint = ctx.accounts.redemption_mint.key();
    pool.settlement_mint = ctx.accounts.settlement_mint.key();
    pool.redemption_token_account = ctx.accounts.redemption_token_account.key();
    pool.settlement_token_account = ctx.accounts.settlement_token_account.key();
    pool.redemption_token_program = ctx.accounts.redemption_token_program.key();
    pool.settlement_token_program = ctx.accounts.settlement_token_program.key();
    pool.status = PoolStatus::Disabled;
    pool.redemption_token_decimals = ctx.accounts.redemption_mint.decimals;
    pool.settlement_token_decimals = ctx.accounts.settlement_mint.decimals;
    pool.bump = ctx.bumps.pool;

    require!(
        pool.redemption_token_decimals
            .abs_diff(pool.settlement_token_decimals)
            <= 19,
        PSmError::MathOverflow
    );

    Ok(())
}

#[derive(Accounts)]
pub struct ManagePool<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        constraint = config.load()?.is_admin(admin.key) @ PSmError::NotAuthorized,
    )]
    pub config: AccountLoader<'info, Config>,

    #[account(mut)]
    pub pool: AccountLoader<'info, Pool>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub enum PoolManagementAction {
    SetStatus { status: PoolStatus },
}

pub fn manage_pool(ctx: Context<ManagePool>, action: PoolManagementAction) -> Result<()> {
    let mut pool = ctx.accounts.pool.load_mut()?;

    match action {
        PoolManagementAction::SetStatus { status } => {
            pool.set_status(status);
        },
    }

    Ok(())
}

#[derive(Accounts)]
pub struct Supply<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(
        mut,
        token::mint = redemption_mint,
        token::authority = admin,
    )]
    pub admin_redemption_token_account: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        constraint = config.load()?.is_admin(admin.key) @ PSmError::NotAuthorized,
    )]
    pub config: AccountLoader<'info, Config>,
    #[account(mut)]
    pub redemption_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        mut,
        has_one = redemption_mint,
        has_one = redemption_token_account,
        has_one = redemption_token_program,
    )]
    pub pool: AccountLoader<'info, Pool>,
    #[account(mut)]
    pub redemption_token_account: Box<InterfaceAccount<'info, TokenAccount>>,
    pub redemption_token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

pub fn supply(ctx: Context<Supply>, amount: u64) -> Result<()> {
    require!(amount > 0, PSmError::ZeroAmount);

    let mut pool = ctx.accounts.pool.load_mut()?;
    let config = ctx.accounts.config.load()?;

    require!(!config.is_paused(), PSmError::ProtocolPaused);
    pool.can_supply()?;
    pool.record_supply(amount);

    transfer_checked(
        ctx.accounts.deposit_redemption_tokens(),
        amount,
        ctx.accounts.redemption_mint.decimals,
    )?;

    Ok(())
}

impl<'info> Supply<'info> {
    fn deposit_redemption_tokens(&self) -> CpiContext<'_, '_, '_, 'info, TransferChecked<'info>> {
        let cpi_accounts = TransferChecked {
            from: self.admin_redemption_token_account.to_account_info(),
            mint: self.redemption_mint.to_account_info(),
            to: self.redemption_token_account.to_account_info(),
            authority: self.admin.to_account_info(),
        };
        let cpi_program = self.redemption_token_program.to_account_info();
        CpiContext::new(cpi_program, cpi_accounts)
    }
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(
        mut,
        token::mint = settlement_mint,
        token::authority = admin,
    )]
    pub admin_settlement_token_account: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        constraint = config.load()?.is_admin(admin.key) @ PSmError::NotAuthorized,
    )]
    pub config: AccountLoader<'info, Config>,
    /// CHECK: checked with constraint
    pub authority: UncheckedAccount<'info>,
    #[account(mut)]
    pub settlement_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        mut,
        has_one = settlement_mint,
        has_one = settlement_token_account,
        has_one = settlement_token_program,
    )]
    pub pool: AccountLoader<'info, Pool>,
    #[account(mut)]
    pub settlement_token_account: Box<InterfaceAccount<'info, TokenAccount>>,
    pub settlement_token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
    require!(amount > 0, PSmError::ZeroAmount);
    require!(
        ctx.accounts.settlement_token_account.amount >= amount,
        PSmError::InsufficientPoolBalance
    );

    let mut pool = ctx.accounts.pool.load_mut()?;
    let config = ctx.accounts.config.load()?;

    require!(!config.is_paused(), PSmError::ProtocolPaused);
    pool.can_withdraw()?;
    pool.record_withdraw(amount);

    transfer_checked(
        ctx.accounts
            .withdraw_settlement_tokens()
            .with_signer(&[authority_seeds!(config.authority_bump)]),
        amount,
        ctx.accounts.settlement_mint.decimals,
    )?;

    Ok(())
}

impl<'info> Withdraw<'info> {
    fn withdraw_settlement_tokens(&self) -> CpiContext<'_, '_, '_, 'info, TransferChecked<'info>> {
        let cpi_accounts = TransferChecked {
            from: self.settlement_token_account.to_account_info(),
            mint: self.settlement_mint.to_account_info(),
            to: self.admin_settlement_token_account.to_account_info(),
            authority: self.authority.to_account_info(),
        };
        let cpi_program = self.settlement_token_program.to_account_info();
        CpiContext::new(cpi_program, cpi_accounts)
    }
}
