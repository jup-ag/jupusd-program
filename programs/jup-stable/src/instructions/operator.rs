use anchor_lang::prelude::*;

use crate::{
    error::JupStableError,
    state::operator::{Operator, OperatorRole, OperatorStatus, OPERATOR_PREFIX},
};

#[derive(Accounts)]
pub struct CreateOperator<'info> {
    pub operator_authority: Signer<'info>,
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        has_one = operator_authority @ JupStableError::NotAuthorized,
    )]
    pub operator: AccountLoader<'info, Operator>,

    /// CHECK:
    pub new_operator_authority: UncheckedAccount<'info>,
    #[account(
        init,
        payer = payer,
        space = 8 + Operator::MAX_SIZE,
        seeds = [OPERATOR_PREFIX, new_operator_authority.key().as_ref()],
        bump
    )]
    pub new_operator: AccountLoader<'info, Operator>,
    pub system_program: Program<'info, System>,
}

pub fn create_operator(ctx: Context<CreateOperator>, role: OperatorRole) -> Result<()> {
    let operator = ctx.accounts.operator.load()?;
    operator.is(OperatorRole::Admin)?;

    let mut new_operator = ctx.accounts.new_operator.load_init()?;
    *new_operator = Operator {
        operator_authority: ctx.accounts.new_operator_authority.key(),
        status: OperatorStatus::Enabled,
        ..Default::default()
    };
    new_operator.set_role(role);

    Ok(())
}

#[derive(Accounts)]
pub struct DeleteOperator<'info> {
    pub operator_authority: Signer<'info>,
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        has_one = operator_authority @ JupStableError::NotAuthorized,
    )]
    pub operator: AccountLoader<'info, Operator>,

    #[account(
        mut,
        close = payer
    )]
    pub deleted_operator: AccountLoader<'info, Operator>,
}

pub fn delete_operator(ctx: Context<DeleteOperator>) -> Result<()> {
    require!(
        ctx.accounts.deleted_operator.key() != ctx.accounts.operator.key(),
        JupStableError::OperatorCannotDeleteItself
    );

    let operator = ctx.accounts.operator.load()?;
    operator.is(OperatorRole::Admin)?;

    Ok(())
}

#[derive(Accounts)]
pub struct ManageOperator<'info> {
    pub operator_authority: Signer<'info>,
    #[account(
        has_one = operator_authority @ JupStableError::NotAuthorized,
    )]
    pub operator: AccountLoader<'info, Operator>,

    #[account(mut)]
    pub managed_operator: AccountLoader<'info, Operator>,
    pub system_program: Program<'info, System>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub enum OperatorManagementAction {
    SetStatus { status: OperatorStatus },
    SetRole { role: OperatorRole },
    ClearRole { role: OperatorRole },
}

pub fn manage_operator(
    ctx: Context<ManageOperator>,
    action: OperatorManagementAction,
) -> Result<()> {
    let operator = ctx.accounts.operator.load()?;
    operator.is(OperatorRole::Admin)?;
    drop(operator);

    let mut managed_operator = ctx.accounts.managed_operator.load_mut()?;
    match action {
        OperatorManagementAction::SetStatus { status } => {
            managed_operator.status = status;
        },
        OperatorManagementAction::SetRole { role } => {
            managed_operator.set_role(role);
        },
        OperatorManagementAction::ClearRole { role } => {
            managed_operator.clear_role(role);
        },
    }

    Ok(())
}
