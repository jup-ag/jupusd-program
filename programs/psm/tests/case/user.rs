use anchor_spl::token_interface::TokenAccount;
use fixtures::test::TestFixture;
use psm::state::pool::Pool;
use solana_program_test::*;
use solana_sdk::{signature::Keypair, signer::Signer};
use spl_associated_token_account::get_associated_token_address_with_program_id;

use crate::common::{
    constants::{MSOL_DECIMALS, MSOL_MINT, USDC_DECIMALS, USDC_MINT, USDT_DECIMALS, USDT_MINT},
    derivation::{
        find_pool, find_pool_redemption_token_account, find_pool_settlement_token_account,
    },
    faciliter::{
        create_active_pool, create_associated_token_account, redeem_from_pool,
        setup_full_test_context, supply_pool, withdraw_from_pool,
    },
};

#[tokio::test]
async fn supply_success() -> anyhow::Result<()> {
    let test_f = TestFixture::new().await;
    let _test_context = setup_full_test_context(&test_f, USDC_MINT, USDT_MINT).await?;

    create_active_pool(&test_f, USDC_MINT, USDT_MINT).await?;

    let admin = &test_f.deployer;
    let admin_redemption_ata =
        get_associated_token_address_with_program_id(&admin.pubkey(), &USDC_MINT, &spl_token::ID);

    create_associated_token_account(&test_f, &admin.pubkey(), &USDC_MINT).await?;

    let supply_amount = 1000 * 10_u64.pow(USDC_DECIMALS.into());
    test_f
        .mint_tokens(&admin_redemption_ata, supply_amount)
        .await;

    supply_pool(&test_f, admin, USDC_MINT, USDT_MINT, supply_amount).await?;

    let admin_redemption_account: TokenAccount =
        test_f.load_and_deserialize(&admin_redemption_ata).await;
    assert_eq!(
        admin_redemption_account.amount, 0,
        "Admin's redemption token balance should be 0 after supply"
    );

    let pool_address = find_pool(&USDC_MINT, &USDT_MINT);
    let pool_redemption_token_account = find_pool_redemption_token_account(&pool_address);
    let pool_redemption_account: TokenAccount = test_f
        .load_and_deserialize(&pool_redemption_token_account)
        .await;
    assert_eq!(
        pool_redemption_account.amount, supply_amount,
        "Pool should have received the supplied tokens"
    );

    let pool: Pool = test_f.load_and_deserialize(&pool_address).await;
    assert_eq!(
        u128::from_le_bytes(pool.total_supplied),
        u128::from(supply_amount),
        "Pool total supplied should be updated"
    );

    let admin_redemption_account: TokenAccount =
        test_f.load_and_deserialize(&admin_redemption_ata).await;
    assert_eq!(
        admin_redemption_account.amount, 0,
        "Admin's redemption token balance should be 0"
    );

    Ok(())
}

#[tokio::test]
async fn redeem_success() -> anyhow::Result<()> {
    let test_f = TestFixture::new().await;
    let _test_context = setup_full_test_context(&test_f, USDC_MINT, USDT_MINT).await?;

    create_active_pool(&test_f, USDC_MINT, USDT_MINT).await?;

    let pool_address = find_pool(&USDC_MINT, &USDT_MINT);
    let pool_redemption_token_account = find_pool_redemption_token_account(&pool_address);
    let supply_amount = 10000 * 10_u64.pow(USDC_DECIMALS.into());
    test_f
        .mint_tokens(&pool_redemption_token_account, supply_amount)
        .await;

    let user = Keypair::new();
    test_f.fund_account(&user.pubkey()).await;

    let user_redemption_ata =
        get_associated_token_address_with_program_id(&user.pubkey(), &USDC_MINT, &spl_token::ID);
    let user_settlement_ata =
        get_associated_token_address_with_program_id(&user.pubkey(), &USDT_MINT, &spl_token::ID);

    create_associated_token_account(&test_f, &user.pubkey(), &USDC_MINT).await?;
    create_associated_token_account(&test_f, &user.pubkey(), &USDT_MINT).await?;

    let redeem_amount = 1000 * 10_u64.pow(USDT_DECIMALS.into());
    test_f
        .mint_tokens(&user_settlement_ata, redeem_amount)
        .await;

    redeem_from_pool(&test_f, &user, USDC_MINT, USDT_MINT, redeem_amount).await?;

    let user_settlement_account: TokenAccount =
        test_f.load_and_deserialize(&user_settlement_ata).await;
    assert_eq!(
        user_settlement_account.amount, 0,
        "User's settlement token balance should be 0 after redeem"
    );

    let expected_redemption = redeem_amount;
    let user_redemption_account: TokenAccount =
        test_f.load_and_deserialize(&user_redemption_ata).await;
    assert_eq!(
        user_redemption_account.amount, expected_redemption,
        "User should receive redemption tokens"
    );

    let user_settlement_account: TokenAccount =
        test_f.load_and_deserialize(&user_settlement_ata).await;
    assert_eq!(
        user_settlement_account.amount, 0,
        "User's settlement token balance should be 0"
    );

    let pool: Pool = test_f.load_and_deserialize(&pool_address).await;
    assert_eq!(
        u128::from_le_bytes(pool.total_redeemed),
        u128::from(redeem_amount),
        "Pool total redeemed should be updated"
    );

    let pool_redemption_token_account = find_pool_redemption_token_account(&pool_address);
    let pool_redemption_account: TokenAccount = test_f
        .load_and_deserialize(&pool_redemption_token_account)
        .await;
    assert_eq!(
        pool_redemption_account.amount,
        supply_amount - expected_redemption,
        "Pool redemption token balance should decrease"
    );

    let pool_settlement_token_account = find_pool_settlement_token_account(&pool_address);
    let pool_settlement_account: TokenAccount = test_f
        .load_and_deserialize(&pool_settlement_token_account)
        .await;
    assert_eq!(
        pool_settlement_account.amount, redeem_amount,
        "Pool settlement token balance should increase"
    );

    Ok(())
}

#[tokio::test]
async fn withdraw_success() -> anyhow::Result<()> {
    let test_f = TestFixture::new().await;
    let _test_context = setup_full_test_context(&test_f, USDC_MINT, USDT_MINT).await?;

    create_active_pool(&test_f, USDC_MINT, USDT_MINT).await?;

    let admin = &test_f.deployer;
    let admin_settlement_ata =
        get_associated_token_address_with_program_id(&admin.pubkey(), &USDT_MINT, &spl_token::ID);

    create_associated_token_account(&test_f, &admin.pubkey(), &USDT_MINT).await?;

    let pool_address = find_pool(&USDC_MINT, &USDT_MINT);
    let pool_settlement_token_account = find_pool_settlement_token_account(&pool_address);

    let withdraw_amount = 10000 * 10_u64.pow(USDT_DECIMALS.into());
    test_f
        .mint_tokens(&pool_settlement_token_account, withdraw_amount)
        .await;

    withdraw_from_pool(&test_f, admin, USDC_MINT, USDT_MINT, withdraw_amount).await?;

    let pool_settlement_account: TokenAccount = test_f
        .load_and_deserialize(&pool_settlement_token_account)
        .await;
    assert_eq!(
        pool_settlement_account.amount, 0,
        "Pool settlement token balance should be 0"
    );

    let admin_settlement_account: TokenAccount =
        test_f.load_and_deserialize(&admin_settlement_ata).await;
    assert_eq!(
        admin_settlement_account.amount, withdraw_amount,
        "Admin should receive withdrawn settlement tokens"
    );

    Ok(())
}

#[tokio::test]
async fn redeem_with_different_decimals() -> anyhow::Result<()> {
    let test_f = TestFixture::new().await;

    let _test_context = setup_full_test_context(&test_f, USDC_MINT, MSOL_MINT).await?;

    create_active_pool(&test_f, USDC_MINT, MSOL_MINT).await?;

    let pool_address = find_pool(&USDC_MINT, &MSOL_MINT);
    let pool_redemption_token_account = find_pool_redemption_token_account(&pool_address);
    let supply_amount = 10000 * 10_u64.pow(USDC_DECIMALS.into());
    test_f
        .mint_tokens(&pool_redemption_token_account, supply_amount)
        .await;

    let user = Keypair::new();
    test_f.fund_account(&user.pubkey()).await;

    let user_redemption_ata =
        get_associated_token_address_with_program_id(&user.pubkey(), &USDC_MINT, &spl_token::ID);
    let user_settlement_ata =
        get_associated_token_address_with_program_id(&user.pubkey(), &MSOL_MINT, &spl_token::ID);

    create_associated_token_account(&test_f, &user.pubkey(), &USDC_MINT).await?;
    create_associated_token_account(&test_f, &user.pubkey(), &MSOL_MINT).await?;

    let redeem_amount = 1000 * 10_u64.pow(MSOL_DECIMALS.into());
    test_f
        .mint_tokens(&user_settlement_ata, redeem_amount)
        .await;

    redeem_from_pool(&test_f, &user, USDC_MINT, MSOL_MINT, redeem_amount).await?;

    let user_redemption_account: TokenAccount =
        test_f.load_and_deserialize(&user_redemption_ata).await;

    assert_eq!(
        redeem_amount as u128 * 10_u128.pow(USDC_DECIMALS.into())
            / 10_u128.pow(MSOL_DECIMALS.into()),
        user_redemption_account.amount as u128,
        "User should receive redemption tokens"
    );

    Ok(())
}

#[tokio::test]
async fn redeem_with_different_decimals_2() -> anyhow::Result<()> {
    let test_f = TestFixture::new().await;

    let _test_context = setup_full_test_context(&test_f, MSOL_MINT, USDC_MINT).await?;

    create_active_pool(&test_f, MSOL_MINT, USDC_MINT).await?;

    let pool_address = find_pool(&MSOL_MINT, &USDC_MINT);
    let pool_redemption_token_account = find_pool_redemption_token_account(&pool_address);
    let supply_amount = 10000 * 10_u64.pow(MSOL_DECIMALS.into());
    test_f
        .mint_tokens(&pool_redemption_token_account, supply_amount)
        .await;

    let user = Keypair::new();
    test_f.fund_account(&user.pubkey()).await;

    let user_redemption_ata =
        get_associated_token_address_with_program_id(&user.pubkey(), &MSOL_MINT, &spl_token::ID);
    let user_settlement_ata =
        get_associated_token_address_with_program_id(&user.pubkey(), &USDC_MINT, &spl_token::ID);

    create_associated_token_account(&test_f, &user.pubkey(), &MSOL_MINT).await?;
    create_associated_token_account(&test_f, &user.pubkey(), &USDC_MINT).await?;

    let redeem_amount = 1000 * 10_u64.pow(USDC_DECIMALS.into());
    test_f
        .mint_tokens(&user_settlement_ata, redeem_amount)
        .await;

    redeem_from_pool(&test_f, &user, MSOL_MINT, USDC_MINT, redeem_amount).await?;

    let user_redemption_account: TokenAccount =
        test_f.load_and_deserialize(&user_redemption_ata).await;

    assert_eq!(
        redeem_amount as u128 * 10_u128.pow(MSOL_DECIMALS.into())
            / 10_u128.pow(USDC_DECIMALS.into()),
        user_redemption_account.amount as u128,
        "User should receive redemption tokens"
    );

    Ok(())
}
