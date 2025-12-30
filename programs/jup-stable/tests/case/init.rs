use fixtures::test::TestFixture;
use jup_stable::state::{
    config::Config,
    operator::{Operator, OperatorRole, OperatorStatus},
};
use solana_program_test::*;
use solana_sdk::{
    bpf_loader_upgradeable::get_program_data_address, signature::Keypair, signer::Signer,
    transaction::Transaction,
};

use crate::common::{
    constants::{JUPUSD_DECIMALS, JUPUSD_NAME, JUPUSD_SYMBOL, JUPUSD_URI},
    derivation::{find_authority, find_config, find_operator},
    instructions::{create_init_instruction, InitInstructionAccounts, InitInstructionArgs},
};

#[tokio::test]
async fn init_success() -> anyhow::Result<()> {
    let test_f = TestFixture::new().await;

    let payer = test_f.deployer.pubkey();
    let program_data = get_program_data_address(&jup_stable::ID);
    let lp_mint = Keypair::new();

    let accounts = InitInstructionAccounts {
        payer,
        upgrade_authority: test_f.deployer.pubkey(),
        program_data,
        mint: lp_mint.pubkey(),
        token_program: spl_token::ID,
    };

    let args = InitInstructionArgs {
        decimals: JUPUSD_DECIMALS,
        name: JUPUSD_NAME.to_string(),
        symbol: JUPUSD_SYMBOL.to_string(),
        uri: JUPUSD_URI.to_string(),
    };

    {
        let mut ctx = test_f.context.borrow_mut();
        let last_blockhash = ctx.get_new_latest_blockhash().await?;
        let tx = Transaction::new_signed_with_payer(
            &[create_init_instruction(accounts, args)],
            Some(&payer),
            &[&test_f.deployer, &lp_mint],
            last_blockhash,
        );

        ctx.banks_client.process_transaction(tx).await?;
    }

    let config_account: Config = test_f.load_and_deserialize(&find_config()).await;

    assert_eq!(
        config_account.mint,
        lp_mint.pubkey(),
        "Config should have the correct mint"
    );

    assert_eq!(
        config_account.decimals, JUPUSD_DECIMALS,
        "Config should have the correct decimals"
    );

    assert_eq!(
        config_account.authority,
        find_authority(),
        "Config should have the correct authority"
    );

    assert_eq!(
        config_account.token_program,
        spl_token::ID,
        "Config should have the correct token program"
    );

    assert!(
        config_account.authority_bump != 0,
        "Config should have non null authority bump"
    );

    assert!(
        config_account.config_bump != 0,
        "Config should have non null config bump"
    );

    let operator_account: Operator = test_f.load_and_deserialize(&find_operator(&payer)).await;

    assert_eq!(
        operator_account.operator_authority, payer,
        "Operator should have the correct authority"
    );

    assert!(
        operator_account.is(OperatorRole::Admin).is_ok(),
        "Operator should be an admin"
    );

    assert_eq!(
        operator_account.status,
        OperatorStatus::Enabled,
        "Operator should be enabled"
    );

    Ok(())
}
