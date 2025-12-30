use anchor_lang::{system_program, InstructionData, ToAccountMetas};
use solana_sdk::{instruction::Instruction, pubkey::Pubkey, sysvar};
use spl_associated_token_account::get_associated_token_address_with_program_id;

use crate::common::derivation::{
    find_authority, find_config, find_pool, find_pool_redemption_token_account,
    find_pool_settlement_token_account,
};

pub struct InitInstructionAccounts {
    pub payer: Pubkey,
    pub upgrade_authority: Pubkey,
    pub program_data: Pubkey,
}

pub fn create_init_instruction(accounts: InitInstructionAccounts) -> Instruction {
    let accounts = psm::accounts::Init {
        payer: accounts.payer,
        upgrade_authority: accounts.upgrade_authority,
        config: find_config(),
        authority: find_authority(),
        program_data: accounts.program_data,
        program: psm::id(),
        system_program: system_program::ID,
        rent: sysvar::rent::ID,
    }
    .to_account_metas(Some(true));

    Instruction {
        program_id: psm::id(),
        accounts,
        data: psm::instruction::Init {}.data(),
    }
}

pub struct ManageConfigInstructionAccounts {
    pub admin: Pubkey,
}

pub fn create_manage_config_instruction(
    accounts: ManageConfigInstructionAccounts,
    action: psm::instructions::ConfigManagementAction,
) -> Instruction {
    let accounts = psm::accounts::ManageConfig {
        admin: accounts.admin,
        config: find_config(),
    }
    .to_account_metas(Some(true));

    Instruction {
        program_id: psm::id(),
        accounts,
        data: psm::instruction::ManageConfig { action }.data(),
    }
}

pub fn create_add_admin_instruction(admin: Pubkey, new_admin: Pubkey) -> Instruction {
    create_manage_config_instruction(
        ManageConfigInstructionAccounts { admin },
        psm::instructions::ConfigManagementAction::AddAdmin { admin: new_admin },
    )
}

pub fn create_remove_admin_instruction(admin: Pubkey, remove_admin: Pubkey) -> Instruction {
    create_manage_config_instruction(
        ManageConfigInstructionAccounts { admin },
        psm::instructions::ConfigManagementAction::RemoveAdmin {
            admin: remove_admin,
        },
    )
}

pub fn create_update_pause_flag_instruction(admin: Pubkey, is_paused: bool) -> Instruction {
    create_manage_config_instruction(
        ManageConfigInstructionAccounts { admin },
        psm::instructions::ConfigManagementAction::UpdatePauseFlag { is_paused },
    )
}

// CreatePool instruction
pub struct CreatePoolInstructionAccounts {
    pub admin: Pubkey,
    pub payer: Pubkey,
    pub redemption_mint: Pubkey,
    pub settlement_mint: Pubkey,
    pub redemption_token_program: Pubkey,
    pub settlement_token_program: Pubkey,
}

pub fn create_create_pool_instruction(accounts: CreatePoolInstructionAccounts) -> Instruction {
    let pool = find_pool(&accounts.redemption_mint, &accounts.settlement_mint);
    let accounts = psm::accounts::CreatePool {
        admin: accounts.admin,
        payer: accounts.payer,
        redemption_mint: accounts.redemption_mint,
        settlement_mint: accounts.settlement_mint,
        config: find_config(),
        authority: find_authority(),
        pool,
        redemption_token_account: find_pool_redemption_token_account(&pool),
        settlement_token_account: find_pool_settlement_token_account(&pool),
        redemption_token_program: accounts.redemption_token_program,
        settlement_token_program: accounts.settlement_token_program,
        system_program: system_program::ID,
    }
    .to_account_metas(Some(true));

    Instruction {
        program_id: psm::id(),
        accounts,
        data: psm::instruction::CreatePool {}.data(),
    }
}

// ManagePool instruction
pub struct ManagePoolInstructionAccounts {
    pub admin: Pubkey,
    pub redemption_mint: Pubkey,
    pub settlement_mint: Pubkey,
}

pub fn create_manage_pool_instruction(
    accounts: ManagePoolInstructionAccounts,
    action: psm::instructions::PoolManagementAction,
) -> Instruction {
    let accounts = psm::accounts::ManagePool {
        admin: accounts.admin,
        config: find_config(),
        pool: find_pool(&accounts.redemption_mint, &accounts.settlement_mint),
    }
    .to_account_metas(Some(true));

    Instruction {
        program_id: psm::id(),
        accounts,
        data: psm::instruction::ManagePool { action }.data(),
    }
}

pub fn create_set_pool_status_instruction(
    admin: Pubkey,
    redemption_mint: Pubkey,
    settlement_mint: Pubkey,
    status: psm::state::pool::PoolStatus,
) -> Instruction {
    create_manage_pool_instruction(
        ManagePoolInstructionAccounts {
            admin,
            redemption_mint,
            settlement_mint,
        },
        psm::instructions::PoolManagementAction::SetStatus { status },
    )
}

pub struct SupplyInstructionAccounts {
    pub admin: Pubkey,
    pub redemption_mint: Pubkey,
    pub settlement_mint: Pubkey,
    pub redemption_token_program: Pubkey,
}

pub fn create_supply_instruction(accounts: SupplyInstructionAccounts, amount: u64) -> Instruction {
    let pool = find_pool(&accounts.redemption_mint, &accounts.settlement_mint);
    let admin_redemption_token_account = get_associated_token_address_with_program_id(
        &accounts.admin,
        &accounts.redemption_mint,
        &accounts.redemption_token_program,
    );

    let accounts = psm::accounts::Supply {
        admin: accounts.admin,
        admin_redemption_token_account,
        config: find_config(),
        redemption_mint: accounts.redemption_mint,
        pool,
        redemption_token_account: find_pool_redemption_token_account(&pool),
        redemption_token_program: accounts.redemption_token_program,
        system_program: system_program::ID,
    }
    .to_account_metas(Some(false));

    Instruction {
        program_id: psm::id(),
        accounts,
        data: psm::instruction::Supply { amount }.data(),
    }
}

pub struct RedeemInstructionAccounts {
    pub user: Pubkey,
    pub redemption_mint: Pubkey,
    pub settlement_mint: Pubkey,
    pub redemption_token_program: Pubkey,
    pub settlement_token_program: Pubkey,
}

pub fn create_redeem_instruction(accounts: RedeemInstructionAccounts, amount: u64) -> Instruction {
    let pool = find_pool(&accounts.redemption_mint, &accounts.settlement_mint);
    let user_redemption_token_account = get_associated_token_address_with_program_id(
        &accounts.user,
        &accounts.redemption_mint,
        &accounts.redemption_token_program,
    );
    let user_settlement_token_account = get_associated_token_address_with_program_id(
        &accounts.user,
        &accounts.settlement_mint,
        &accounts.settlement_token_program,
    );

    let accounts = psm::accounts::Redeem {
        user: accounts.user,
        user_redemption_token_account,
        user_settlement_token_account,
        config: find_config(),
        authority: find_authority(),
        settlement_mint: accounts.settlement_mint,
        redemption_mint: accounts.redemption_mint,
        pool,
        redemption_token_account: find_pool_redemption_token_account(&pool),
        settlement_token_account: find_pool_settlement_token_account(&pool),
        redemption_token_program: accounts.redemption_token_program,
        settlement_token_program: accounts.settlement_token_program,
        system_program: system_program::ID,
    }
    .to_account_metas(Some(false));

    Instruction {
        program_id: psm::id(),
        accounts,
        data: psm::instruction::Redeem { amount }.data(),
    }
}

pub struct WithdrawInstructionAccounts {
    pub admin: Pubkey,
    pub redemption_mint: Pubkey,
    pub settlement_mint: Pubkey,
    pub settlement_token_program: Pubkey,
}

pub fn create_withdraw_instruction(
    accounts: WithdrawInstructionAccounts,
    amount: u64,
) -> Instruction {
    let pool = find_pool(&accounts.redemption_mint, &accounts.settlement_mint);
    let admin_settlement_token_account = get_associated_token_address_with_program_id(
        &accounts.admin,
        &accounts.settlement_mint,
        &accounts.settlement_token_program,
    );

    let accounts = psm::accounts::Withdraw {
        admin: accounts.admin,
        admin_settlement_token_account,
        config: find_config(),
        authority: find_authority(),
        settlement_mint: accounts.settlement_mint,
        pool,
        settlement_token_account: find_pool_settlement_token_account(&pool),
        settlement_token_program: accounts.settlement_token_program,
        system_program: system_program::ID,
    }
    .to_account_metas(Some(false));

    Instruction {
        program_id: psm::id(),
        accounts,
        data: psm::instruction::Withdraw { amount }.data(),
    }
}
