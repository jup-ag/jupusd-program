use std::{fs::read, mem};

use anchor_lang::{prelude::UpgradeableLoaderState, system_program, Discriminator, ZeroCopy};
use solana_account::Account;
use solana_program_test::ProgramTest;
use solana_rent::Rent;
#[allow(deprecated)]
use solana_sdk::{
    bpf_loader_upgradeable::{self, get_program_data_address},
    native_token::LAMPORTS_PER_SOL,
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
    transaction::Transaction,
};

use crate::test::TestFixture;

pub const MS_PER_SLOT: u64 = 400;
pub const RUST_LOG_DEFAULT: &str = "solana_rbpf::vm=info,solana_program_runtime::stable_log=debug,\
                                    solana_runtime::message_processor=debug,\
                                    solana_runtime::system_instruction_processor=info,\
                                    solana_program_test=info,solana_bpf_loader_program=debug";

pub fn clone_keypair(keypair: &Keypair) -> Keypair { keypair.insecure_clone() }

pub async fn patch_program_data_account(
    test_f: &TestFixture,
    program_id: &Pubkey,
    upgrade_authority: Option<Pubkey>,
) {
    let programdata_address = get_program_data_address(program_id);
    let program_data = bincode::serialize(&UpgradeableLoaderState::ProgramData {
        slot: 0,
        upgrade_authority_address: upgrade_authority,
    })
    .unwrap();

    test_f
        .patch_account(programdata_address, 0, &program_data)
        .await;
}

pub fn create_funded_system_program_account(setup: &mut ProgramTest, public_key: &Pubkey) {
    let account = Account {
        lamports: 1_000_000 * LAMPORTS_PER_SOL,
        data: vec![],
        owner: system_program::ID,
        executable: false,
        rent_epoch: 0,
    };

    setup.add_genesis_account(*public_key, account);
}

pub async fn wrap_native_sol(
    test_f: &TestFixture,
    user: &Pubkey,
    amount: u64,
) -> anyhow::Result<()> {
    let deployer = test_f.deployer.pubkey();
    let mint = spl_token::native_mint::ID;

    let user_associated_token_account =
        spl_associated_token_account::get_associated_token_address(user, &mint);
    let ctx = test_f.context.borrow_mut();
    let tx = Transaction::new_signed_with_payer(
        &[
            spl_associated_token_account::instruction::create_associated_token_account(
                &deployer,
                user,
                &mint,
                &spl_token::ID,
            ),
            solana_sdk::system_instruction::transfer(
                &deployer,
                &user_associated_token_account,
                amount,
            ),
            spl_token::instruction::sync_native(&spl_token::ID, &user_associated_token_account)?,
        ],
        Some(&deployer),
        &[&test_f.deployer],
        ctx.last_blockhash,
    );

    ctx.banks_client.process_transaction(tx).await?;

    Ok(())
}

pub fn add_external_program_to_genesis(
    setup: &mut ProgramTest,
    program_id: Pubkey,
    program_file_path: &str,
) {
    let elf = read(program_file_path).expect("Failed to read program file");
    let program_accounts =
        bpf_loader_upgradeable_program_accounts(&program_id, &elf, &Rent::default());

    for (address, account) in program_accounts {
        setup.add_genesis_account(address, account);
    }
}

pub fn bpf_loader_upgradeable_program_accounts(
    program_id: &Pubkey,
    elf: &[u8],
    rent: &Rent,
) -> [(Pubkey, Account); 2] {
    let programdata_address = get_program_data_address(program_id);
    let program_account = {
        let space = UpgradeableLoaderState::size_of_program();
        let lamports = rent.minimum_balance(space);
        let data = bincode::serialize(&UpgradeableLoaderState::Program {
            programdata_address,
        })
        .unwrap();
        Account {
            lamports,
            data,
            owner: bpf_loader_upgradeable::id(),
            executable: true,
            rent_epoch: 0,
        }
    };
    let programdata_account = {
        let space = UpgradeableLoaderState::size_of_programdata_metadata() + elf.len();
        let lamports = rent.minimum_balance(space);
        let mut data = bincode::serialize(&UpgradeableLoaderState::ProgramData {
            slot: 0,
            upgrade_authority_address: Some(Pubkey::default()),
        })
        .unwrap();
        data.extend_from_slice(elf);
        Account {
            lamports,
            data,
            owner: bpf_loader_upgradeable::id(),
            executable: false,
            rent_epoch: 0,
        }
    };
    [
        (*program_id, program_account),
        (programdata_address, programdata_account),
    ]
}

pub fn load_zero_copy<T: Discriminator + ZeroCopy>(account: &mut Account) -> &mut T {
    let disc = T::DISCRIMINATOR;
    bytemuck::from_bytes_mut(&mut account.data[disc.len()..mem::size_of::<T>() + disc.len()])
}
