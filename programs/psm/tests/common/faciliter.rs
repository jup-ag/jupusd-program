use anyhow::Result;
use fixtures::test::TestFixture;
use psm::state::pool::PoolStatus;
use solana_sdk::{
    bpf_loader_upgradeable::get_program_data_address, pubkey::Pubkey, signature::Keypair,
    signer::Signer, transaction::Transaction,
};

use crate::common::instructions::{
    create_add_admin_instruction, create_create_pool_instruction, create_init_instruction,
    create_redeem_instruction, create_set_pool_status_instruction, create_supply_instruction,
    create_withdraw_instruction, CreatePoolInstructionAccounts, InitInstructionAccounts,
    RedeemInstructionAccounts, SupplyInstructionAccounts, WithdrawInstructionAccounts,
};

pub async fn init_program(test_f: &TestFixture) -> Result<()> {
    let payer = test_f.deployer.pubkey();
    let program_data = get_program_data_address(&psm::ID);

    let accounts = InitInstructionAccounts {
        payer,
        upgrade_authority: test_f.deployer.pubkey(),
        program_data,
    };

    let mut ctx = test_f.context.borrow_mut();
    let last_blockhash = ctx.get_new_latest_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(
        &[create_init_instruction(accounts)],
        Some(&payer),
        &[&test_f.deployer],
        last_blockhash,
    );

    ctx.banks_client.process_transaction(tx).await?;

    Ok(())
}

pub async fn create_pool(
    test_f: &TestFixture,
    redemption_mint: Pubkey,
    settlement_mint: Pubkey,
) -> Result<()> {
    let payer = test_f.deployer.pubkey();

    let accounts = CreatePoolInstructionAccounts {
        admin: payer,
        payer,
        redemption_mint,
        settlement_mint,
        redemption_token_program: spl_token::ID,
        settlement_token_program: spl_token::ID,
    };

    let mut ctx = test_f.context.borrow_mut();
    let last_blockhash = ctx.get_new_latest_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(
        &[create_create_pool_instruction(accounts)],
        Some(&payer),
        &[&test_f.deployer],
        last_blockhash,
    );

    ctx.banks_client.process_transaction(tx).await?;

    Ok(())
}

pub async fn create_active_pool(
    test_f: &TestFixture,
    redemption_mint: Pubkey,
    settlement_mint: Pubkey,
) -> Result<()> {
    let payer = test_f.deployer.pubkey();

    let accounts = CreatePoolInstructionAccounts {
        admin: payer,
        payer,
        redemption_mint,
        settlement_mint,
        redemption_token_program: spl_token::ID,
        settlement_token_program: spl_token::ID,
    };

    let mut ctx = test_f.context.borrow_mut();
    let last_blockhash = ctx.get_new_latest_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(
        &[
            create_create_pool_instruction(accounts),
            create_set_pool_status_instruction(
                payer,
                redemption_mint,
                settlement_mint,
                PoolStatus::Active,
            ),
        ],
        Some(&payer),
        &[&test_f.deployer],
        last_blockhash,
    );

    ctx.banks_client.process_transaction(tx).await?;

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

pub async fn supply_pool(
    test_f: &TestFixture,
    admin: &Keypair,
    redemption_mint: Pubkey,
    settlement_mint: Pubkey,
    amount: u64,
) -> Result<()> {
    let accounts = SupplyInstructionAccounts {
        admin: admin.pubkey(),
        redemption_mint,
        settlement_mint,
        redemption_token_program: spl_token::ID,
    };

    let mut ctx = test_f.context.borrow_mut();
    let last_blockhash = ctx.get_new_latest_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(
        &[create_supply_instruction(accounts, amount)],
        Some(&admin.pubkey()),
        &[admin],
        last_blockhash,
    );

    ctx.banks_client.process_transaction(tx).await?;

    Ok(())
}

pub async fn redeem_from_pool(
    test_f: &TestFixture,
    user: &Keypair,
    redemption_mint: Pubkey,
    settlement_mint: Pubkey,
    amount: u64,
) -> Result<()> {
    let accounts = RedeemInstructionAccounts {
        user: user.pubkey(),
        redemption_mint,
        settlement_mint,
        redemption_token_program: spl_token::ID,
        settlement_token_program: spl_token::ID,
    };

    let mut ctx = test_f.context.borrow_mut();
    let last_blockhash = ctx.get_new_latest_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(
        &[create_redeem_instruction(accounts, amount)],
        Some(&user.pubkey()),
        &[user],
        last_blockhash,
    );

    ctx.banks_client.process_transaction(tx).await?;

    Ok(())
}

pub async fn withdraw_from_pool(
    test_f: &TestFixture,
    admin: &Keypair,
    redemption_mint: Pubkey,
    settlement_mint: Pubkey,
    amount: u64,
) -> Result<()> {
    let accounts = WithdrawInstructionAccounts {
        admin: admin.pubkey(),
        redemption_mint,
        settlement_mint,
        settlement_token_program: spl_token::ID,
    };

    let mut ctx = test_f.context.borrow_mut();
    let last_blockhash = ctx.get_new_latest_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(
        &[create_withdraw_instruction(accounts, amount)],
        Some(&admin.pubkey()),
        &[admin],
        last_blockhash,
    );

    ctx.banks_client.process_transaction(tx).await?;

    Ok(())
}

#[allow(dead_code)]
pub async fn add_admin(test_f: &TestFixture, new_admin: Pubkey) -> Result<()> {
    let payer = test_f.deployer.pubkey();

    let mut ctx = test_f.context.borrow_mut();
    let last_blockhash = ctx.get_new_latest_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(
        &[create_add_admin_instruction(payer, new_admin)],
        Some(&payer),
        &[&test_f.deployer],
        last_blockhash,
    );

    ctx.banks_client.process_transaction(tx).await?;

    Ok(())
}

pub struct TestContext {
    #[allow(dead_code)]
    pub redemption_mint: Pubkey,
    #[allow(dead_code)]
    pub settlement_mint: Pubkey,
}

pub async fn setup_full_test_context(
    test_f: &TestFixture,
    redemption_mint: Pubkey,
    settlement_mint: Pubkey,
) -> Result<TestContext> {
    test_f
        .replicate_account_from_mainnet(&redemption_mint)
        .await?;
    test_f
        .replicate_account_from_mainnet(&settlement_mint)
        .await?;
    init_program(test_f).await?;

    Ok(TestContext {
        redemption_mint,
        settlement_mint,
    })
}
