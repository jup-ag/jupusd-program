use anchor_lang::prelude::*;

use crate::{
    program::Psm,
    state::config::{Config, AUTHORITY_PREFIX, CONFIG_PREFIX},
};

#[derive(Accounts)]
pub struct Init<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    pub upgrade_authority: Signer<'info>,

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

    #[account(constraint = program_data.upgrade_authority_address == Some(upgrade_authority.key()))]
    pub program_data: Account<'info, ProgramData>,
    #[account(constraint = program.programdata_address()? == Some(program_data.key()))]
    pub program: Program<'info, Psm>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn init(ctx: Context<Init>) -> Result<()> {
    let mut config = ctx.accounts.config.load_init()?;

    config.add_admin(ctx.accounts.upgrade_authority.key)?;
    config.authority = ctx.accounts.authority.key();
    config.config_bump = ctx.bumps.config;
    config.authority_bump = ctx.bumps.authority;

    Ok(())
}
