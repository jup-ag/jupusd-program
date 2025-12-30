use anchor_lang::prelude::*;

use crate::{error::PSmError, state::config::Config};

#[derive(Accounts)]
pub struct ManageConfig<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(
        mut,
        constraint = config.load()?.is_admin(admin.key) @ PSmError::NotAuthorized,
    )]
    pub config: AccountLoader<'info, Config>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub enum ConfigManagementAction {
    AddAdmin { admin: Pubkey },
    RemoveAdmin { admin: Pubkey },
    UpdatePauseFlag { is_paused: bool },
}

pub fn manage_config(ctx: Context<ManageConfig>, action: ConfigManagementAction) -> Result<()> {
    let mut config = ctx.accounts.config.load_mut()?;

    match action {
        ConfigManagementAction::AddAdmin { admin } => {
            require!(admin != Pubkey::default(), PSmError::SomeError);
            require!(!config.is_admin(&admin), PSmError::DuplicateRessources);
            config.add_admin(&admin)?;
        },
        ConfigManagementAction::RemoveAdmin { admin } => {
            config.remove_admin(&admin)?;
            require!(config.num_admins() > 0, PSmError::NoAdminLeft);
        },
        ConfigManagementAction::UpdatePauseFlag { is_paused } => {
            config.update_pause_flag(is_paused)?;
        },
    }

    Ok(())
}
