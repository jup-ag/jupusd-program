use fixtures::test::TestFixture;
use jup_stable::state::config::Config;
use solana_program_test::*;
use solana_sdk::{signature::Keypair, signer::Signer, transaction::Transaction};

use crate::common::{
    derivation::find_config,
    faciliter::setup_full_test_context,
    instructions::{
        create_reset_config_period_limit_instruction, create_update_config_period_limit_instruction,
    },
};

#[tokio::test]
async fn update_config_period_limit_success() -> anyhow::Result<()> {
    let test_f = TestFixture::new().await;
    let _test_context = setup_full_test_context(&test_f).await?;
    let deployer = test_f.deployer.pubkey();

    let duration_seconds = 3600u64; // 1 hour
    let max_mint_amount = 10_000_000u64; // 10M
    let max_redeem_amount = 5_000_000u64; // 5M

    {
        let mut ctx = test_f.context.borrow_mut();
        let last_blockhash = ctx.get_new_latest_blockhash().await?;
        let tx = Transaction::new_signed_with_payer(
            &[create_update_config_period_limit_instruction(
                deployer,
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

    let config_account: Config = test_f.load_and_deserialize(&find_config()).await;
    let period_limit = config_account.period_limits[0];
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
    let new_max_mint = 20_000_000u64;
    let new_max_redeem = 10_000_000u64;

    {
        let mut ctx = test_f.context.borrow_mut();
        let last_blockhash = ctx.get_new_latest_blockhash().await?;
        let tx = Transaction::new_signed_with_payer(
            &[create_update_config_period_limit_instruction(
                deployer,
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

    let config_account: Config = test_f.load_and_deserialize(&find_config()).await;
    let period_limit = config_account.period_limits[0];
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
async fn reset_config_period_limit_success() -> anyhow::Result<()> {
    let test_f = TestFixture::new().await;
    let _test_context = setup_full_test_context(&test_f).await?;
    let deployer = test_f.deployer.pubkey();

    let duration_seconds = 3600u64;
    let max_mint_amount = 10_000_000u64;
    let max_redeem_amount = 5_000_000u64;

    {
        let mut ctx = test_f.context.borrow_mut();
        let last_blockhash = ctx.get_new_latest_blockhash().await?;
        let tx = Transaction::new_signed_with_payer(
            &[create_update_config_period_limit_instruction(
                deployer,
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

    let config_account: Config = test_f.load_and_deserialize(&find_config()).await;
    let period_limit = config_account.period_limits[0];
    assert_eq!(period_limit.duration_seconds, duration_seconds);

    {
        let mut ctx = test_f.context.borrow_mut();
        let last_blockhash = ctx.get_new_latest_blockhash().await?;
        let tx = Transaction::new_signed_with_payer(
            &[create_reset_config_period_limit_instruction(deployer, 0)],
            Some(&deployer),
            &[&test_f.deployer],
            last_blockhash,
        );

        ctx.banks_client.process_transaction(tx).await?;
    }

    let config_account: Config = test_f.load_and_deserialize(&find_config()).await;
    let period_limit = config_account.period_limits[0];
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
async fn update_config_period_limit_fails_when_not_admin() -> anyhow::Result<()> {
    let test_f = TestFixture::new().await;
    let _test_context = setup_full_test_context(&test_f).await?;

    let unauthorized_user = Keypair::new();
    test_f.fund_account(&unauthorized_user.pubkey()).await;

    {
        let mut ctx = test_f.context.borrow_mut();
        let last_blockhash = ctx.get_new_latest_blockhash().await?;
        let tx = Transaction::new_signed_with_payer(
            &[create_update_config_period_limit_instruction(
                unauthorized_user.pubkey(),
                0,
                3600,
                1_000_000,
                500_000,
            )],
            Some(&unauthorized_user.pubkey()),
            &[&unauthorized_user],
            last_blockhash,
        );

        let result = ctx.banks_client.process_transaction(tx).await;
        assert!(
            result.is_err(),
            "Transaction should fail when called by non-admin"
        );
    }

    Ok(())
}
