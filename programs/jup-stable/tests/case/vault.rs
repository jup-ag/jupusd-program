use anchor_spl::token::TokenAccount;
use fixtures::test::TestFixture;
use jup_stable::state::vault::{Vault, VaultStatus};
use solana_program_test::*;
use solana_sdk::{signature::Keypair, signer::Signer, transaction::Transaction};
use spl_associated_token_account::get_associated_token_address_with_program_id;
use switchboard_on_demand::Pubkey;

use crate::common::{
    constants::{USDC_DECIMALS, USDC_FEED_ID, USDC_MINT, USDC_ORACLE_CONFIG},
    derivation::{find_vault, find_vault_token_account},
    faciliter::{
        create_associated_token_account, create_vault, create_vault_with_oracle,
        setup_full_test_context,
    },
    instructions::{
        create_reset_vault_period_limit_instruction, create_set_custodian_instruction,
        create_set_max_oracle_price_instruction, create_set_min_oracle_price_instruction,
        create_set_stalesness_threshold_instruction, create_set_vault_status_instruction,
        create_update_vault_oracle_instruction, create_update_vault_period_limit_instruction,
        create_withdraw_instruction, WithdrawInstructionAccounts,
    },
};

#[tokio::test]
async fn create_vault_success() -> anyhow::Result<()> {
    let test_f = TestFixture::new().await;
    let _test_context = setup_full_test_context(&test_f).await?;

    let mint = USDC_MINT;
    create_vault(&test_f, mint).await?;

    let vault_pubkey = find_vault(&mint);
    let vault_token_account_pubkey = find_vault_token_account(&mint);
    // Verify vault account was created
    let vault_account: Vault = test_f.load_and_deserialize(&vault_pubkey).await;
    assert_eq!(
        vault_account.mint, mint,
        "Vault should have the correct stablecoin mint"
    );
    assert_eq!(
        vault_account.custodian,
        Pubkey::default(),
        "Vault should not have a custodian"
    );
    assert_eq!(
        vault_account.token_account, vault_token_account_pubkey,
        "Vault should have the correct token account"
    );
    assert_eq!(
        vault_account.stalesness_threshold, 300,
        "Vault should have the correct stalesness threshold"
    );
    assert_eq!(
        vault_account.min_oracle_price_usd, 5000,
        "Vault should have the correct min oracle price USD"
    );
    assert_eq!(
        vault_account.max_oracle_price_usd, 10000,
        "Vault should have the correct max oracle price USD"
    );
    assert_eq!(
        vault_account.status,
        VaultStatus::Disabled,
        "Vault should start as disabled"
    );
    assert_eq!(
        vault_account.token_program,
        spl_token::ID,
        "Vault should have the correct token program"
    );
    assert_eq!(
        vault_account.decimals, 6,
        "Vault should have the correct decimals"
    );
    assert_eq!(
        vault_account.total_minted, [0; 16],
        "Vault should have the correct total minted"
    );
    assert_eq!(
        vault_account.total_redeemed, [0; 16],
        "Vault should have the correct total redeemed"
    );

    Ok(())
}

#[tokio::test]
async fn set_vault_status_success() -> anyhow::Result<()> {
    let test_f = TestFixture::new().await;
    let deployer = test_f.deployer.pubkey();
    let _test_context = setup_full_test_context(&test_f).await?;

    let mint = USDC_MINT;
    create_vault(&test_f, mint).await?;

    let vault_pubkey = find_vault(&mint);

    let mut ctx = test_f.context.borrow_mut();
    let last_blockhash = ctx.get_new_latest_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(
        &[
            create_set_custodian_instruction(deployer, mint, deployer),
            create_update_vault_oracle_instruction(deployer, mint, 0, USDC_ORACLE_CONFIG),
            create_set_vault_status_instruction(deployer, mint, VaultStatus::Enabled),
        ],
        Some(&deployer),
        &[&test_f.deployer],
        last_blockhash,
    );
    ctx.banks_client.process_transaction(tx).await?;
    drop(ctx);

    let vault_account: Vault = test_f.load_and_deserialize(&vault_pubkey).await;
    assert_eq!(
        vault_account.status,
        VaultStatus::Enabled,
        "Vault status should be Enabled"
    );

    let mut ctx = test_f.context.borrow_mut();
    let last_blockhash = ctx.get_new_latest_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(
        &[create_set_vault_status_instruction(
            deployer,
            mint,
            VaultStatus::Disabled,
        )],
        Some(&deployer),
        &[&test_f.deployer],
        last_blockhash,
    );
    ctx.banks_client.process_transaction(tx).await?;
    drop(ctx);

    let vault_account: Vault = test_f.load_and_deserialize(&vault_pubkey).await;
    assert_eq!(
        vault_account.status,
        VaultStatus::Disabled,
        "Vault status should be Disabled"
    );

    Ok(())
}

#[tokio::test]
async fn set_custodian_success() -> anyhow::Result<()> {
    let test_f = TestFixture::new().await;
    let deployer = test_f.deployer.pubkey();
    let _test_context = setup_full_test_context(&test_f).await?;

    let mint = USDC_MINT;
    create_vault(&test_f, mint).await?;

    let vault_pubkey = find_vault(&mint);
    let new_custodian = Keypair::new();

    let mut ctx = test_f.context.borrow_mut();
    let last_blockhash = ctx.get_new_latest_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(
        &[create_set_custodian_instruction(
            deployer,
            mint,
            new_custodian.pubkey(),
        )],
        Some(&deployer),
        &[&test_f.deployer],
        last_blockhash,
    );
    ctx.banks_client.process_transaction(tx).await?;
    drop(ctx);

    let vault_account: Vault = test_f.load_and_deserialize(&vault_pubkey).await;
    assert_eq!(
        vault_account.custodian,
        new_custodian.pubkey(),
        "Custodian should be updated to new custodian"
    );

    Ok(())
}

#[tokio::test]
async fn update_oracle_success() -> anyhow::Result<()> {
    let test_f = TestFixture::new().await;
    let deployer = test_f.deployer.pubkey();
    let _test_context = setup_full_test_context(&test_f).await?;

    let mint = USDC_MINT;
    create_vault(&test_f, mint).await?;

    let vault_pubkey = find_vault(&mint);

    let mut ctx = test_f.context.borrow_mut();
    let last_blockhash = ctx.get_new_latest_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(
        &[create_update_vault_oracle_instruction(
            deployer,
            mint,
            0,
            USDC_ORACLE_CONFIG,
        )],
        Some(&deployer),
        &[&test_f.deployer],
        last_blockhash,
    );
    ctx.banks_client.process_transaction(tx).await?;
    drop(ctx);

    let vault_account: Vault = test_f.load_and_deserialize(&vault_pubkey).await;
    match vault_account.oracles[0] {
        jup_stable::state::vault::OracleType::Pyth(oracle) => {
            assert_eq!(oracle.feed_id, USDC_FEED_ID, "Pyth feed ID should match");
        },
        _ => panic!("Expected Pyth oracle"),
    }

    let switchboard_account = Keypair::new().pubkey();
    let mut ctx = test_f.context.borrow_mut();
    let last_blockhash = ctx.get_new_latest_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(
        &[create_update_vault_oracle_instruction(
            deployer,
            mint,
            1,
            jup_stable::instructions::OracleConfig::SwitchboardOnDemand(switchboard_account),
        )],
        Some(&deployer),
        &[&test_f.deployer],
        last_blockhash,
    );
    ctx.banks_client.process_transaction(tx).await?;
    drop(ctx);

    let vault_account: Vault = test_f.load_and_deserialize(&vault_pubkey).await;
    match vault_account.oracles[1] {
        jup_stable::state::vault::OracleType::SwitchboardOnDemand(oracle) => {
            assert_eq!(
                oracle.account, switchboard_account,
                "Switchboard account should match"
            );
        },
        _ => panic!("Expected Switchboard oracle"),
    }

    let mut ctx = test_f.context.borrow_mut();
    let last_blockhash = ctx.get_new_latest_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(
        &[create_update_vault_oracle_instruction(
            deployer,
            mint,
            0,
            jup_stable::instructions::OracleConfig::None,
        )],
        Some(&deployer),
        &[&test_f.deployer],
        last_blockhash,
    );
    ctx.banks_client.process_transaction(tx).await?;
    drop(ctx);

    let vault_account: Vault = test_f.load_and_deserialize(&vault_pubkey).await;
    match vault_account.oracles[0] {
        jup_stable::state::vault::OracleType::Empty(_) => {},
        _ => panic!("Expected Empty oracle"),
    }

    Ok(())
}

#[tokio::test]
async fn update_period_limit_success() -> anyhow::Result<()> {
    let test_f = TestFixture::new().await;
    let deployer = test_f.deployer.pubkey();
    let _test_context = setup_full_test_context(&test_f).await?;

    let mint = USDC_MINT;
    create_vault(&test_f, mint).await?;

    let vault_pubkey = find_vault(&mint);

    let duration_seconds = 3600u64; // 1 hour
    let max_mint_amount = 1_000_000u64; // 1M USDC
    let max_redeem_amount = 500_000u64; // 500K USDC

    let mut ctx = test_f.context.borrow_mut();
    let last_blockhash = ctx.get_new_latest_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(
        &[create_update_vault_period_limit_instruction(
            deployer,
            mint,
            0,
            duration_seconds,
            max_mint_amount,
            max_redeem_amount,
        )],
        Some(&deployer),
        &[&test_f.deployer],
        last_blockhash,
    );
    ctx.banks_client.process_transaction(tx).await?;
    drop(ctx);

    let vault_account: Vault = test_f.load_and_deserialize(&vault_pubkey).await;
    let period_limit = vault_account.period_limits[0];
    assert_eq!(
        period_limit.duration_seconds, duration_seconds,
        "Duration should match"
    );
    assert_eq!(
        period_limit.max_mint_amount, max_mint_amount,
        "Max mint amount should match"
    );
    assert_eq!(
        period_limit.max_redeem_amount, max_redeem_amount,
        "Max redeem amount should match"
    );

    let new_duration = 7200u64; // 2 hours
    let new_max_mint = 2_000_000u64;
    let new_max_redeem = 1_000_000u64;

    let mut ctx = test_f.context.borrow_mut();
    let last_blockhash = ctx.get_new_latest_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(
        &[create_update_vault_period_limit_instruction(
            deployer,
            mint,
            0,
            new_duration,
            new_max_mint,
            new_max_redeem,
        )],
        Some(&deployer),
        &[&test_f.deployer],
        last_blockhash,
    );
    ctx.banks_client.process_transaction(tx).await?;
    drop(ctx);

    let vault_account: Vault = test_f.load_and_deserialize(&vault_pubkey).await;
    let period_limit = vault_account.period_limits[0];
    assert_eq!(
        period_limit.duration_seconds, new_duration,
        "Duration should be updated"
    );
    assert_eq!(
        period_limit.max_mint_amount, new_max_mint,
        "Max mint amount should be updated"
    );
    assert_eq!(
        period_limit.max_redeem_amount, new_max_redeem,
        "Max redeem amount should be updated"
    );

    Ok(())
}

#[tokio::test]
async fn reset_period_limit_success() -> anyhow::Result<()> {
    let test_f = TestFixture::new().await;
    let deployer = test_f.deployer.pubkey();
    let _test_context = setup_full_test_context(&test_f).await?;

    let mint = USDC_MINT;
    create_vault(&test_f, mint).await?;

    let vault_pubkey = find_vault(&mint);

    let duration_seconds = 3600u64;
    let max_mint_amount = 1_000_000u64;
    let max_redeem_amount = 500_000u64;

    let mut ctx = test_f.context.borrow_mut();
    let last_blockhash = ctx.get_new_latest_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(
        &[create_update_vault_period_limit_instruction(
            deployer,
            mint,
            0,
            duration_seconds,
            max_mint_amount,
            max_redeem_amount,
        )],
        Some(&deployer),
        &[&test_f.deployer],
        last_blockhash,
    );
    ctx.banks_client.process_transaction(tx).await?;
    drop(ctx);

    let vault_account: Vault = test_f.load_and_deserialize(&vault_pubkey).await;
    let period_limit = vault_account.period_limits[0];
    assert_eq!(period_limit.duration_seconds, duration_seconds);

    let mut ctx = test_f.context.borrow_mut();
    let last_blockhash = ctx.get_new_latest_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(
        &[create_reset_vault_period_limit_instruction(
            deployer, mint, 0,
        )],
        Some(&deployer),
        &[&test_f.deployer],
        last_blockhash,
    );
    ctx.banks_client.process_transaction(tx).await?;
    drop(ctx);

    let vault_account: Vault = test_f.load_and_deserialize(&vault_pubkey).await;
    let period_limit = vault_account.period_limits[0];
    assert_eq!(
        period_limit.duration_seconds, 0,
        "Duration should be reset to 0"
    );
    assert_eq!(
        period_limit.max_mint_amount, 0,
        "Max mint amount should be reset to 0"
    );
    assert_eq!(
        period_limit.max_redeem_amount, 0,
        "Max redeem amount should be reset to 0"
    );

    Ok(())
}

#[tokio::test]
async fn set_stalesness_threshold_success() -> anyhow::Result<()> {
    let test_f = TestFixture::new().await;
    let deployer = test_f.deployer.pubkey();
    let _test_context = setup_full_test_context(&test_f).await?;

    let mint = USDC_MINT;
    create_vault(&test_f, mint).await?;

    let vault_pubkey = find_vault(&mint);
    let stalesness_threshold = 500;

    let mut ctx = test_f.context.borrow_mut();
    let last_blockhash = ctx.get_new_latest_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(
        &[create_set_stalesness_threshold_instruction(
            deployer,
            mint,
            stalesness_threshold,
        )],
        Some(&deployer),
        &[&test_f.deployer],
        last_blockhash,
    );
    ctx.banks_client.process_transaction(tx).await?;
    drop(ctx);

    let vault_account: Vault = test_f.load_and_deserialize(&vault_pubkey).await;
    assert_eq!(
        vault_account.stalesness_threshold, stalesness_threshold,
        "Stalesness threshold should be updated"
    );

    Ok(())
}

#[tokio::test]
async fn set_oracle_price_success() -> anyhow::Result<()> {
    let test_f = TestFixture::new().await;
    let deployer = test_f.deployer.pubkey();
    let _test_context = setup_full_test_context(&test_f).await?;

    let mint = USDC_MINT;
    create_vault(&test_f, mint).await?;

    let vault_pubkey = find_vault(&mint);
    let min_oracle_price_usd = 2000;
    let max_oracle_price_usd = 12000;

    let mut ctx = test_f.context.borrow_mut();
    let last_blockhash = ctx.get_new_latest_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(
        &[
            create_set_min_oracle_price_instruction(deployer, mint, min_oracle_price_usd),
            create_set_max_oracle_price_instruction(deployer, mint, max_oracle_price_usd),
        ],
        Some(&deployer),
        &[&test_f.deployer],
        last_blockhash,
    );
    ctx.banks_client.process_transaction(tx).await?;
    drop(ctx);

    let vault_account: Vault = test_f.load_and_deserialize(&vault_pubkey).await;
    assert_eq!(
        vault_account.min_oracle_price_usd, min_oracle_price_usd,
        "Min oracle price USD should be updated"
    );
    assert_eq!(
        vault_account.max_oracle_price_usd, max_oracle_price_usd,
        "Max oracle price USD should be updated"
    );

    Ok(())
}

#[tokio::test]
async fn withdraw_collateral_success() -> anyhow::Result<()> {
    let test_f = TestFixture::new().await;
    let deployer = test_f.deployer.pubkey();
    let _test_context = setup_full_test_context(&test_f).await?;

    let mint = USDC_MINT;
    let custodian: Keypair = Keypair::new();
    create_vault_with_oracle(&test_f, mint, custodian.pubkey(), USDC_ORACLE_CONFIG).await?;

    let custodian_collateral_ata =
        get_associated_token_address_with_program_id(&custodian.pubkey(), &mint, &spl_token::ID);

    create_associated_token_account(&test_f, &custodian.pubkey(), &mint).await?;

    let amount = 1000 * 10_u64.pow(USDC_DECIMALS.into());
    test_f
        .mint_tokens(&find_vault_token_account(&mint), amount)
        .await;

    let mut ctx = test_f.context.borrow_mut();
    let last_blockhash = ctx.get_new_latest_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(
        &[create_withdraw_instruction(
            WithdrawInstructionAccounts {
                operator_authority: deployer,
                custodian: custodian.pubkey(),
                vault_mint: mint,
                vault_token_program: spl_token::ID,
            },
            amount,
        )],
        Some(&deployer),
        &[&test_f.deployer],
        last_blockhash,
    );
    ctx.banks_client.process_transaction(tx).await?;
    drop(ctx);

    let custodian_token_account: TokenAccount =
        test_f.load_and_deserialize(&custodian_collateral_ata).await;
    assert!(
        custodian_token_account.amount == amount,
        "Custodian's balance should be equal to the withdrawn amount: {} >= {}",
        custodian_token_account.amount,
        amount
    );

    Ok(())
}
