use anchor_lang::prelude::*;
use anchor_spl::token_interface::{
    burn, mint_to, transfer_checked, Burn, MintTo, TokenAccount, TokenInterface, TransferChecked,
};
use rust_decimal::{prelude::ToPrimitive, Decimal};

use crate::{
    authority_seeds,
    error::JupStableError,
    oracle::OraclePrice,
    state::{
        benefactor::Benefactor,
        config::{Config, AUTHORITY_PREFIX, PEG_PRICE_DECIMALS},
        vault::Vault,
    },
};

#[event_cpi]
#[derive(Accounts)]
pub struct Mint<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        mut,
        token::mint = vault_mint,
        token::authority = user,
    )]
    pub user_collateral_token_account: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(
        mut,
        token::mint = lp_mint,
        token::authority = user,
    )]
    pub user_lp_token_account: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        constraint = config.load()?.mint == lp_mint.key() @ JupStableError::InvalidLPMint,
        constraint = config.load()?.authority == authority.key() @ JupStableError::InvalidAuthority,
        constraint = config.load()?.token_program == lp_token_program.key() @ JupStableError::InvalidTokenProgram,
    )]
    pub config: AccountLoader<'info, Config>,
    /// CHECK: checked with constraint
    pub authority: UncheckedAccount<'info>,
    #[account(mut)]
    pub lp_mint: Box<InterfaceAccount<'info, anchor_spl::token_interface::Mint>>,

    #[account(
        mut,
        constraint = vault.load()?.custodian == custodian.key() @ JupStableError::InvalidCustodian,
        constraint = vault.load()?.mint == vault_mint.key() @ JupStableError::InvalidVaultMint,
        constraint = vault.load()?.token_program == vault_token_program.key() @ JupStableError::InvalidTokenProgram,
    )]
    pub vault: AccountLoader<'info, Vault>,
    pub vault_mint: Box<InterfaceAccount<'info, anchor_spl::token_interface::Mint>>,

    /// CHECK: checked with constraint on vault
    pub custodian: UncheckedAccount<'info>,
    #[account(
        mut,
        associated_token::authority = custodian,
        associated_token::mint = vault_mint,
        associated_token::token_program = vault_token_program,
    )]
    pub custodian_token_account: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        constraint = benefactor.load()?.authority == user.key() @ JupStableError::InvalidBenefactor,
    )]
    pub benefactor: AccountLoader<'info, Benefactor>,

    pub lp_token_program: Interface<'info, TokenInterface>,
    pub vault_token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

pub fn mint(ctx: Context<Mint>, amount: u64, min_amount_out: u64) -> Result<()> {
    require!(amount > 0, JupStableError::ZeroAmount);

    let mut vault = ctx.accounts.vault.load_mut()?;
    let mut benefactor = ctx.accounts.benefactor.load_mut()?;
    let mut config = ctx.accounts.config.load_mut()?;

    let clock = Clock::get()?;
    let current_time = clock.unix_timestamp;

    // Oracle accounts are passed as remaining_accounts
    let oracle_accounts = &ctx.remaining_accounts;
    let oracle_price = OraclePrice::parse_oracles(
        &vault.oracles,
        oracle_accounts,
        &clock,
        vault.stalesness_threshold,
    )?;

    vault.validate_oracle_price(&oracle_price, true)?;

    let peg_price = Decimal::new(config.peg_price_usd.try_into()?, PEG_PRICE_DECIMALS);
    let net_amount = amount - benefactor.calculate_mint_fee(amount);

    let (mint_amount, one_to_one_amount, oracle_amount) = compute_mint_amount(
        amount,
        net_amount,
        &oracle_price,
        peg_price,
        ctx.accounts.vault_mint.decimals,
        ctx.accounts.lp_mint.decimals,
    )?;

    emit_cpi!(MintV0Event {
        amount,
        net_amount,
        oracle_price: decimal_to_u64(oracle_price.0 * Decimal::from(10_i64.pow(6)))?,
        one_to_one_amount,
        oracle_amount,
        mint_amount,
    });

    config.can_mint(mint_amount, current_time)?;
    benefactor.can_mint(mint_amount, current_time)?;
    vault.can_mint(mint_amount, current_time)?;

    require!(mint_amount > 0, JupStableError::ZeroAmount);
    require!(
        mint_amount >= min_amount_out,
        JupStableError::SlippageToleranceExceeded
    );

    config.record_mint(mint_amount);
    benefactor.record_mint(mint_amount);
    vault.record_mint(mint_amount);

    let amount_before = ctx.accounts.custodian_token_account.amount;
    transfer_checked(
        ctx.accounts.deposit_collateral(),
        amount,
        ctx.accounts.vault_mint.decimals,
    )?;
    ctx.accounts.custodian_token_account.reload()?;
    let amount_after = ctx.accounts.custodian_token_account.amount;
    require!(
        amount_after == amount_before + amount,
        JupStableError::InsufficientAmount
    );

    mint_to(
        ctx.accounts
            .mint_lp_tokens()
            .with_signer(&[authority_seeds!(config.authority_bump)]),
        mint_amount,
    )?;

    Ok(())
}

impl<'info> Mint<'info> {
    fn deposit_collateral(&self) -> CpiContext<'_, '_, '_, 'info, TransferChecked<'info>> {
        let cpi_accounts = TransferChecked {
            from: self.user_collateral_token_account.to_account_info(),
            mint: self.vault_mint.to_account_info(),
            to: self.custodian_token_account.to_account_info(),
            authority: self.user.to_account_info(),
        };
        let cpi_program = self.vault_token_program.to_account_info();
        CpiContext::new(cpi_program, cpi_accounts)
    }

    fn mint_lp_tokens(&self) -> CpiContext<'_, '_, '_, 'info, MintTo<'info>> {
        let cpi_accounts = MintTo {
            mint: self.lp_mint.to_account_info(),
            to: self.user_lp_token_account.to_account_info(),
            authority: self.authority.to_account_info(),
        };
        let cpi_program = self.lp_token_program.to_account_info();
        CpiContext::new(cpi_program, cpi_accounts)
    }
}

#[event_cpi]
#[derive(Accounts)]
pub struct Redeem<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        mut,
        token::mint = lp_mint,
        token::authority = user,
    )]
    pub user_lp_token_account: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(
        mut,
        token::mint = vault_mint,
        token::authority = user,
    )]
    pub user_collateral_token_account: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        constraint = config.load()?.mint == lp_mint.key() @ JupStableError::InvalidLPMint,
        constraint = config.load()?.authority == authority.key() @ JupStableError::InvalidAuthority,
        constraint = config.load()?.token_program == lp_token_program.key() @ JupStableError::InvalidTokenProgram,
    )]
    pub config: AccountLoader<'info, Config>,
    /// CHECK: checked with constraint
    pub authority: UncheckedAccount<'info>,
    #[account(mut)]
    pub lp_mint: Box<InterfaceAccount<'info, anchor_spl::token_interface::Mint>>,

    #[account(
        mut,
        constraint = vault.load()?.mint == vault_mint.key() @ JupStableError::InvalidVaultMint,
        constraint = vault.load()?.token_account == vault_token_account.key() @ JupStableError::InvalidVaultTokenAccount,
        constraint = vault.load()?.token_program == vault_token_program.key() @ JupStableError::InvalidTokenProgram,
    )]
    pub vault: AccountLoader<'info, Vault>,
    #[account(mut)]
    pub vault_token_account: Box<InterfaceAccount<'info, TokenAccount>>,

    pub vault_mint: Box<InterfaceAccount<'info, anchor_spl::token_interface::Mint>>,

    #[account(
        mut,
        constraint = benefactor.load()?.authority == user.key() @ JupStableError::InvalidBenefactor,
    )]
    pub benefactor: AccountLoader<'info, Benefactor>,

    pub lp_token_program: Interface<'info, TokenInterface>,
    pub vault_token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

pub fn redeem(ctx: Context<Redeem>, amount: u64, min_amount_out: u64) -> Result<()> {
    require!(amount > 0, JupStableError::ZeroAmount);

    let mut vault = ctx.accounts.vault.load_mut()?;
    let mut benefactor = ctx.accounts.benefactor.load_mut()?;
    let mut config = ctx.accounts.config.load_mut()?;

    let clock = Clock::get()?;
    let current_time = clock.unix_timestamp;

    let oracle_accounts = &ctx.remaining_accounts;
    let oracle_price = OraclePrice::parse_oracles(
        &vault.oracles,
        oracle_accounts,
        &clock,
        vault.stalesness_threshold,
    )?;

    vault.validate_oracle_price(&oracle_price, false)?;

    let peg_price = Decimal::new(config.peg_price_usd.try_into()?, PEG_PRICE_DECIMALS);
    let net_amount = amount - benefactor.calculate_redeem_fee(amount);

    let (redeem_amount, one_to_one_amount, oracle_amount) = compute_redeem_amount(
        amount,
        net_amount,
        &oracle_price,
        peg_price,
        ctx.accounts.lp_mint.decimals,
        ctx.accounts.vault_mint.decimals,
    )?;

    emit_cpi!(RedeemV0Event {
        amount,
        net_amount,
        oracle_price: decimal_to_u64(oracle_price.0 * Decimal::from(10_i64.pow(6)))?,
        one_to_one_amount,
        oracle_amount,
        redeem_amount,
    });

    config.can_redeem(net_amount, current_time)?;
    vault.can_redeem(net_amount, current_time)?;
    benefactor.can_redeem(net_amount, current_time)?;

    require!(redeem_amount > 0, JupStableError::ZeroAmount);
    require!(
        redeem_amount >= min_amount_out,
        JupStableError::SlippageToleranceExceeded
    );
    require!(
        ctx.accounts.vault_token_account.amount >= redeem_amount,
        JupStableError::VaultIsDry
    );

    config.record_redeem(net_amount);
    benefactor.record_redeem(net_amount);
    vault.record_redeem(net_amount);

    burn(ctx.accounts.burn_lp_tokens(), amount)?;

    let amount_before = ctx.accounts.vault_token_account.amount;
    transfer_checked(
        ctx.accounts
            .withdraw_collateral()
            .with_signer(&[authority_seeds!(config.authority_bump)]),
        redeem_amount,
        ctx.accounts.vault_mint.decimals,
    )?;
    ctx.accounts.vault_token_account.reload()?;
    let amount_after = ctx.accounts.vault_token_account.amount;
    require!(
        amount_after == amount_before - redeem_amount,
        JupStableError::InsufficientAmount
    );

    Ok(())
}

impl<'info> Redeem<'info> {
    fn burn_lp_tokens(&self) -> CpiContext<'_, '_, '_, 'info, Burn<'info>> {
        let cpi_accounts = Burn {
            mint: self.lp_mint.to_account_info(),
            from: self.user_lp_token_account.to_account_info(),
            authority: self.user.to_account_info(),
        };
        let cpi_program = self.lp_token_program.to_account_info();
        CpiContext::new(cpi_program, cpi_accounts)
    }

    fn withdraw_collateral(&self) -> CpiContext<'_, '_, '_, 'info, TransferChecked<'info>> {
        let cpi_accounts = TransferChecked {
            from: self.vault_token_account.to_account_info(),
            mint: self.vault_mint.to_account_info(),
            to: self.user_collateral_token_account.to_account_info(),
            authority: self.authority.to_account_info(),
        };
        let cpi_program = self.vault_token_program.to_account_info();
        CpiContext::new(cpi_program, cpi_accounts)
    }
}

pub fn calculate_mint_amount(
    price: &OraclePrice,
    amount: Decimal,
    peg_price: Decimal,
    expected_decimals: u32,
) -> Result<Decimal> {
    Ok((amount * price.0 / peg_price) * Decimal::from(10_i64.pow(expected_decimals)))
}

pub fn calculate_redeem_amount(
    price: &OraclePrice,
    lp_amount: Decimal,
    peg_price: Decimal,
    expected_decimals: u32,
) -> Result<Decimal> {
    Ok((lp_amount * peg_price / price.0) * Decimal::from(10_i64.pow(expected_decimals)))
}

fn compute_mint_amount(
    amount: u64,
    net_amount: u64,
    oracle_price: &OraclePrice,
    peg_price: Decimal,
    vault_mint_decimals: u8,
    lp_mint_decimals: u8,
) -> Result<(u64, u64, u64)> {
    let vault_decimals = vault_mint_decimals as u32;
    let lp_decimals = lp_mint_decimals as u32;

    // Calculate 1:1 exchange rate amount (net amount after fees)
    let one_to_one_amount = Decimal::new(net_amount.try_into()?, vault_decimals) / peg_price
        * Decimal::from(10_i64.pow(lp_decimals));

    // Calculate oracle-based amount
    let oracle_amount = calculate_mint_amount(
        oracle_price,
        Decimal::new(amount.try_into()?, vault_decimals),
        peg_price,
        lp_decimals,
    )?;

    // Take the minimum and convert back to u64
    let mint_amount_decimal = oracle_amount.min(one_to_one_amount);
    let mint_amount = decimal_to_u64(mint_amount_decimal)?;

    Ok((
        mint_amount,
        decimal_to_u64(one_to_one_amount)?,
        decimal_to_u64(oracle_amount)?,
    ))
}

fn compute_redeem_amount(
    amount: u64,
    net_amount: u64,
    oracle_price: &OraclePrice,
    peg_price: Decimal,
    lp_mint_decimals: u8,
    vault_mint_decimals: u8,
) -> Result<(u64, u64, u64)> {
    let lp_decimals = lp_mint_decimals as u32;
    let vault_decimals = vault_mint_decimals as u32;

    // Calculate 1:1 exchange rate amount (net amount after fees)
    let one_to_one_amount = Decimal::new(net_amount.try_into()?, lp_decimals)
        * peg_price
        * Decimal::from(10_i64.pow(vault_decimals));

    // Calculate oracle-based amount
    let oracle_amount = calculate_redeem_amount(
        oracle_price,
        Decimal::new(amount.try_into()?, lp_decimals),
        peg_price,
        vault_decimals,
    )?;

    // Take the minimum and convert to u64
    let redeem_amount_decimal = oracle_amount.min(one_to_one_amount);
    let redeem_amount = decimal_to_u64(redeem_amount_decimal)?;

    Ok((
        redeem_amount,
        decimal_to_u64(one_to_one_amount)?,
        decimal_to_u64(oracle_amount)?,
    ))
}

fn decimal_to_u64(value: Decimal) -> Result<u64> {
    value.to_u64().ok_or(error!(JupStableError::MathOverflow))
}

#[event]
pub struct MintV0Event {
    pub amount: u64,
    pub net_amount: u64,
    pub oracle_price: u64,
    pub one_to_one_amount: u64,
    pub oracle_amount: u64,
    pub mint_amount: u64,
}

#[event]
pub struct RedeemV0Event {
    pub amount: u64,
    pub net_amount: u64,
    pub oracle_price: u64,
    pub one_to_one_amount: u64,
    pub oracle_amount: u64,
    pub redeem_amount: u64,
}
