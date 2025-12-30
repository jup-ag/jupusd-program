use anchor_spl::token_interface::TokenAccount;
use fixtures::test::TestFixture;
use psm::state::pool::{Pool, PoolStatus};
use solana_program_test::*;
use solana_sdk::{signer::Signer, transaction::Transaction};

use crate::common::{
    constants::{USDC_DECIMALS, USDC_MINT, USDT_DECIMALS, USDT_MINT},
    derivation::{
        find_pool, find_pool_redemption_token_account, find_pool_settlement_token_account,
    },
    faciliter::{create_pool, setup_full_test_context},
    instructions::create_set_pool_status_instruction,
};

#[tokio::test]
async fn create_pool_success() -> anyhow::Result<()> {
    let test_f = TestFixture::new().await;
    let _test_context = setup_full_test_context(&test_f, USDC_MINT, USDT_MINT).await?;

    create_pool(&test_f, USDC_MINT, USDT_MINT).await?;

    let pool_address = find_pool(&USDC_MINT, &USDT_MINT);
    let pool: Pool = test_f.load_and_deserialize(&pool_address).await;
    let redemption_token_account = find_pool_redemption_token_account(&pool_address);
    let settlement_token_account = find_pool_settlement_token_account(&pool_address);

    assert_eq!(
        pool.redemption_mint, USDC_MINT,
        "Pool should have correct redemption mint"
    );
    assert_eq!(
        pool.settlement_mint, USDT_MINT,
        "Pool should have correct settlement mint"
    );
    assert_eq!(
        pool.status,
        PoolStatus::Disabled,
        "Pool should be disabled initially"
    );
    assert_eq!(
        pool.redemption_token_decimals, USDC_DECIMALS,
        "Pool should have correct redemption token decimals"
    );
    assert_eq!(
        pool.settlement_token_decimals, USDT_DECIMALS,
        "Pool should have correct settlement token decimals"
    );
    assert_eq!(
        pool.redemption_token_account,
        find_pool_redemption_token_account(&pool_address),
        "Pool should have correct redemption token account"
    );
    assert_eq!(
        pool.settlement_token_account,
        find_pool_settlement_token_account(&pool_address),
        "Pool should have correct settlement token account"
    );
    assert!(pool.bump != 0, "Pool should have non-zero bump");
    assert_eq!(
        u128::from_le_bytes(pool.total_redeemed),
        0,
        "Pool should have zero total redeemed initially"
    );
    assert_eq!(
        u128::from_le_bytes(pool.total_supplied),
        0,
        "Pool should have zero total supplied initially"
    );

    let redemption_account: TokenAccount =
        test_f.load_and_deserialize(&redemption_token_account).await;
    assert_eq!(
        redemption_account.mint, USDC_MINT,
        "Redemption token account should have correct mint"
    );
    assert_eq!(
        redemption_account.amount, 0,
        "Redemption token account should have zero balance"
    );

    let settlement_account: TokenAccount =
        test_f.load_and_deserialize(&settlement_token_account).await;
    assert_eq!(
        settlement_account.mint, USDT_MINT,
        "Settlement token account should have correct mint"
    );
    assert_eq!(
        settlement_account.amount, 0,
        "Settlement token account should have zero balance"
    );

    Ok(())
}

#[tokio::test]
async fn set_pool_status_success() -> anyhow::Result<()> {
    let test_f = TestFixture::new().await;
    let _test_context = setup_full_test_context(&test_f, USDC_MINT, USDT_MINT).await?;

    create_pool(&test_f, USDC_MINT, USDT_MINT).await?;

    let pool_address = find_pool(&USDC_MINT, &USDT_MINT);
    let payer = test_f.deployer.pubkey();

    {
        let mut ctx = test_f.context.borrow_mut();
        let last_blockhash = ctx.get_new_latest_blockhash().await?;
        let tx = Transaction::new_signed_with_payer(
            &[create_set_pool_status_instruction(
                payer,
                USDC_MINT,
                USDT_MINT,
                PoolStatus::Active,
            )],
            Some(&payer),
            &[&test_f.deployer],
            last_blockhash,
        );

        ctx.banks_client.process_transaction(tx).await?;
    }

    let pool: Pool = test_f.load_and_deserialize(&pool_address).await;
    assert_eq!(pool.status, PoolStatus::Active, "Pool should be active");

    {
        let mut ctx = test_f.context.borrow_mut();
        let last_blockhash = ctx.get_new_latest_blockhash().await?;
        let tx = Transaction::new_signed_with_payer(
            &[create_set_pool_status_instruction(
                payer,
                USDC_MINT,
                USDT_MINT,
                PoolStatus::Paused,
            )],
            Some(&payer),
            &[&test_f.deployer],
            last_blockhash,
        );

        ctx.banks_client.process_transaction(tx).await?;
    }

    let pool: Pool = test_f.load_and_deserialize(&pool_address).await;
    assert_eq!(pool.status, PoolStatus::Paused, "Pool should be paused");

    // Set pool to disabled
    {
        let mut ctx = test_f.context.borrow_mut();
        let last_blockhash = ctx.get_new_latest_blockhash().await?;
        let tx = Transaction::new_signed_with_payer(
            &[create_set_pool_status_instruction(
                payer,
                USDC_MINT,
                USDT_MINT,
                PoolStatus::Disabled,
            )],
            Some(&payer),
            &[&test_f.deployer],
            last_blockhash,
        );

        ctx.banks_client.process_transaction(tx).await?;
    }

    let pool: Pool = test_f.load_and_deserialize(&pool_address).await;
    assert_eq!(pool.status, PoolStatus::Disabled, "Pool should be disabled");

    Ok(())
}
