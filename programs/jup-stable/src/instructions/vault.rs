use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_2022::{transfer_checked, TransferChecked},
    token_interface::{Mint, TokenAccount, TokenInterface},
};

use crate::{
    authority_seeds,
    error::JupStableError,
    state::{
        config::{Config, AUTHORITY_PREFIX},
        operator::{Operator, OperatorRole},
        vault::{
            DovesOracle, EmptyOracle, OracleType, PythV2Oracle, SwitchboardOnDemandOracle, Vault,
            VaultStatus, VAULT_PREFIX,
        },
    },
};

#[derive(AnchorSerialize, AnchorDeserialize)]
pub enum OracleConfig {
    None,
    Pyth([u8; 32], Pubkey),
    SwitchboardOnDemand(Pubkey),
    Doves(Pubkey),
}

impl From<OracleConfig> for OracleType {
    fn from(c: OracleConfig) -> Self {
        match c {
            OracleConfig::None => OracleType::Empty(EmptyOracle::default()),
            OracleConfig::Pyth(feed_id, account) => OracleType::Pyth(PythV2Oracle {
                feed_id,
                account,
                ..Default::default()
            }),
            OracleConfig::SwitchboardOnDemand(account) => {
                OracleType::SwitchboardOnDemand(SwitchboardOnDemandOracle {
                    account,
                    ..Default::default()
                })
            },
            OracleConfig::Doves(account) => OracleType::Doves(DovesOracle {
                account,
                ..Default::default()
            }),
        }
    }
}

#[derive(Accounts)]
pub struct CreateVault<'info> {
    pub operator_authority: Signer<'info>,
    #[account(
        has_one = operator_authority @ JupStableError::NotAuthorized,
    )]
    pub operator: AccountLoader<'info, Operator>,

    #[account(mut)]
    pub payer: Signer<'info>,
    pub mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        has_one = authority @ JupStableError::InvalidAuthority,
    )]
    pub config: AccountLoader<'info, Config>,
    /// CHECK: checked with constraint
    pub authority: UncheckedAccount<'info>,

    #[account(
        init,
        payer = payer,
        space = 8 + Vault::MAX_SIZE,
        seeds = [VAULT_PREFIX, mint.key().as_ref()],
        bump
    )]
    pub vault: AccountLoader<'info, Vault>,

    #[account(
        init_if_needed,
        payer = payer,
        associated_token::authority = authority,
        associated_token::mint = mint,
        associated_token::token_program = token_program,
    )]
    pub token_account: Box<InterfaceAccount<'info, TokenAccount>>,

    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

pub fn create_vault(ctx: Context<CreateVault>) -> Result<()> {
    let config = ctx.accounts.config.load()?;
    let operator = ctx.accounts.operator.load()?;
    operator.is(OperatorRole::VaultManager)?;

    let mint = ctx.accounts.mint.key();
    require!(mint != config.mint, JupStableError::InvalidVaultMint);

    let mut vault = ctx.accounts.vault.load_init()?;
    *vault = Vault {
        mint,
        decimals: ctx.accounts.mint.decimals,
        token_account: ctx.accounts.token_account.key(),
        token_program: ctx.accounts.token_program.key(),
        status: VaultStatus::Disabled,
        bump: ctx.bumps.vault,
        ..Default::default()
    };

    Ok(())
}

#[derive(Accounts)]
pub struct ManageVault<'info> {
    pub operator_authority: Signer<'info>,
    #[account(
        has_one = operator_authority @ JupStableError::NotAuthorized,
    )]
    pub operator: AccountLoader<'info, Operator>,

    #[account(mut)]
    pub vault: AccountLoader<'info, Vault>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub enum VaultManagementAction {
    Disable,
    SetStatus {
        status: VaultStatus,
    },
    UpdateOracle {
        index: u8,
        oracle: OracleConfig,
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
    SetCustodian {
        new_custodian: Pubkey,
    },
    SetStalesnessThreshold {
        stalesness_threshold: u64,
    },
    SetMinOraclePrice {
        min_oracle_price_usd: u64,
    },
    SetMaxOraclePrice {
        max_oracle_price_usd: u64,
    },
}

pub fn manage_vault(ctx: Context<ManageVault>, action: VaultManagementAction) -> Result<()> {
    let mut vault = ctx.accounts.vault.load_mut()?;
    let operator = ctx.accounts.operator.load()?;

    match action {
        VaultManagementAction::Disable => {
            operator.is(OperatorRole::VaultDisabler)?;

            vault.is_enabled()?;
            vault.status = VaultStatus::Disabled;
        },
        VaultManagementAction::SetStatus { status } => {
            operator.is(OperatorRole::VaultManager)?;

            if status == VaultStatus::Enabled {
                require!(
                    vault.custodian != Pubkey::default(),
                    JupStableError::InvalidCustodian
                );

                let valid_oracles = vault
                    .oracles
                    .iter()
                    .any(|oracle| !matches!(oracle, OracleType::Empty(_)));
                require!(valid_oracles, JupStableError::NoValidOracle);
            }

            vault.set_status(status);
        },
        VaultManagementAction::UpdateOracle { index, oracle } => {
            operator.is(OperatorRole::VaultManager)?;

            vault.update_oracle(index.into(), &oracle.into())?;

            let valid_oracles = vault
                .oracles
                .iter()
                .any(|oracle| !matches!(oracle, OracleType::Empty(_)));

            if !valid_oracles {
                vault.set_status(VaultStatus::Disabled);
            }
        },
        VaultManagementAction::UpdatePeriodLimit {
            index,
            duration_seconds,
            max_mint_amount,
            max_redeem_amount,
        } => {
            operator.is(OperatorRole::PeriodManager)?;

            let current_time = Clock::get()?.unix_timestamp;
            vault.update_period_limit(
                index.into(),
                duration_seconds,
                max_mint_amount,
                max_redeem_amount,
                current_time,
            )?;
        },
        VaultManagementAction::ResetPeriodLimit { index } => {
            operator.is(OperatorRole::PeriodManager)?;

            vault.reset_period_limit(index.into())?;
        },
        VaultManagementAction::SetCustodian { new_custodian } => {
            operator.is(OperatorRole::VaultManager)?;

            require!(
                new_custodian != Pubkey::default(),
                JupStableError::InvalidCustodian
            );

            vault.custodian = new_custodian;
        },
        VaultManagementAction::SetStalesnessThreshold {
            stalesness_threshold,
        } => {
            operator.is(OperatorRole::VaultManager)?;

            vault.set_stalesness_threshold(stalesness_threshold);
        },
        VaultManagementAction::SetMinOraclePrice {
            min_oracle_price_usd,
        } => {
            operator.is(OperatorRole::VaultManager)?;

            require!(min_oracle_price_usd > 0, JupStableError::BadInput);
            require!(
                min_oracle_price_usd < vault.max_oracle_price_usd,
                JupStableError::BadInput
            );

            vault.set_min_oracle_price_usd(min_oracle_price_usd);
        },
        VaultManagementAction::SetMaxOraclePrice {
            max_oracle_price_usd,
        } => {
            operator.is(OperatorRole::VaultManager)?;

            require!(max_oracle_price_usd > 0, JupStableError::BadInput);
            require!(
                max_oracle_price_usd > vault.min_oracle_price_usd,
                JupStableError::BadInput
            );

            vault.set_max_oracle_price_usd(max_oracle_price_usd);
        },
    }

    Ok(())
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    pub operator_authority: Signer<'info>,
    #[account(
        has_one = operator_authority @ JupStableError::NotAuthorized,
    )]
    pub operator: AccountLoader<'info, Operator>,

    /// CHECK: checked with constraint on vault
    pub custodian: UncheckedAccount<'info>,

    #[account(
        mut,
        associated_token::mint = vault_mint,
        associated_token::authority = custodian,
        associated_token::token_program = token_program,
    )]
    pub custodian_token_account: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        constraint = config.load()?.authority == authority.key() @ JupStableError::InvalidAuthority,
    )]
    pub config: AccountLoader<'info, Config>,
    /// CHECK: checked with constraint
    pub authority: UncheckedAccount<'info>,

    #[account(
        mut,
        constraint = vault.load()?.custodian == custodian.key() @ JupStableError::InvalidCustodian,
        constraint = vault.load()?.mint == vault_mint.key() @ JupStableError::InvalidVaultMint,
        constraint = vault.load()?.token_account == vault_token_account.key() @ JupStableError::InvalidVaultTokenAccount,
        constraint = vault.load()?.token_program == token_program.key() @ JupStableError::InvalidTokenProgram,
    )]
    pub vault: AccountLoader<'info, Vault>,

    #[account(mut)]
    pub vault_token_account: Box<InterfaceAccount<'info, TokenAccount>>,

    pub vault_mint: Box<InterfaceAccount<'info, Mint>>,

    pub token_program: Interface<'info, TokenInterface>,
}

pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
    require!(amount > 0, JupStableError::ZeroAmount);

    let operator = ctx.accounts.operator.load()?;
    operator.is(OperatorRole::CollateralManager)?;

    let vault = ctx.accounts.vault.load()?;
    let config = ctx.accounts.config.load()?;

    vault.is_enabled()?;

    require!(
        ctx.accounts.vault_token_account.amount >= amount,
        JupStableError::InsufficientAmount
    );

    transfer_checked(
        ctx.accounts
            .withdraw_from_vault()
            .with_signer(&[authority_seeds!(config.authority_bump)]),
        amount,
        ctx.accounts.vault_mint.decimals,
    )?;

    Ok(())
}

impl<'info> Withdraw<'info> {
    fn withdraw_from_vault(&self) -> CpiContext<'_, '_, '_, 'info, TransferChecked<'info>> {
        let cpi_accounts = TransferChecked {
            from: self.vault_token_account.to_account_info(),
            mint: self.vault_mint.to_account_info(),
            to: self.custodian_token_account.to_account_info(),
            authority: self.authority.to_account_info(),
        };
        let cpi_program = self.token_program.to_account_info();
        CpiContext::new(cpi_program, cpi_accounts)
    }
}
