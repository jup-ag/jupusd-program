use fixtures::test::TestFixture;
use jup_stable::state::benefactor::{Benefactor, BenefactorStatus};
use solana_program_test::*;
use solana_sdk::{signature::Keypair, signer::Signer, transaction::Transaction};

use super::super::common::instructions::create_create_benefactor_instruction;
use crate::common::{
    constants::USDC_MINT,
    derivation::find_benefactor,
    faciliter::{create_benefactor, create_vault, setup_full_test_context},
    instructions::{
        create_delete_benefactor_instruction, create_set_benefactor_status_instruction,
        create_update_benefactor_period_limit_instruction, create_update_fee_rates_instruction,
        CreateBenefactorInstructionAccounts, CreateBenefactorInstructionArgs,
        DeleteBenefactorInstructionAccounts,
    },
};

#[tokio::test]
async fn create_benefactor_success() -> anyhow::Result<()> {
    let test_f = TestFixture::new().await;
    let deployer = test_f.deployer.pubkey();
    let _test_context = setup_full_test_context(&test_f).await?;

    let mint = USDC_MINT;
    create_vault(&test_f, mint).await?;

    let benefactor_authority = Keypair::new();
    let mint_fee_rate = 100u16;
    let redeem_fee_rate = 50u16;

    let mut ctx = test_f.context.borrow_mut();
    let last_blockhash = ctx.get_new_latest_blockhash().await?;
    drop(ctx);
    let tx = Transaction::new_signed_with_payer(
        &[create_create_benefactor_instruction(
            CreateBenefactorInstructionAccounts {
                authority: deployer,
                payer: deployer,
                benefactor_authority: benefactor_authority.pubkey(),
            },
            CreateBenefactorInstructionArgs {
                mint_fee_rate,
                redeem_fee_rate,
            },
        )],
        Some(&deployer),
        &[&test_f.deployer],
        last_blockhash,
    );
    test_f
        .context
        .borrow_mut()
        .banks_client
        .process_transaction(tx)
        .await?;

    let benefactor_account: Benefactor = test_f
        .load_and_deserialize(&find_benefactor(&benefactor_authority.pubkey()))
        .await;

    assert_eq!(
        benefactor_account.authority,
        benefactor_authority.pubkey(),
        "Benefactor should have the correct authority"
    );
    assert!(
        benefactor_account.status == BenefactorStatus::Disabled,
        "Benefactor should start as disabled"
    );
    assert_eq!(
        benefactor_account.mint_fee_rate, mint_fee_rate,
        "Benefactor should have the correct mint fee rate"
    );
    assert_eq!(
        benefactor_account.redeem_fee_rate, redeem_fee_rate,
        "Benefactor should have the correct redeem fee rate"
    );

    Ok(())
}

#[tokio::test]
async fn set_benefactor_status_success() -> anyhow::Result<()> {
    let test_f = TestFixture::new().await;
    let deployer = test_f.deployer.pubkey();
    let _test_context = setup_full_test_context(&test_f).await?;

    let mint = USDC_MINT;
    let benefactor_authority = Keypair::new();

    create_vault(&test_f, mint).await?;
    let benefactor_pubkey =
        create_benefactor(&test_f, &benefactor_authority.pubkey(), 100u16, 50u16).await?;

    {
        let mut ctx = test_f.context.borrow_mut();
        let last_blockhash = ctx.get_new_latest_blockhash().await?;
        let tx = Transaction::new_signed_with_payer(
            &[create_set_benefactor_status_instruction(
                deployer,
                benefactor_pubkey,
                BenefactorStatus::Active,
            )],
            Some(&deployer),
            &[&test_f.deployer],
            last_blockhash,
        );
        ctx.banks_client.process_transaction(tx).await?;
    }

    let benefactor_account: Benefactor = test_f.load_and_deserialize(&benefactor_pubkey).await;
    assert!(
        benefactor_account.status == BenefactorStatus::Active,
        "Benefactor status should be Active"
    );

    {
        let mut ctx = test_f.context.borrow_mut();
        let last_blockhash = ctx.get_new_latest_blockhash().await?;
        let tx = Transaction::new_signed_with_payer(
            &[create_set_benefactor_status_instruction(
                deployer,
                benefactor_pubkey,
                BenefactorStatus::Disabled,
            )],
            Some(&deployer),
            &[&test_f.deployer],
            last_blockhash,
        );
        ctx.banks_client.process_transaction(tx).await?;
    }

    let benefactor_account: Benefactor = test_f.load_and_deserialize(&benefactor_pubkey).await;
    assert!(
        benefactor_account.status == BenefactorStatus::Disabled,
        "Benefactor status should be Disabled"
    );

    Ok(())
}

#[tokio::test]
async fn update_fee_rates_success() -> anyhow::Result<()> {
    let test_f = TestFixture::new().await;
    let deployer = test_f.deployer.pubkey();
    let _test_context = setup_full_test_context(&test_f).await?;

    let mint = USDC_MINT;
    let benefactor_authority = Keypair::new();

    create_vault(&test_f, mint).await?;
    let benefactor_pubkey =
        create_benefactor(&test_f, &benefactor_authority.pubkey(), 100u16, 50u16).await?;

    let new_mint_fee_rate = 200u16;
    let new_redeem_fee_rate = 150u16;

    {
        let mut ctx = test_f.context.borrow_mut();
        let last_blockhash = ctx.get_new_latest_blockhash().await?;
        let tx = Transaction::new_signed_with_payer(
            &[create_update_fee_rates_instruction(
                deployer,
                benefactor_pubkey,
                new_mint_fee_rate,
                new_redeem_fee_rate,
            )],
            Some(&deployer),
            &[&test_f.deployer],
            last_blockhash,
        );
        ctx.banks_client.process_transaction(tx).await?;
    }

    let benefactor_account: Benefactor = test_f.load_and_deserialize(&benefactor_pubkey).await;
    assert_eq!(
        benefactor_account.mint_fee_rate, new_mint_fee_rate,
        "Mint fee rate should be updated"
    );
    assert_eq!(
        benefactor_account.redeem_fee_rate, new_redeem_fee_rate,
        "Redeem fee rate should be updated"
    );

    let updated_mint_fee_rate = 300u16;
    let updated_redeem_fee_rate = 250u16;

    {
        let mut ctx = test_f.context.borrow_mut();
        let last_blockhash = ctx.get_new_latest_blockhash().await?;
        let tx = Transaction::new_signed_with_payer(
            &[create_update_fee_rates_instruction(
                deployer,
                benefactor_pubkey,
                updated_mint_fee_rate,
                updated_redeem_fee_rate,
            )],
            Some(&deployer),
            &[&test_f.deployer],
            last_blockhash,
        );
        ctx.banks_client.process_transaction(tx).await?;
    }

    let benefactor_account: Benefactor = test_f.load_and_deserialize(&benefactor_pubkey).await;
    assert_eq!(
        benefactor_account.mint_fee_rate, updated_mint_fee_rate,
        "Mint fee rate should be updated again"
    );
    assert_eq!(
        benefactor_account.redeem_fee_rate, updated_redeem_fee_rate,
        "Redeem fee rate should be updated again"
    );

    Ok(())
}

#[tokio::test]
async fn update_benefactor_period_limit_success() -> anyhow::Result<()> {
    let test_f = TestFixture::new().await;
    let deployer = test_f.deployer.pubkey();
    let _test_context = setup_full_test_context(&test_f).await?;

    let mint = USDC_MINT;
    let benefactor_authority = Keypair::new();

    create_vault(&test_f, mint).await?;
    let benefactor_pubkey =
        create_benefactor(&test_f, &benefactor_authority.pubkey(), 100u16, 50u16).await?;

    let duration_seconds = 3600u64;
    let max_mint_amount = 1_000_000u64;
    let max_redeem_amount = 500_000u64;

    {
        let mut ctx = test_f.context.borrow_mut();
        let last_blockhash = ctx.get_new_latest_blockhash().await?;
        let tx = Transaction::new_signed_with_payer(
            &[create_update_benefactor_period_limit_instruction(
                deployer,
                benefactor_pubkey,
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
    }

    let benefactor_account: Benefactor = test_f.load_and_deserialize(&benefactor_pubkey).await;
    let period_limit = benefactor_account.period_limits[0];
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

    let new_duration = 7200u64;
    let new_max_mint = 2_000_000u64;
    let new_max_redeem = 1_000_000u64;

    {
        let mut ctx = test_f.context.borrow_mut();
        let last_blockhash = ctx.get_new_latest_blockhash().await?;
        let tx = Transaction::new_signed_with_payer(
            &[create_update_benefactor_period_limit_instruction(
                deployer,
                benefactor_pubkey,
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
    }

    let benefactor_account: Benefactor = test_f.load_and_deserialize(&benefactor_pubkey).await;
    let period_limit = benefactor_account.period_limits[0];
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
async fn delete_benefactor_success() -> anyhow::Result<()> {
    let test_f = TestFixture::new().await;
    let deployer = test_f.deployer.pubkey();
    let _test_context = setup_full_test_context(&test_f).await?;

    let mint = USDC_MINT;
    let benefactor_authority = Keypair::new();

    create_vault(&test_f, mint).await?;
    let benefactor_pubkey =
        create_benefactor(&test_f, &benefactor_authority.pubkey(), 100u16, 50u16).await?;

    {
        let ctx = test_f.context.borrow_mut();
        let account = ctx.banks_client.get_account(benefactor_pubkey).await?;
        assert!(account.is_some(), "Benefactor account should exist");
    }

    {
        let mut ctx = test_f.context.borrow_mut();
        let last_blockhash = ctx.get_new_latest_blockhash().await?;
        let tx = Transaction::new_signed_with_payer(
            &[create_delete_benefactor_instruction(
                DeleteBenefactorInstructionAccounts {
                    authority: deployer,
                    receiver: deployer,
                    benefactor: benefactor_pubkey,
                },
            )],
            Some(&deployer),
            &[&test_f.deployer],
            last_blockhash,
        );
        ctx.banks_client.process_transaction(tx).await?;
    }

    let ctx = test_f.context.borrow_mut();
    let account = ctx.banks_client.get_account(benefactor_pubkey).await?;
    assert!(account.is_none(), "Benefactor account should be deleted");

    Ok(())
}
