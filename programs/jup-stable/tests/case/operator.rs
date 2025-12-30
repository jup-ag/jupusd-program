use fixtures::test::TestFixture;
use jup_stable::{
    instructions::OperatorManagementAction,
    state::operator::{Operator, OperatorRole, OperatorStatus},
};
use solana_program_test::*;
use solana_sdk::{signature::Keypair, signer::Signer, transaction::Transaction};

use crate::common::{
    derivation::find_operator,
    faciliter::setup_full_test_context,
    instructions::{
        create_create_operator_instruction, create_delete_operator_instruction,
        create_manage_operator_instruction, CreateOperatorInstructionAccounts,
        DeleteOperatorInstructionAccounts, ManageOperatorInstructionAccounts,
    },
};

#[tokio::test]
async fn create_operator_success() -> anyhow::Result<()> {
    let test_f = TestFixture::new().await;
    let _test_context = setup_full_test_context(&test_f).await?;

    let deployer = test_f.deployer.pubkey();
    let operator_authority = Keypair::new();
    let accounts = CreateOperatorInstructionAccounts {
        operator_authority: deployer,
        payer: deployer,
        new_operator_authority: operator_authority.pubkey(),
    };

    let role = OperatorRole::Admin;
    {
        let mut ctx = test_f.context.borrow_mut();
        let last_blockhash = ctx.get_new_latest_blockhash().await?;
        let tx = Transaction::new_signed_with_payer(
            &[create_create_operator_instruction(accounts, role)],
            Some(&deployer),
            &[&test_f.deployer],
            last_blockhash,
        );

        ctx.banks_client.process_transaction(tx).await?;
    }

    let operator_account: Operator = test_f
        .load_and_deserialize(&find_operator(&operator_authority.pubkey()))
        .await;

    assert_eq!(
        operator_account.operator_authority,
        operator_authority.pubkey(),
        "Operator authority should match"
    );
    assert_eq!(
        operator_account.status,
        OperatorStatus::Enabled,
        "Operator should be enabled by default"
    );
    assert!(
        operator_account.is(role).is_ok(),
        "Operator should have the correct role"
    );

    Ok(())
}

#[tokio::test]
async fn manage_operator_set_status_success() -> anyhow::Result<()> {
    let test_f = TestFixture::new().await;
    let _test_context = setup_full_test_context(&test_f).await?;

    let deployer = test_f.deployer.pubkey();
    let operator_authority = Keypair::new();

    {
        let accounts = CreateOperatorInstructionAccounts {
            operator_authority: deployer,
            payer: deployer,
            new_operator_authority: operator_authority.pubkey(),
        };

        let mut ctx = test_f.context.borrow_mut();
        let last_blockhash = ctx.get_new_latest_blockhash().await?;
        let tx = Transaction::new_signed_with_payer(
            &[create_create_operator_instruction(
                accounts,
                OperatorRole::Admin,
            )],
            Some(&deployer),
            &[&test_f.deployer],
            last_blockhash,
        );

        ctx.banks_client.process_transaction(tx).await?;
    }

    {
        let accounts = ManageOperatorInstructionAccounts {
            operator_authority: deployer,
            managed_operator: find_operator(&operator_authority.pubkey()),
        };

        let action = jup_stable::instructions::OperatorManagementAction::SetStatus {
            status: OperatorStatus::Disabled,
        };

        let mut ctx = test_f.context.borrow_mut();
        let last_blockhash = ctx.get_new_latest_blockhash().await?;
        let tx = Transaction::new_signed_with_payer(
            &[create_manage_operator_instruction(accounts, action)],
            Some(&deployer),
            &[&test_f.deployer],
            last_blockhash,
        );

        ctx.banks_client.process_transaction(tx).await?;
    }

    let operator_account: Operator = test_f
        .load_and_deserialize(&find_operator(&operator_authority.pubkey()))
        .await;
    assert_eq!(
        operator_account.status,
        OperatorStatus::Disabled,
        "Operator should be disabled"
    );

    Ok(())
}

#[tokio::test]
async fn manage_operator_set_role_success() -> anyhow::Result<()> {
    let test_f = TestFixture::new().await;
    let _test_context = setup_full_test_context(&test_f).await?;

    let deployer = test_f.deployer.pubkey();
    let operator_authority = Keypair::new();

    {
        let accounts = CreateOperatorInstructionAccounts {
            operator_authority: deployer,
            payer: deployer,
            new_operator_authority: operator_authority.pubkey(),
        };

        let mut ctx = test_f.context.borrow_mut();
        let last_blockhash = ctx.get_new_latest_blockhash().await?;
        let tx = Transaction::new_signed_with_payer(
            &[create_create_operator_instruction(
                accounts,
                OperatorRole::VaultManager,
            )],
            Some(&deployer),
            &[&test_f.deployer],
            last_blockhash,
        );

        ctx.banks_client.process_transaction(tx).await?;
    }

    let operator_account: Operator = test_f
        .load_and_deserialize(&find_operator(&operator_authority.pubkey()))
        .await;

    assert!(
        operator_account.is(OperatorRole::VaultManager).is_ok(),
        "Should have VaultManager role"
    );
    assert!(
        operator_account.is(OperatorRole::PegManager).is_err(),
        "Should not have PegManager role"
    );

    {
        let accounts = ManageOperatorInstructionAccounts {
            operator_authority: deployer,
            managed_operator: find_operator(&operator_authority.pubkey()),
        };

        let action = jup_stable::instructions::OperatorManagementAction::SetRole {
            role: OperatorRole::PegManager,
        };

        let mut ctx = test_f.context.borrow_mut();
        let last_blockhash = ctx.get_new_latest_blockhash().await?;
        let tx = Transaction::new_signed_with_payer(
            &[create_manage_operator_instruction(accounts, action)],
            Some(&deployer),
            &[&test_f.deployer],
            last_blockhash,
        );

        ctx.banks_client.process_transaction(tx).await?;
    }

    let operator_account: Operator = test_f
        .load_and_deserialize(&find_operator(&operator_authority.pubkey()))
        .await;
    assert!(
        operator_account.is(OperatorRole::VaultManager).is_ok(),
        "Should have VaultManager role"
    );
    assert!(
        operator_account.is(OperatorRole::PegManager).is_ok(),
        "Should have PegManager role"
    );
    Ok(())
}

#[tokio::test]
async fn create_operator_fails_when_not_admin() -> anyhow::Result<()> {
    let test_f = TestFixture::new().await;
    let _test_context = setup_full_test_context(&test_f).await?;

    let deployer = test_f.deployer.pubkey();
    let operator_authority = Keypair::new();

    {
        let accounts = CreateOperatorInstructionAccounts {
            operator_authority: deployer,
            payer: deployer,
            new_operator_authority: operator_authority.pubkey(),
        };

        let mut ctx = test_f.context.borrow_mut();
        let last_blockhash = ctx.get_new_latest_blockhash().await?;
        let tx = Transaction::new_signed_with_payer(
            &[create_create_operator_instruction(
                accounts,
                OperatorRole::VaultManager,
            )],
            Some(&deployer),
            &[&test_f.deployer],
            last_blockhash,
        );

        ctx.banks_client.process_transaction(tx).await?;
    }

    {
        let new_operator_authority = Keypair::new();
        let accounts = CreateOperatorInstructionAccounts {
            operator_authority: operator_authority.pubkey(),
            payer: operator_authority.pubkey(),
            new_operator_authority: new_operator_authority.pubkey(),
        };
        let mut ctx = test_f.context.borrow_mut();
        let last_blockhash = ctx.get_new_latest_blockhash().await?;
        let tx = Transaction::new_signed_with_payer(
            &[create_create_operator_instruction(
                accounts,
                OperatorRole::VaultManager,
            )],
            Some(&operator_authority.pubkey()),
            &[&operator_authority],
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

#[tokio::test]
async fn manage_operator_fails_when_not_admin() -> anyhow::Result<()> {
    let test_f = TestFixture::new().await;
    let _test_context = setup_full_test_context(&test_f).await?;

    let deployer = test_f.deployer.pubkey();
    let operator_authority = Keypair::new();
    test_f.fund_account(&operator_authority.pubkey()).await;

    {
        let accounts = CreateOperatorInstructionAccounts {
            operator_authority: deployer,
            payer: deployer,
            new_operator_authority: operator_authority.pubkey(),
        };

        let mut ctx = test_f.context.borrow_mut();
        let last_blockhash = ctx.get_new_latest_blockhash().await?;
        let tx = Transaction::new_signed_with_payer(
            &[create_create_operator_instruction(
                accounts,
                OperatorRole::VaultManager,
            )],
            Some(&deployer),
            &[&test_f.deployer],
            last_blockhash,
        );

        ctx.banks_client.process_transaction(tx).await?;
    }

    {
        let accounts: ManageOperatorInstructionAccounts = ManageOperatorInstructionAccounts {
            operator_authority: operator_authority.pubkey(),
            managed_operator: find_operator(&operator_authority.pubkey()),
        };

        let action = OperatorManagementAction::SetRole {
            role: OperatorRole::Admin,
        };

        let mut ctx = test_f.context.borrow_mut();
        let last_blockhash = ctx.get_new_latest_blockhash().await?;
        let tx = Transaction::new_signed_with_payer(
            &[create_manage_operator_instruction(accounts, action)],
            Some(&operator_authority.pubkey()),
            &[&operator_authority],
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

#[tokio::test]
async fn delete_operator_success() -> anyhow::Result<()> {
    let test_f = TestFixture::new().await;
    let _test_context = setup_full_test_context(&test_f).await?;

    let deployer = test_f.deployer.pubkey();
    let operator_authority = Keypair::new();

    {
        let accounts = CreateOperatorInstructionAccounts {
            operator_authority: deployer,
            payer: deployer,
            new_operator_authority: operator_authority.pubkey(),
        };

        let role = OperatorRole::Admin;

        let mut ctx = test_f.context.borrow_mut();
        let last_blockhash = ctx.get_new_latest_blockhash().await?;
        let tx = Transaction::new_signed_with_payer(
            &[create_create_operator_instruction(accounts, role)],
            Some(&deployer),
            &[&test_f.deployer],
            last_blockhash,
        );

        ctx.banks_client.process_transaction(tx).await?;
    }

    {
        let accounts = DeleteOperatorInstructionAccounts {
            operator_authority: deployer,
            payer: deployer,
            deleted_operator: find_operator(&operator_authority.pubkey()),
        };

        let mut ctx = test_f.context.borrow_mut();
        let last_blockhash = ctx.get_new_latest_blockhash().await?;
        let tx = Transaction::new_signed_with_payer(
            &[create_delete_operator_instruction(accounts)],
            Some(&deployer),
            &[&test_f.deployer],
            last_blockhash,
        );
        ctx.banks_client.process_transaction(tx).await?;
    }

    let ctx = test_f.context.borrow_mut();
    let account = ctx
        .banks_client
        .get_account(find_operator(&operator_authority.pubkey()))
        .await?;
    assert!(account.is_none(), "Operator account should be deleted");

    Ok(())
}
