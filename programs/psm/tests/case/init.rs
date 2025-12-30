use fixtures::test::TestFixture;
use psm::state::config::Config;
use solana_program_test::*;
use solana_sdk::{
    bpf_loader_upgradeable::get_program_data_address, signer::Signer, transaction::Transaction,
};

use crate::common::{
    derivation::{find_authority, find_config},
    instructions::{create_init_instruction, InitInstructionAccounts},
};

#[tokio::test]
async fn init_success() -> anyhow::Result<()> {
    let test_f = TestFixture::new().await;

    let payer = test_f.deployer.pubkey();
    let program_data = get_program_data_address(&psm::ID);

    let accounts = InitInstructionAccounts {
        payer,
        upgrade_authority: test_f.deployer.pubkey(),
        program_data,
    };

    {
        let mut ctx = test_f.context.borrow_mut();
        let last_blockhash = ctx.get_new_latest_blockhash().await?;
        let tx = Transaction::new_signed_with_payer(
            &[create_init_instruction(accounts)],
            Some(&payer),
            &[&test_f.deployer],
            last_blockhash,
        );

        ctx.banks_client.process_transaction(tx).await?;
    }

    let config_account: Config = test_f.load_and_deserialize(&find_config()).await;

    assert_eq!(
        config_account.admins[0],
        test_f.deployer.pubkey(),
        "First admin should be the upgrade authority"
    );

    assert_eq!(
        config_account.authority,
        find_authority(),
        "Config should have the correct authority"
    );

    assert!(
        config_account.authority_bump != 0,
        "Config should have non null authority bump"
    );

    assert!(
        config_account.config_bump != 0,
        "Config should have non null config bump"
    );

    assert_eq!(config_account.is_paused, 0, "Protocol should not be paused");

    Ok(())
}
