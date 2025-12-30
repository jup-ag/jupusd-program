use anchor_lang::{system_program, Id, InstructionData, ToAccountMetas};
use anchor_spl::{associated_token::AssociatedToken, metadata};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    sysvar,
};
use spl_associated_token_account::get_associated_token_address_with_program_id;

use crate::common::derivation::{
    find_authority, find_benefactor, find_config, find_event_authority, find_metadata,
    find_operator, find_vault, find_vault_token_account,
};

#[derive(Debug)]
pub struct InitInstructionAccounts {
    pub payer: Pubkey,
    pub upgrade_authority: Pubkey,
    pub program_data: Pubkey,
    pub mint: Pubkey,
    pub token_program: Pubkey,
}

pub struct InitInstructionArgs {
    pub decimals: u8,
    pub name: String,
    pub symbol: String,
    pub uri: String,
}

pub fn create_init_instruction(
    accounts: InitInstructionAccounts,
    args: InitInstructionArgs,
) -> Instruction {
    let accounts = jup_stable::accounts::Init {
        payer: accounts.payer,
        upgrade_authority: accounts.upgrade_authority,
        operator: find_operator(&accounts.upgrade_authority),
        config: find_config(),
        authority: find_authority(),
        mint: accounts.mint,
        metadata: find_metadata(&accounts.mint),
        program_data: accounts.program_data,
        program: jup_stable::id(),
        metadata_program: metadata::ID,
        token_program: accounts.token_program,
        system_program: system_program::ID,
        rent: sysvar::rent::ID,
    }
    .to_account_metas(Some(true));

    Instruction {
        program_id: jup_stable::id(),
        accounts,
        data: jup_stable::instruction::Init {
            decimals: args.decimals,
            name: args.name,
            symbol: args.symbol,
            uri: args.uri,
        }
        .data(),
    }
}

#[derive(Debug)]
pub struct CreateVaultInstructionAccounts {
    pub authority: Pubkey,
    pub payer: Pubkey,
    pub mint: Pubkey,
    pub token_program: Pubkey,
}

pub fn create_create_vault_instruction(accounts: CreateVaultInstructionAccounts) -> Instruction {
    Instruction {
        program_id: jup_stable::id(),
        accounts: jup_stable::accounts::CreateVault {
            operator_authority: accounts.authority,
            operator: find_operator(&accounts.authority),
            payer: accounts.payer,
            mint: accounts.mint,
            config: find_config(),
            authority: find_authority(),
            vault: find_vault(&accounts.mint),
            token_account: find_vault_token_account(&accounts.mint),
            token_program: accounts.token_program,
            system_program: system_program::ID,
            associated_token_program: AssociatedToken::id(),
        }
        .to_account_metas(Some(true)),
        data: jup_stable::instruction::CreateVault {}.data(),
    }
}

pub struct CreateBenefactorInstructionAccounts {
    pub authority: Pubkey,
    pub payer: Pubkey,
    pub benefactor_authority: Pubkey,
}

pub struct CreateBenefactorInstructionArgs {
    pub mint_fee_rate: u16,
    pub redeem_fee_rate: u16,
}

pub fn create_create_benefactor_instruction(
    accounts: CreateBenefactorInstructionAccounts,
    args: CreateBenefactorInstructionArgs,
) -> Instruction {
    let accounts = jup_stable::accounts::CreateBenefactor {
        operator_authority: accounts.authority,
        operator: find_operator(&accounts.authority),
        payer: accounts.payer,
        benefactor_authority: accounts.benefactor_authority,
        benefactor: find_benefactor(&accounts.benefactor_authority),
        system_program: system_program::ID,
    }
    .to_account_metas(Some(true));

    Instruction {
        program_id: jup_stable::id(),
        accounts,
        data: jup_stable::instruction::CreateBenefactor {
            mint_fee_rate: args.mint_fee_rate,
            redeem_fee_rate: args.redeem_fee_rate,
        }
        .data(),
    }
}

pub struct MintInstructionAccounts {
    pub user: Pubkey,
    pub benefactor: Pubkey,
    pub custodian: Pubkey,
    pub vault_mint: Pubkey,
    pub lp_mint: Pubkey,
    pub vault_token_program: Pubkey,
    pub lp_token_program: Pubkey,
    pub remaining_accounts: Vec<Pubkey>,
}

pub fn create_mint_instruction(
    amount: u64,
    min_amount_out: u64,
    accounts: MintInstructionAccounts,
) -> Instruction {
    let user_collateral_ata = get_associated_token_address_with_program_id(
        &accounts.user,
        &accounts.vault_mint,
        &accounts.vault_token_program,
    );
    let user_lp_ata = get_associated_token_address_with_program_id(
        &accounts.user,
        &accounts.lp_mint,
        &accounts.lp_token_program,
    );
    let custodian_ata = get_associated_token_address_with_program_id(
        &accounts.custodian,
        &accounts.vault_mint,
        &accounts.vault_token_program,
    );

    let mut acc = jup_stable::accounts::Mint {
        user: accounts.user,
        user_collateral_token_account: user_collateral_ata,
        user_lp_token_account: user_lp_ata,
        config: find_config(),
        authority: find_authority(),
        lp_mint: accounts.lp_mint,
        vault: find_vault(&accounts.vault_mint),
        custodian: accounts.custodian,
        custodian_token_account: custodian_ata,
        vault_mint: accounts.vault_mint,
        benefactor: accounts.benefactor,
        lp_token_program: accounts.lp_token_program,
        vault_token_program: accounts.vault_token_program,
        system_program: system_program::ID,
        event_authority: find_event_authority(),
        program: jup_stable::id(),
    }
    .to_account_metas(Some(false));

    acc.extend(
        accounts
            .remaining_accounts
            .iter()
            .map(|account| AccountMeta::new_readonly(*account, false)),
    );

    Instruction {
        program_id: jup_stable::id(),
        accounts: acc,
        data: jup_stable::instruction::Mint {
            amount,
            min_amount_out,
        }
        .data(),
    }
}

pub struct RedeemInstructionAccounts {
    pub user: Pubkey,
    pub benefactor: Pubkey,
    pub vault_mint: Pubkey,
    pub lp_mint: Pubkey,
    pub vault_token_program: Pubkey,
    pub lp_token_program: Pubkey,
    pub remaining_accounts: Vec<Pubkey>,
}

pub fn create_redeem_instruction(
    amount: u64,
    min_amount_out: u64,
    accounts: RedeemInstructionAccounts,
) -> Instruction {
    let user_collateral_ata = get_associated_token_address_with_program_id(
        &accounts.user,
        &accounts.vault_mint,
        &accounts.vault_token_program,
    );
    let user_lp_ata = get_associated_token_address_with_program_id(
        &accounts.user,
        &accounts.lp_mint,
        &accounts.lp_token_program,
    );

    let mut accs = jup_stable::accounts::Redeem {
        user: accounts.user,
        user_lp_token_account: user_lp_ata,
        user_collateral_token_account: user_collateral_ata,
        config: find_config(),
        authority: find_authority(),
        lp_mint: accounts.lp_mint,
        vault: find_vault(&accounts.vault_mint),
        vault_token_account: find_vault_token_account(&accounts.vault_mint),
        vault_mint: accounts.vault_mint,
        benefactor: accounts.benefactor,
        lp_token_program: accounts.lp_token_program,
        vault_token_program: accounts.vault_token_program,
        system_program: system_program::ID,
        event_authority: find_event_authority(),
        program: jup_stable::id(),
    }
    .to_account_metas(Some(false));
    accs.extend(
        accounts
            .remaining_accounts
            .iter()
            .map(|account| AccountMeta::new_readonly(*account, false)),
    );

    Instruction {
        program_id: jup_stable::id(),
        accounts: accs,
        data: jup_stable::instruction::Redeem {
            amount,
            min_amount_out,
        }
        .data(),
    }
}

pub struct WithdrawInstructionAccounts {
    pub operator_authority: Pubkey,
    pub custodian: Pubkey,
    pub vault_mint: Pubkey,
    pub vault_token_program: Pubkey,
}

pub fn create_withdraw_instruction(
    accounts: WithdrawInstructionAccounts,
    amount: u64,
) -> Instruction {
    let accounts = jup_stable::accounts::Withdraw {
        operator_authority: accounts.operator_authority,
        operator: find_operator(&accounts.operator_authority),
        custodian: accounts.custodian,
        custodian_token_account: get_associated_token_address_with_program_id(
            &accounts.custodian,
            &accounts.vault_mint,
            &accounts.vault_token_program,
        ),
        config: find_config(),
        authority: find_authority(),
        vault: find_vault(&accounts.vault_mint),
        vault_token_account: find_vault_token_account(&accounts.vault_mint),
        vault_mint: accounts.vault_mint,
        token_program: accounts.vault_token_program,
    }
    .to_account_metas(Some(false));

    Instruction {
        program_id: jup_stable::id(),
        accounts,
        data: jup_stable::instruction::Withdraw { amount }.data(),
    }
}

pub struct ManageConfigInstructionAccounts {
    pub authority: Pubkey,
}

pub fn create_manage_config_instruction(
    accounts: ManageConfigInstructionAccounts,
    action: jup_stable::instructions::ConfigManagementAction,
) -> Instruction {
    let accounts = jup_stable::accounts::ManageConfig {
        operator_authority: accounts.authority,
        operator: find_operator(&accounts.authority),
        config: find_config(),
    }
    .to_account_metas(Some(true));

    Instruction {
        program_id: jup_stable::id(),
        accounts,
        data: jup_stable::instruction::ManageConfig { action }.data(),
    }
}

pub fn create_update_pause_flag_instruction(
    authority: Pubkey,
    is_mint_redeem_enabled: bool,
) -> Instruction {
    create_manage_config_instruction(
        ManageConfigInstructionAccounts { authority },
        jup_stable::instructions::ConfigManagementAction::UpdatePauseFlag {
            is_mint_redeem_enabled,
        },
    )
}

pub fn create_update_config_period_limit_instruction(
    authority: Pubkey,
    index: u8,
    duration_seconds: u64,
    max_mint_amount: u64,
    max_redeem_amount: u64,
) -> Instruction {
    create_manage_config_instruction(
        ManageConfigInstructionAccounts { authority },
        jup_stable::instructions::ConfigManagementAction::UpdatePeriodLimit {
            index,
            duration_seconds,
            max_mint_amount,
            max_redeem_amount,
        },
    )
}

pub fn create_reset_config_period_limit_instruction(authority: Pubkey, index: u8) -> Instruction {
    create_manage_config_instruction(
        ManageConfigInstructionAccounts { authority },
        jup_stable::instructions::ConfigManagementAction::ResetPeriodLimit { index },
    )
}

pub struct ManageVaultInstructionAccounts {
    pub authority: Pubkey,
    pub vault_mint: Pubkey,
}

pub fn create_manage_vault_instruction(
    accounts: ManageVaultInstructionAccounts,
    action: jup_stable::instructions::VaultManagementAction,
) -> Instruction {
    let accounts = jup_stable::accounts::ManageVault {
        operator_authority: accounts.authority,
        operator: find_operator(&accounts.authority),
        vault: find_vault(&accounts.vault_mint),
    }
    .to_account_metas(Some(true));

    Instruction {
        program_id: jup_stable::id(),
        accounts,
        data: jup_stable::instruction::ManageVault { action }.data(),
    }
}

// Convenience functions for common vault management actions
pub fn create_set_vault_status_instruction(
    authority: Pubkey,
    vault_mint: Pubkey,
    status: jup_stable::state::vault::VaultStatus,
) -> Instruction {
    create_manage_vault_instruction(
        ManageVaultInstructionAccounts {
            authority,
            vault_mint,
        },
        jup_stable::instructions::VaultManagementAction::SetStatus { status },
    )
}

pub fn create_set_custodian_instruction(
    authority: Pubkey,
    vault_mint: Pubkey,
    new_custodian: Pubkey,
) -> Instruction {
    create_manage_vault_instruction(
        ManageVaultInstructionAccounts {
            authority,
            vault_mint,
        },
        jup_stable::instructions::VaultManagementAction::SetCustodian { new_custodian },
    )
}

pub fn create_update_vault_oracle_instruction(
    authority: Pubkey,
    vault_mint: Pubkey,
    index: u8,
    oracle: jup_stable::instructions::OracleConfig,
) -> Instruction {
    create_manage_vault_instruction(
        ManageVaultInstructionAccounts {
            authority,
            vault_mint,
        },
        jup_stable::instructions::VaultManagementAction::UpdateOracle { index, oracle },
    )
}

pub fn create_update_vault_period_limit_instruction(
    authority: Pubkey,
    vault_mint: Pubkey,
    index: u8,
    duration_seconds: u64,
    max_mint_amount: u64,
    max_redeem_amount: u64,
) -> Instruction {
    create_manage_vault_instruction(
        ManageVaultInstructionAccounts {
            authority,
            vault_mint,
        },
        jup_stable::instructions::VaultManagementAction::UpdatePeriodLimit {
            index,
            duration_seconds,
            max_mint_amount,
            max_redeem_amount,
        },
    )
}

pub fn create_reset_vault_period_limit_instruction(
    authority: Pubkey,
    vault_mint: Pubkey,
    index: u8,
) -> Instruction {
    create_manage_vault_instruction(
        ManageVaultInstructionAccounts {
            authority,
            vault_mint,
        },
        jup_stable::instructions::VaultManagementAction::ResetPeriodLimit { index },
    )
}

pub fn create_set_stalesness_threshold_instruction(
    authority: Pubkey,
    vault_mint: Pubkey,
    stalesness_threshold: u64,
) -> Instruction {
    create_manage_vault_instruction(
        ManageVaultInstructionAccounts {
            authority,
            vault_mint,
        },
        jup_stable::instructions::VaultManagementAction::SetStalesnessThreshold {
            stalesness_threshold,
        },
    )
}

pub fn create_set_min_oracle_price_instruction(
    authority: Pubkey,
    vault_mint: Pubkey,
    min_oracle_price_usd: u64,
) -> Instruction {
    create_manage_vault_instruction(
        ManageVaultInstructionAccounts {
            authority,
            vault_mint,
        },
        jup_stable::instructions::VaultManagementAction::SetMinOraclePrice {
            min_oracle_price_usd,
        },
    )
}

pub fn create_set_max_oracle_price_instruction(
    authority: Pubkey,
    vault_mint: Pubkey,
    max_oracle_price_usd: u64,
) -> Instruction {
    create_manage_vault_instruction(
        ManageVaultInstructionAccounts {
            authority,
            vault_mint,
        },
        jup_stable::instructions::VaultManagementAction::SetMaxOraclePrice {
            max_oracle_price_usd,
        },
    )
}

pub struct ManageBenefactorInstructionAccounts {
    pub authority: Pubkey,
    pub benefactor: Pubkey,
}

pub fn create_manage_benefactor_instruction(
    accounts: ManageBenefactorInstructionAccounts,
    action: jup_stable::instructions::BenefactorManagementAction,
) -> Instruction {
    let accounts = jup_stable::accounts::ManageBenefactor {
        operator_authority: accounts.authority,
        operator: find_operator(&accounts.authority),
        benefactor: accounts.benefactor,
    }
    .to_account_metas(Some(true));

    Instruction {
        program_id: jup_stable::id(),
        accounts,
        data: jup_stable::instruction::ManageBenefactor { action }.data(),
    }
}

// Convenience functions for common benefactor management actions
pub fn create_set_benefactor_status_instruction(
    authority: Pubkey,
    benefactor: Pubkey,
    status: jup_stable::state::benefactor::BenefactorStatus,
) -> Instruction {
    create_manage_benefactor_instruction(
        ManageBenefactorInstructionAccounts {
            authority,
            benefactor,
        },
        jup_stable::instructions::BenefactorManagementAction::SetStatus { status },
    )
}

pub fn create_update_fee_rates_instruction(
    authority: Pubkey,
    benefactor: Pubkey,
    mint_fee_rate: u16,
    redeem_fee_rate: u16,
) -> Instruction {
    create_manage_benefactor_instruction(
        ManageBenefactorInstructionAccounts {
            authority,
            benefactor,
        },
        jup_stable::instructions::BenefactorManagementAction::UpdateFeeRates {
            mint_fee_rate,
            redeem_fee_rate,
        },
    )
}

pub fn create_update_benefactor_period_limit_instruction(
    authority: Pubkey,
    benefactor: Pubkey,
    index: u8,
    duration_seconds: u64,
    max_mint_amount: u64,
    max_redeem_amount: u64,
) -> Instruction {
    create_manage_benefactor_instruction(
        ManageBenefactorInstructionAccounts {
            authority,
            benefactor,
        },
        jup_stable::instructions::BenefactorManagementAction::UpdatePeriodLimit {
            index,
            duration_seconds,
            max_mint_amount,
            max_redeem_amount,
        },
    )
}

#[allow(dead_code)]
pub fn create_reset_benefactor_period_limit_instruction(
    authority: Pubkey,
    benefactor: Pubkey,
    index: u8,
) -> Instruction {
    create_manage_benefactor_instruction(
        ManageBenefactorInstructionAccounts {
            authority,
            benefactor,
        },
        jup_stable::instructions::BenefactorManagementAction::ResetPeriodLimit { index },
    )
}

pub struct DeleteBenefactorInstructionAccounts {
    pub authority: Pubkey,
    pub receiver: Pubkey,
    pub benefactor: Pubkey,
}

pub fn create_delete_benefactor_instruction(
    accounts: DeleteBenefactorInstructionAccounts,
) -> Instruction {
    let accounts = jup_stable::accounts::DeleteBenefactor {
        operator_authority: accounts.authority,
        operator: find_operator(&accounts.authority),
        receiver: accounts.receiver,
        benefactor: accounts.benefactor,
    }
    .to_account_metas(Some(true));

    Instruction {
        program_id: jup_stable::id(),
        accounts,
        data: jup_stable::instruction::DeleteBenefactor {}.data(),
    }
}

pub struct CreateOperatorInstructionAccounts {
    pub operator_authority: Pubkey,
    pub payer: Pubkey,
    pub new_operator_authority: Pubkey,
}

pub fn create_create_operator_instruction(
    accounts: CreateOperatorInstructionAccounts,
    role: jup_stable::state::operator::OperatorRole,
) -> Instruction {
    let accounts = jup_stable::accounts::CreateOperator {
        operator_authority: accounts.operator_authority,
        payer: accounts.payer,
        operator: find_operator(&accounts.operator_authority),
        new_operator_authority: accounts.new_operator_authority,
        new_operator: find_operator(&accounts.new_operator_authority),
        system_program: system_program::ID,
    }
    .to_account_metas(Some(true));

    Instruction {
        program_id: jup_stable::id(),
        accounts,
        data: jup_stable::instruction::CreateOperator { role }.data(),
    }
}

pub struct DeleteOperatorInstructionAccounts {
    pub operator_authority: Pubkey,
    pub payer: Pubkey,
    pub deleted_operator: Pubkey,
}

pub fn create_delete_operator_instruction(
    accounts: DeleteOperatorInstructionAccounts,
) -> Instruction {
    let accounts = jup_stable::accounts::DeleteOperator {
        operator_authority: accounts.operator_authority,
        operator: find_operator(&accounts.operator_authority),
        payer: accounts.payer,
        deleted_operator: accounts.deleted_operator,
    }
    .to_account_metas(Some(true));

    Instruction {
        program_id: jup_stable::id(),
        accounts,
        data: jup_stable::instruction::DeleteOperator {}.data(),
    }
}

pub struct ManageOperatorInstructionAccounts {
    pub operator_authority: Pubkey,
    pub managed_operator: Pubkey,
}

pub fn create_manage_operator_instruction(
    accounts: ManageOperatorInstructionAccounts,
    action: jup_stable::instructions::OperatorManagementAction,
) -> Instruction {
    let accounts = jup_stable::accounts::ManageOperator {
        operator_authority: accounts.operator_authority,
        operator: find_operator(&accounts.operator_authority),
        managed_operator: accounts.managed_operator,
        system_program: system_program::ID,
    }
    .to_account_metas(Some(true));

    Instruction {
        program_id: jup_stable::id(),
        accounts,
        data: jup_stable::instruction::ManageOperator { action }.data(),
    }
}
