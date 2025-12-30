use anchor_spl::token_interface::TokenAccount;
use fixtures::test::TestFixture;
use jup_stable::state::{benefactor::Benefactor, config::Config, vault::Vault};
use solana_program_test::*;
use solana_sdk::{signature::Keypair, signer::Signer};
use spl_associated_token_account::get_associated_token_address_with_program_id;

use crate::common::{
    constants::{
        JUPUSD_DECIMALS, USDC_DECIMALS, USDC_MINT, USDC_ORACLE_CONFIG, USDC_PRICE_ACCOUNT,
    },
    derivation::{find_config, find_vault, find_vault_token_account},
    faciliter::{
        create_active_benefactor, create_associated_token_account, create_vault_with_oracle,
        mint_stablecoin, redeem_stablecoin, refresh_pyth_feed, set_period_limit,
        setup_full_test_context, MintRedeemParams, PeriodLimitArgs, PeriodLimitTarget,
    },
};

#[tokio::test]
async fn mint_redeem_success() -> anyhow::Result<()> {
    let test_f = TestFixture::new().await;
    let test_context = setup_full_test_context(&test_f).await?;

    let mint = USDC_MINT;
    let custodian: Keypair = Keypair::new();
    create_vault_with_oracle(&test_f, mint, custodian.pubkey(), USDC_ORACLE_CONFIG).await?;

    let user = Keypair::new();
    test_f.fund_account(&user.pubkey()).await;

    let benefactor_pubkey = create_active_benefactor(&test_f, &user.pubkey(), 0u16, 0u16).await?;

    let max_mint_amount = 1_000_000 * 10_u64.pow(JUPUSD_DECIMALS.into());
    let max_redeem_amount = 1_000_000 * 10_u64.pow(JUPUSD_DECIMALS.into());
    set_period_limit(&test_f, vec![
        PeriodLimitArgs {
            target: PeriodLimitTarget::Benefactor(benefactor_pubkey),
            index: 0,
            duration_seconds: 3600u64,
            max_mint_amount,
            max_redeem_amount,
        },
        PeriodLimitArgs {
            target: PeriodLimitTarget::Vault(mint),
            index: 0,
            duration_seconds: 3600u64,
            max_mint_amount,
            max_redeem_amount,
        },
        PeriodLimitArgs {
            target: PeriodLimitTarget::Config,
            index: 0,
            duration_seconds: 3600u64,
            max_mint_amount,
            max_redeem_amount,
        },
    ])
    .await?;

    let user_collateral_ata =
        get_associated_token_address_with_program_id(&user.pubkey(), &mint, &spl_token::ID);
    let user_lp_ata = get_associated_token_address_with_program_id(
        &user.pubkey(),
        &test_context.lp_mint,
        &spl_token::ID,
    );
    let custodian_collateral_ata =
        get_associated_token_address_with_program_id(&custodian.pubkey(), &mint, &spl_token::ID);

    create_associated_token_account(&test_f, &user.pubkey(), &mint).await?;
    create_associated_token_account(&test_f, &user.pubkey(), &test_context.lp_mint).await?;
    create_associated_token_account(&test_f, &custodian.pubkey(), &mint).await?;

    let amount_in = 600 * 10_u64.pow(USDC_DECIMALS.into());
    let min_amount_out = amount_in * 99 / 100;
    test_f.mint_tokens(&user_collateral_ata, amount_in).await;

    test_f
        .replicate_account_from_mainnet(&USDC_PRICE_ACCOUNT)
        .await?;
    refresh_pyth_feed(&test_f, USDC_PRICE_ACCOUNT).await?;

    let accounts = MintRedeemParams {
        user,
        benefactor: benefactor_pubkey,
        custodian: custodian.pubkey(),
        vault_mint: mint,
        lp_mint: test_context.lp_mint,
        vault_token_program: None,
        lp_token_program: None,
        remaining_accounts: vec![USDC_PRICE_ACCOUNT],
    };

    mint_stablecoin(&test_f, &accounts, amount_in, min_amount_out).await?;

    let user_collateral_account: TokenAccount =
        test_f.load_and_deserialize(&user_collateral_ata).await;
    assert_eq!(
        user_collateral_account.amount, 0,
        "User's vault mint balance should be 0"
    );

    let user_lp_mint_account: TokenAccount = test_f.load_and_deserialize(&user_lp_ata).await;
    assert!(
        user_lp_mint_account.amount >= min_amount_out,
        "User's balance should be greater than or equal to the minimum amount out: {} >= {}",
        user_lp_mint_account.amount,
        min_amount_out
    );

    let config: Config = test_f.load_and_deserialize(&find_config()).await;
    assert_eq!(
        config.period_limits[0].minted_amount, user_lp_mint_account.amount,
        "Config period limit should be updated"
    );

    let vault: Vault = test_f.load_and_deserialize(&find_vault(&mint)).await;
    assert_eq!(
        vault.period_limits[0].minted_amount, user_lp_mint_account.amount,
        "Vault period limit should be updated"
    );
    assert_eq!(
        u128::from_le_bytes(vault.total_minted),
        u128::from(user_lp_mint_account.amount),
        "Vault total minted should be updated"
    );
    assert_eq!(
        u128::from_le_bytes(vault.total_redeemed),
        0,
        "Vault total redeemed should be 0"
    );

    let benefactor: Benefactor = test_f.load_and_deserialize(&benefactor_pubkey).await;
    assert_eq!(
        benefactor.period_limits[0].minted_amount, user_lp_mint_account.amount,
        "Benefactor period limit should be updated"
    );
    assert_eq!(
        u128::from_le_bytes(benefactor.total_minted),
        u128::from(user_lp_mint_account.amount),
        "Benefactor total minted should be updated"
    );
    assert_eq!(
        u128::from_le_bytes(benefactor.total_redeemed),
        0,
        "Benefactor total redeemed should be 0"
    );
    let custodian_token_account: TokenAccount =
        test_f.load_and_deserialize(&custodian_collateral_ata).await;
    assert!(
        custodian_token_account.amount == amount_in,
        "Custodian's balance should be equal to the amount in: {} >= {}",
        custodian_token_account.amount,
        amount_in
    );

    let redeem_amount = user_lp_mint_account.amount;
    let redeem_amount_out = user_lp_mint_account.amount * 99 / 100;
    test_f
        .mint_tokens(&find_vault_token_account(&mint), redeem_amount)
        .await;
    redeem_stablecoin(&test_f, &accounts, redeem_amount, redeem_amount_out).await?;

    let user_collateral_account: TokenAccount =
        test_f.load_and_deserialize(&user_collateral_ata).await;
    assert!(
        user_collateral_account.amount >= redeem_amount_out,
        "User's vault mint balance should be greater than or equal to the redeem amount out: {} \
         >= {}",
        user_collateral_account.amount,
        redeem_amount_out
    );

    let user_lp_mint_account: TokenAccount = test_f.load_and_deserialize(&user_lp_ata).await;
    assert_eq!(user_lp_mint_account.amount, 0, "User's balance should be 0");

    let net_redeem_amount = redeem_amount - benefactor.calculate_redeem_fee(redeem_amount);
    let config: Config = test_f.load_and_deserialize(&find_config()).await;
    assert_eq!(
        config.period_limits[0].redeemed_amount, net_redeem_amount,
        "Config period limit should be updated"
    );

    let vault: Vault = test_f.load_and_deserialize(&find_vault(&mint)).await;
    assert_eq!(
        vault.period_limits[0].redeemed_amount, net_redeem_amount,
        "Vault period limit should be updated"
    );
    assert_eq!(
        u128::from_le_bytes(vault.total_redeemed),
        u128::from(net_redeem_amount),
        "Vault total redeemed should be updated"
    );

    let benefactor: Benefactor = test_f.load_and_deserialize(&benefactor_pubkey).await;
    assert_eq!(
        benefactor.period_limits[0].redeemed_amount, net_redeem_amount,
        "Benefactor period limit should be updated"
    );
    assert_eq!(
        u128::from_le_bytes(benefactor.total_minted),
        u128::from(redeem_amount),
        "Benefactor total minted should be updated"
    );
    assert_eq!(
        u128::from_le_bytes(benefactor.total_redeemed),
        u128::from(net_redeem_amount),
        "Benefactor total redeemed should be updated"
    );

    Ok(())
}

#[tokio::test]
async fn mint_redeem_with_benefactor_fees_success() -> anyhow::Result<()> {
    let test_f = TestFixture::new().await;
    let test_context = setup_full_test_context(&test_f).await?;

    let mint = USDC_MINT;
    let custodian: Keypair = Keypair::new();
    create_vault_with_oracle(&test_f, mint, custodian.pubkey(), USDC_ORACLE_CONFIG).await?;

    let user = Keypair::new();
    test_f.fund_account(&user.pubkey()).await;

    let mint_fee_rate = 100u16; // 1%
    let redeem_fee_rate = 100u16;
    let benefactor_pubkey =
        create_active_benefactor(&test_f, &user.pubkey(), mint_fee_rate, redeem_fee_rate).await?;

    let user_collateral_ata =
        get_associated_token_address_with_program_id(&user.pubkey(), &mint, &spl_token::ID);
    let user_lp_ata = get_associated_token_address_with_program_id(
        &user.pubkey(),
        &test_context.lp_mint,
        &spl_token::ID,
    );
    let custodian_collateral_ata =
        get_associated_token_address_with_program_id(&custodian.pubkey(), &mint, &spl_token::ID);

    create_associated_token_account(&test_f, &user.pubkey(), &mint).await?;
    create_associated_token_account(&test_f, &user.pubkey(), &test_context.lp_mint).await?;
    create_associated_token_account(&test_f, &custodian.pubkey(), &mint).await?;

    let amount_in = 1000 * 10_u64.pow(USDC_DECIMALS.into());
    let min_amount_out = amount_in * 98 / 100; // same decimal

    test_f.mint_tokens(&user_collateral_ata, amount_in).await;
    test_f
        .replicate_account_from_mainnet(&USDC_PRICE_ACCOUNT)
        .await?;
    refresh_pyth_feed(&test_f, USDC_PRICE_ACCOUNT).await?;

    let accounts = MintRedeemParams {
        user,
        benefactor: benefactor_pubkey,
        custodian: custodian.pubkey(),
        vault_mint: mint,
        lp_mint: test_context.lp_mint,
        vault_token_program: None,
        lp_token_program: None,
        remaining_accounts: vec![USDC_PRICE_ACCOUNT],
    };

    mint_stablecoin(&test_f, &accounts, amount_in, min_amount_out).await?;

    let user_collateral_account: TokenAccount =
        test_f.load_and_deserialize(&user_collateral_ata).await;
    assert_eq!(
        user_collateral_account.amount, 0,
        "User's vault mint balance should be 0"
    );
    let custodian_token_account: TokenAccount =
        test_f.load_and_deserialize(&custodian_collateral_ata).await;
    assert!(
        custodian_token_account.amount == amount_in,
        "Custodian's balance should be equal to the amount in: {} >= {}",
        custodian_token_account.amount,
        amount_in
    );

    let user_lp_mint_account: TokenAccount = test_f.load_and_deserialize(&user_lp_ata).await;
    assert!(
        user_lp_mint_account.amount >= min_amount_out,
        "User's LP token balance should be >= min_amount_out: {} >= {}",
        user_lp_mint_account.amount,
        min_amount_out
    );

    let redeem_amount = user_lp_mint_account.amount;
    let redeem_amount_out = user_lp_mint_account.amount * 99 / 100;
    test_f
        .mint_tokens(&find_vault_token_account(&mint), redeem_amount)
        .await;
    redeem_stablecoin(&test_f, &accounts, redeem_amount, redeem_amount_out).await?;

    Ok(())
}

#[tokio::test]
async fn mint_outside_of_period_limit_fail() -> anyhow::Result<()> {
    let test_f = TestFixture::new().await;
    let test_context = setup_full_test_context(&test_f).await?;

    let mint = USDC_MINT;
    let custodian: Keypair = Keypair::new();
    create_vault_with_oracle(&test_f, mint, custodian.pubkey(), USDC_ORACLE_CONFIG).await?;

    let user = Keypair::new();
    test_f.fund_account(&user.pubkey()).await;

    let benefactor_pubkey = create_active_benefactor(&test_f, &user.pubkey(), 0u16, 0u16).await?;

    let max_mint_amount = 100_000 * 10_u64.pow(JUPUSD_DECIMALS.into());
    let max_redeem_amount = 100_000 * 10_u64.pow(JUPUSD_DECIMALS.into());
    set_period_limit(&test_f, vec![PeriodLimitArgs {
        target: PeriodLimitTarget::Benefactor(benefactor_pubkey),
        index: 0,
        duration_seconds: 3600u64,
        max_mint_amount,
        max_redeem_amount,
    }])
    .await?;

    let user_collateral_ata =
        get_associated_token_address_with_program_id(&user.pubkey(), &mint, &spl_token::ID);

    create_associated_token_account(&test_f, &user.pubkey(), &mint).await?;
    create_associated_token_account(&test_f, &user.pubkey(), &test_context.lp_mint).await?;
    create_associated_token_account(&test_f, &custodian.pubkey(), &mint).await?;

    let mint_amount = 110_000 * 10_u64.pow(USDC_DECIMALS.into());
    test_f.mint_tokens(&user_collateral_ata, mint_amount).await;

    test_f
        .replicate_account_from_mainnet(&USDC_PRICE_ACCOUNT)
        .await?;
    refresh_pyth_feed(&test_f, USDC_PRICE_ACCOUNT).await?;

    let accounts = MintRedeemParams {
        user,
        benefactor: benefactor_pubkey,
        custodian: custodian.pubkey(),
        vault_mint: mint,
        lp_mint: test_context.lp_mint,
        vault_token_program: None,
        lp_token_program: None,
        remaining_accounts: vec![USDC_PRICE_ACCOUNT],
    };

    let result = mint_stablecoin(&test_f, &accounts, mint_amount, 0).await;

    assert!(
        result.is_err(),
        "Transaction should fail when minting outside of period limit"
    );

    set_period_limit(&test_f, vec![
        PeriodLimitArgs {
            target: PeriodLimitTarget::Benefactor(benefactor_pubkey),
            index: 0,
            duration_seconds: 3600u64,
            max_mint_amount: u64::MAX,
            max_redeem_amount: u64::MAX,
        },
        PeriodLimitArgs {
            target: PeriodLimitTarget::Vault(mint),
            index: 0,
            duration_seconds: 3600u64,
            max_mint_amount,
            max_redeem_amount,
        },
    ])
    .await?;

    let result = mint_stablecoin(&test_f, &accounts, mint_amount, 0).await;

    assert!(
        result.is_err(),
        "Transaction should fail when minting outside of period limit"
    );

    set_period_limit(&test_f, vec![
        PeriodLimitArgs {
            target: PeriodLimitTarget::Benefactor(benefactor_pubkey),
            index: 0,
            duration_seconds: 3600u64,
            max_mint_amount: u64::MAX,
            max_redeem_amount: u64::MAX,
        },
        PeriodLimitArgs {
            target: PeriodLimitTarget::Vault(mint),
            index: 0,
            duration_seconds: 3600u64,
            max_mint_amount: u64::MAX,
            max_redeem_amount: u64::MAX,
        },
        PeriodLimitArgs {
            target: PeriodLimitTarget::Vault(mint),
            index: 0,
            duration_seconds: 3600u64,
            max_mint_amount,
            max_redeem_amount,
        },
    ])
    .await?;

    let result = mint_stablecoin(&test_f, &accounts, mint_amount, 0).await;

    assert!(
        result.is_err(),
        "Transaction should fail when minting outside of period limit"
    );

    Ok(())
}

#[tokio::test]
async fn redeem_outside_of_period_limit_fail() -> anyhow::Result<()> {
    let test_f = TestFixture::new().await;
    let test_context = setup_full_test_context(&test_f).await?;

    let mint = USDC_MINT;
    let custodian: Keypair = Keypair::new();
    create_vault_with_oracle(&test_f, mint, custodian.pubkey(), USDC_ORACLE_CONFIG).await?;

    let user = Keypair::new();
    test_f.fund_account(&user.pubkey()).await;

    let benefactor_pubkey = create_active_benefactor(&test_f, &user.pubkey(), 0u16, 0u16).await?;

    let max_mint_amount = 100_000 * 10_u64.pow(JUPUSD_DECIMALS.into());
    let max_redeem_amount = 100_000 * 10_u64.pow(JUPUSD_DECIMALS.into());
    set_period_limit(&test_f, vec![PeriodLimitArgs {
        target: PeriodLimitTarget::Benefactor(benefactor_pubkey),
        index: 0,
        duration_seconds: 3600u64,
        max_mint_amount,
        max_redeem_amount,
    }])
    .await?;

    let user_lp_ata = get_associated_token_address_with_program_id(
        &user.pubkey(),
        &test_context.lp_mint,
        &spl_token::ID,
    );

    create_associated_token_account(&test_f, &user.pubkey(), &mint).await?;
    create_associated_token_account(&test_f, &user.pubkey(), &test_context.lp_mint).await?;
    create_associated_token_account(&test_f, &custodian.pubkey(), &mint).await?;

    let stablecoin_amount = 200_000 * 10_u64.pow(JUPUSD_DECIMALS.into());
    test_f.mint_tokens(&user_lp_ata, stablecoin_amount).await;

    let vault_token_account_pubkey = find_vault_token_account(&mint);
    test_f
        .mint_tokens(
            &vault_token_account_pubkey,
            1_000_000 * 10_u64.pow(USDC_DECIMALS.into()),
        )
        .await;
    test_f
        .replicate_account_from_mainnet(&USDC_PRICE_ACCOUNT)
        .await?;
    refresh_pyth_feed(&test_f, USDC_PRICE_ACCOUNT).await?;

    let accounts = MintRedeemParams {
        user,
        benefactor: benefactor_pubkey,
        custodian: custodian.pubkey(),
        vault_mint: mint,
        lp_mint: test_context.lp_mint,
        vault_token_program: None,
        lp_token_program: None,
        remaining_accounts: vec![USDC_PRICE_ACCOUNT],
    };

    let result = redeem_stablecoin(&test_f, &accounts, stablecoin_amount, 0).await;

    assert!(
        result.is_err(),
        "Transaction should fail when redeeming outside of period limit"
    );

    set_period_limit(&test_f, vec![
        PeriodLimitArgs {
            target: PeriodLimitTarget::Benefactor(benefactor_pubkey),
            index: 0,
            duration_seconds: 3600u64,
            max_mint_amount: u64::MAX,
            max_redeem_amount: u64::MAX,
        },
        PeriodLimitArgs {
            target: PeriodLimitTarget::Vault(mint),
            index: 0,
            duration_seconds: 3600u64,
            max_mint_amount,
            max_redeem_amount,
        },
    ])
    .await?;

    let result = redeem_stablecoin(&test_f, &accounts, stablecoin_amount, 0).await;

    assert!(
        result.is_err(),
        "Transaction should fail when minting outside of period limit"
    );

    set_period_limit(&test_f, vec![
        PeriodLimitArgs {
            target: PeriodLimitTarget::Benefactor(benefactor_pubkey),
            index: 0,
            duration_seconds: 3600u64,
            max_mint_amount: u64::MAX,
            max_redeem_amount: u64::MAX,
        },
        PeriodLimitArgs {
            target: PeriodLimitTarget::Vault(mint),
            index: 0,
            duration_seconds: 3600u64,
            max_mint_amount: u64::MAX,
            max_redeem_amount: u64::MAX,
        },
        PeriodLimitArgs {
            target: PeriodLimitTarget::Vault(mint),
            index: 0,
            duration_seconds: 3600u64,
            max_mint_amount,
            max_redeem_amount,
        },
    ])
    .await?;

    let result = redeem_stablecoin(&test_f, &accounts, stablecoin_amount, 0).await;

    assert!(
        result.is_err(),
        "Transaction should fail when minting outside of period limit"
    );

    Ok(())
}
