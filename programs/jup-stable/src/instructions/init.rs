use anchor_lang::prelude::*;
use anchor_spl::{
    metadata::{
        self,
        mpl_token_metadata::{accounts::Metadata, types::DataV2},
        CreateMetadataAccountsV3,
    },
    token_interface::{Mint, TokenInterface},
};

use crate::{
    authority_seeds,
    program::JupStable,
    state::{
        config::{Config, AUTHORITY_PREFIX, CONFIG_PREFIX},
        operator::{Operator, OperatorStatus, OPERATOR_PREFIX},
    },
};

#[derive(Accounts)]
#[instruction(decimals: u8)]
pub struct Init<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    pub upgrade_authority: Signer<'info>,
    #[account(
        init,
        payer = payer,
        space = 8 + Operator::MAX_SIZE,
        seeds = [OPERATOR_PREFIX, upgrade_authority.key().as_ref()],
        bump
    )]
    pub operator: AccountLoader<'info, Operator>,
    #[account(
        init,
        payer = payer,
        space = 8 + Config::MAX_SIZE,
        seeds = [CONFIG_PREFIX],
        bump
    )]
    pub config: AccountLoader<'info, Config>,
    #[account(
        mut,
        seeds = [AUTHORITY_PREFIX],
        bump
    )]
    /// CHECK: checked with seeds constraint
    pub authority: AccountInfo<'info>,
    #[account(
        init,
        payer = payer,
        mint::decimals = decimals,
        mint::authority = authority,
        mint::token_program = token_program,
        mint::freeze_authority = authority,
    )]
    pub mint: Box<InterfaceAccount<'info, Mint>>,
    #[account(
        mut,
        address = Metadata::find_pda(&mint.key()).0
    )]
    /// CHECK: checked with constraint
    pub metadata: UncheckedAccount<'info>,

    #[account(constraint = program_data.upgrade_authority_address == Some(upgrade_authority.key()))]
    pub program_data: Account<'info, ProgramData>,
    #[account(constraint = program.programdata_address()? == Some(program_data.key()))]
    pub program: Program<'info, JupStable>,
    pub metadata_program: Program<'info, anchor_spl::metadata::Metadata>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn init(
    ctx: Context<Init>,
    _decimals: u8,
    name: String,
    symbol: String,
    uri: String,
) -> Result<()> {
    let mut config = ctx.accounts.config.load_init()?;
    *config = Config {
        mint: ctx.accounts.mint.key(),
        authority: ctx.accounts.authority.key(),
        config_bump: ctx.bumps.config,
        authority_bump: ctx.bumps.authority,
        token_program: ctx.accounts.token_program.key(),
        decimals: ctx.accounts.mint.decimals,
        ..Default::default()
    };

    let mut operator = ctx.accounts.operator.load_init()?;
    *operator = Operator {
        operator_authority: ctx.accounts.upgrade_authority.key(),
        role: u64::MAX,
        status: OperatorStatus::Enabled,
        ..Default::default()
    };

    // create metadata account
    metadata::create_metadata_accounts_v3(
        ctx.accounts
            .create_metadata()
            .with_signer(&[authority_seeds!(config.authority_bump)]),
        DataV2 {
            name: name.clone(),
            symbol: symbol.clone(),
            uri: uri.clone(),
            seller_fee_basis_points: 0,
            creators: None,
            collection: None,
            uses: None,
        },
        true,
        true,
        None,
    )?;

    Ok(())
}

impl<'info> Init<'info> {
    fn create_metadata(&self) -> CpiContext<'_, '_, '_, 'info, CreateMetadataAccountsV3<'info>> {
        let cpi_accounts = CreateMetadataAccountsV3 {
            metadata: self.metadata.to_account_info(),
            mint: self.mint.to_account_info(),
            mint_authority: self.authority.to_account_info(),
            payer: self.payer.to_account_info(),
            update_authority: self.authority.to_account_info(),
            system_program: self.system_program.to_account_info(),
            rent: self.rent.to_account_info(),
        };

        let cpi_program = self.metadata_program.to_account_info();
        CpiContext::new(cpi_program, cpi_accounts)
    }
}
