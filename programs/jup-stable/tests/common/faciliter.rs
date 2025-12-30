use anchor_lang::AnchorSerialize;
use anyhow::Result;
use fixtures::test::TestFixture;
use jup_stable::state::{benefactor::BenefactorStatus, vault::VaultStatus};
use pyth_solana_receiver_sdk::price_update::PriceUpdateV2;
use solana_instruction::Instruction;
use solana_sdk::{
    bpf_loader_upgradeable::get_program_data_address, pubkey::Pubkey, signature::Keypair,
    signer::Signer, transaction::Transaction,
};

use crate::common::{
    constants::{JUPUSD_DECIMALS, JUPUSD_NAME, JUPUSD_SYMBOL, JUPUSD_URI, USDC_MINT},
    derivation::find_benefactor,
    instructions::{
        create_create_benefactor_instruction, create_create_vault_instruction,
        create_init_instruction, create_mint_instruction, create_redeem_instruction,
        create_set_benefactor_status_instruction, create_set_custodian_instruction,
        create_set_vault_status_instruction, create_update_benefactor_period_limit_instruction,
        create_update_config_period_limit_instruction, create_update_pause_flag_instruction,
        create_update_vault_oracle_instruction, create_update_vault_period_limit_instruction,
        CreateBenefactorInstructionAccounts, CreateBenefactorInstructionArgs,
        CreateVaultInstructionAccounts, InitInstructionAccounts, InitInstructionArgs,
        MintInstructionAccounts, RedeemInstructionAccounts,
    },
};

pub async fn init_program(test_f: &TestFixture, mint: &Keypair) -> Result<()> {
    let payer = test_f.deployer.pubkey();
    let program_data = get_program_data_address(&jup_stable::ID);

    let accounts = InitInstructionAccounts {
        payer,
        upgrade_authority: test_f.deployer.pubkey(),
        program_data,
        mint: mint.pubkey(),
        token_program: spl_token::ID,
    };

    let args = InitInstructionArgs {
        decimals: JUPUSD_DECIMALS,
        name: JUPUSD_NAME.to_string(),
        symbol: JUPUSD_SYMBOL.to_string(),
        uri: JUPUSD_URI.to_string(),
    };

    let mut ctx = test_f.context.borrow_mut();
    let last_blockhash = ctx.get_new_latest_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(
        &[
            create_init_instruction(accounts, args),
            create_update_pause_flag_instruction(test_f.deployer.pubkey(), true),
        ],
        Some(&payer),
        &[&test_f.deployer, mint],
        last_blockhash,
    );

    ctx.banks_client.process_transaction(tx).await?;

    Ok(())
}

pub struct TestContext {
    pub lp_mint: Pubkey,
}

pub async fn create_vault(test_f: &TestFixture, vault_mint: Pubkey) -> Result<()> {
    let payer = test_f.deployer.pubkey();

    let accounts = CreateVaultInstructionAccounts {
        authority: payer,
        payer,
        mint: vault_mint,
        token_program: spl_token::ID,
    };

    let mut ctx = test_f.context.borrow_mut();
    let last_blockhash = ctx.get_new_latest_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(
        &[create_create_vault_instruction(accounts)],
        Some(&payer),
        &[&test_f.deployer],
        last_blockhash,
    );

    ctx.banks_client.process_transaction(tx).await?;

    Ok(())
}

pub async fn create_vault_with_oracle(
    test_f: &TestFixture,
    vault_mint: Pubkey,
    custodian: Pubkey,
    oracle: jup_stable::instructions::OracleConfig,
) -> Result<()> {
    let payer = test_f.deployer.pubkey();

    let accounts = CreateVaultInstructionAccounts {
        authority: payer,
        payer,
        mint: vault_mint,
        token_program: spl_token::ID,
    };

    let mut ctx = test_f.context.borrow_mut();
    let last_blockhash = ctx.get_new_latest_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(
        &[
            create_create_vault_instruction(accounts),
            create_set_custodian_instruction(payer, vault_mint, custodian),
            create_update_vault_oracle_instruction(payer, vault_mint, 0, oracle),
            create_set_vault_status_instruction(payer, vault_mint, VaultStatus::Enabled),
        ],
        Some(&payer),
        &[&test_f.deployer],
        last_blockhash,
    );
    ctx.banks_client.process_transaction(tx).await?;
    drop(ctx);

    Ok(())
}

pub async fn refresh_pyth_feed(test_f: &TestFixture, price_account: Pubkey) -> Result<()> {
    let mut oracle = test_f
        .load_and_deserialize::<PriceUpdateV2>(&price_account)
        .await;

    let clock = test_f.get_clock().await;
    oracle.price_message.publish_time = clock.unix_timestamp;

    let data = oracle.try_to_vec().unwrap();
    test_f.patch_account(price_account, 8, &data).await;

    Ok(())
}

pub async fn create_associated_token_account(
    test_f: &TestFixture,
    owner: &Pubkey,
    mint: &Pubkey,
) -> Result<()> {
    let payer = test_f.deployer.pubkey();

    let mut ctx = test_f.context.borrow_mut();
    let last_blockhash = ctx.get_new_latest_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(
        &[
            spl_associated_token_account::instruction::create_associated_token_account(
                &test_f.deployer.pubkey(),
                owner,
                mint,
                &spl_token::ID,
            ),
        ],
        Some(&payer),
        &[&test_f.deployer],
        last_blockhash,
    );

    ctx.banks_client.process_transaction(tx).await?;

    Ok(())
}

pub async fn create_benefactor(
    test_f: &TestFixture,
    benefactor_authority: &Pubkey,
    mint_fee_rate: u16,
    redeem_fee_rate: u16,
) -> Result<Pubkey> {
    let payer = test_f.deployer.pubkey();

    let accounts = CreateBenefactorInstructionAccounts {
        authority: payer,
        payer,
        benefactor_authority: *benefactor_authority,
    };

    let args = CreateBenefactorInstructionArgs {
        mint_fee_rate,
        redeem_fee_rate,
    };

    let mut ctx = test_f.context.borrow_mut();
    let last_blockhash = ctx.get_new_latest_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(
        &[create_create_benefactor_instruction(accounts, args)],
        Some(&payer),
        &[&test_f.deployer],
        last_blockhash,
    );

    ctx.banks_client.process_transaction(tx).await?;

    Ok(find_benefactor(benefactor_authority))
}

pub async fn create_active_benefactor(
    test_f: &TestFixture,
    benefactor_authority: &Pubkey,
    mint_fee_rate: u16,
    redeem_fee_rate: u16,
) -> Result<Pubkey> {
    let payer = test_f.deployer.pubkey();

    let accounts = CreateBenefactorInstructionAccounts {
        authority: payer,
        payer,
        benefactor_authority: *benefactor_authority,
    };

    let args = CreateBenefactorInstructionArgs {
        mint_fee_rate,
        redeem_fee_rate,
    };

    let benefactor = find_benefactor(benefactor_authority);

    let mut ctx = test_f.context.borrow_mut();
    let last_blockhash = ctx.get_new_latest_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(
        &[
            create_create_benefactor_instruction(accounts, args),
            create_set_benefactor_status_instruction(payer, benefactor, BenefactorStatus::Active),
        ],
        Some(&payer),
        &[&test_f.deployer],
        last_blockhash,
    );

    ctx.banks_client.process_transaction(tx).await?;

    Ok(benefactor)
}

pub enum PeriodLimitTarget {
    Config,
    Vault(Pubkey),
    Benefactor(Pubkey),
}

pub struct PeriodLimitArgs {
    pub target: PeriodLimitTarget,
    pub index: u8,
    pub duration_seconds: u64,
    pub max_mint_amount: u64,
    pub max_redeem_amount: u64,
}

pub async fn set_period_limit(test_f: &TestFixture, args: Vec<PeriodLimitArgs>) -> Result<()> {
    let instructions = args
        .iter()
        .map(|arg| match arg.target {
            PeriodLimitTarget::Config => create_update_config_period_limit_instruction(
                test_f.deployer.pubkey(),
                arg.index,
                arg.duration_seconds,
                arg.max_mint_amount,
                arg.max_redeem_amount,
            ),
            PeriodLimitTarget::Vault(mint) => create_update_vault_period_limit_instruction(
                test_f.deployer.pubkey(),
                mint,
                arg.index,
                arg.duration_seconds,
                arg.max_mint_amount,
                arg.max_redeem_amount,
            ),
            PeriodLimitTarget::Benefactor(pubkey) => {
                create_update_benefactor_period_limit_instruction(
                    test_f.deployer.pubkey(),
                    pubkey,
                    arg.index,
                    arg.duration_seconds,
                    arg.max_mint_amount,
                    arg.max_redeem_amount,
                )
            },
        })
        .collect::<Vec<Instruction>>();

    let mut ctx = test_f.context.borrow_mut();
    let last_blockhash = ctx.get_new_latest_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(
        &instructions,
        Some(&test_f.deployer.pubkey()),
        &[&test_f.deployer],
        last_blockhash,
    );
    ctx.banks_client.process_transaction(tx).await?;
    Ok(())
}

pub struct MintRedeemParams {
    pub user: Keypair,
    pub benefactor: Pubkey,
    pub custodian: Pubkey,
    pub vault_mint: Pubkey,
    pub lp_mint: Pubkey,
    pub vault_token_program: Option<Pubkey>,
    pub lp_token_program: Option<Pubkey>,
    pub remaining_accounts: Vec<Pubkey>,
}

pub async fn mint_stablecoin(
    test_f: &TestFixture,
    params: &MintRedeemParams,
    amount: u64,
    min_amount_out: u64,
) -> Result<()> {
    let accounts = MintInstructionAccounts {
        user: params.user.pubkey(),
        benefactor: params.benefactor,
        custodian: params.custodian,
        vault_mint: params.vault_mint,
        lp_mint: params.lp_mint,
        vault_token_program: params.vault_token_program.unwrap_or(spl_token::ID),
        lp_token_program: params.lp_token_program.unwrap_or(spl_token::ID),
        remaining_accounts: params.remaining_accounts.clone(),
    };

    let mut ctx = test_f.context.borrow_mut();
    let last_blockhash = ctx.get_new_latest_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(
        &[create_mint_instruction(amount, min_amount_out, accounts)],
        Some(&params.user.pubkey()),
        &[&params.user],
        last_blockhash,
    );

    ctx.banks_client.process_transaction(tx).await?;
    Ok(())
}

pub async fn redeem_stablecoin(
    test_f: &TestFixture,
    params: &MintRedeemParams,
    amount: u64,
    min_amount_out: u64,
) -> Result<()> {
    let accounts = RedeemInstructionAccounts {
        user: params.user.pubkey(),
        benefactor: params.benefactor,
        vault_mint: params.vault_mint,
        lp_mint: params.lp_mint,
        vault_token_program: params.vault_token_program.unwrap_or(spl_token::ID),
        lp_token_program: params.lp_token_program.unwrap_or(spl_token::ID),
        remaining_accounts: params.remaining_accounts.clone(),
    };

    let mut ctx = test_f.context.borrow_mut();
    let last_blockhash = ctx.get_new_latest_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(
        &[create_redeem_instruction(amount, min_amount_out, accounts)],
        Some(&params.user.pubkey()),
        &[&params.user],
        last_blockhash,
    );

    ctx.banks_client.process_transaction(tx).await?;
    Ok(())
}

pub async fn setup_full_test_context(test_f: &TestFixture) -> Result<TestContext> {
    let lp_mint = Keypair::new();

    test_f.replicate_account_from_mainnet(&USDC_MINT).await?;
    init_program(test_f, &lp_mint).await?;

    Ok(TestContext {
        lp_mint: lp_mint.pubkey(),
    })
}
