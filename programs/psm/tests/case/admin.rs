use fixtures::test::TestFixture;
use psm::state::config::Config;
use solana_program_test::*;
use solana_sdk::{signature::Keypair, signer::Signer, transaction::Transaction};

use crate::common::{
    derivation::find_config,
    faciliter::init_program,
    instructions::{
        create_add_admin_instruction, create_remove_admin_instruction,
        create_update_pause_flag_instruction,
    },
};

#[tokio::test]
async fn add_admin_success() -> anyhow::Result<()> {
    let test_f = TestFixture::new().await;
    init_program(&test_f).await?;

    let new_admin = Keypair::new();
    let payer = test_f.deployer.pubkey();

    {
        let mut ctx = test_f.context.borrow_mut();
        let last_blockhash = ctx.get_new_latest_blockhash().await?;
        let tx = Transaction::new_signed_with_payer(
            &[create_add_admin_instruction(payer, new_admin.pubkey())],
            Some(&payer),
            &[&test_f.deployer],
            last_blockhash,
        );

        ctx.banks_client.process_transaction(tx).await?;
    }

    let config: Config = test_f.load_and_deserialize(&find_config()).await;
    assert!(
        config.is_admin(&new_admin.pubkey()),
        "New admin should be added to config"
    );

    Ok(())
}

#[tokio::test]
async fn remove_admin_success() -> anyhow::Result<()> {
    let test_f = TestFixture::new().await;
    init_program(&test_f).await?;

    let new_admin = Keypair::new();
    let payer = test_f.deployer.pubkey();

    // First add the admin
    {
        let mut ctx = test_f.context.borrow_mut();
        let last_blockhash = ctx.get_new_latest_blockhash().await?;
        let tx = Transaction::new_signed_with_payer(
            &[create_add_admin_instruction(payer, new_admin.pubkey())],
            Some(&payer),
            &[&test_f.deployer],
            last_blockhash,
        );

        ctx.banks_client.process_transaction(tx).await?;
    }

    // Then remove the admin
    {
        let mut ctx = test_f.context.borrow_mut();
        let last_blockhash = ctx.get_new_latest_blockhash().await?;
        let tx = Transaction::new_signed_with_payer(
            &[create_remove_admin_instruction(payer, new_admin.pubkey())],
            Some(&payer),
            &[&test_f.deployer],
            last_blockhash,
        );

        ctx.banks_client.process_transaction(tx).await?;
    }

    let config: Config = test_f.load_and_deserialize(&find_config()).await;
    assert!(
        !config.is_admin(&new_admin.pubkey()),
        "Admin should be removed from config"
    );

    Ok(())
}

#[tokio::test]
async fn update_pause_flag_success() -> anyhow::Result<()> {
    let test_f = TestFixture::new().await;
    init_program(&test_f).await?;

    let payer = test_f.deployer.pubkey();

    // Pause the protocol
    {
        let mut ctx = test_f.context.borrow_mut();
        let last_blockhash = ctx.get_new_latest_blockhash().await?;
        let tx = Transaction::new_signed_with_payer(
            &[create_update_pause_flag_instruction(payer, true)],
            Some(&payer),
            &[&test_f.deployer],
            last_blockhash,
        );

        ctx.banks_client.process_transaction(tx).await?;
    }

    let config: Config = test_f.load_and_deserialize(&find_config()).await;
    assert!(config.is_paused(), "Protocol should be paused");

    // Unpause the protocol
    {
        let mut ctx = test_f.context.borrow_mut();
        let last_blockhash = ctx.get_new_latest_blockhash().await?;
        let tx = Transaction::new_signed_with_payer(
            &[create_update_pause_flag_instruction(payer, false)],
            Some(&payer),
            &[&test_f.deployer],
            last_blockhash,
        );

        ctx.banks_client.process_transaction(tx).await?;
    }

    let config: Config = test_f.load_and_deserialize(&find_config()).await;
    assert!(!config.is_paused(), "Protocol should be unpaused");

    Ok(())
}

#[tokio::test]
async fn add_duplicate_admin_fails() -> anyhow::Result<()> {
    let test_f = TestFixture::new().await;
    init_program(&test_f).await?;

    let new_admin = Keypair::new();
    let payer = test_f.deployer.pubkey();

    // First add the admin
    {
        let mut ctx = test_f.context.borrow_mut();
        let last_blockhash = ctx.get_new_latest_blockhash().await?;
        let tx = Transaction::new_signed_with_payer(
            &[create_add_admin_instruction(payer, new_admin.pubkey())],
            Some(&payer),
            &[&test_f.deployer],
            last_blockhash,
        );

        ctx.banks_client.process_transaction(tx).await?;
    }

    // Try to add the same admin again
    let result = {
        let mut ctx = test_f.context.borrow_mut();
        let last_blockhash = ctx.get_new_latest_blockhash().await?;
        let tx = Transaction::new_signed_with_payer(
            &[create_add_admin_instruction(payer, new_admin.pubkey())],
            Some(&payer),
            &[&test_f.deployer],
            last_blockhash,
        );

        ctx.banks_client.process_transaction(tx).await
    };

    assert!(result.is_err(), "Adding duplicate admin should fail");

    Ok(())
}
