use std::{cell::RefCell, collections::HashMap, rc::Rc, sync::Mutex};

use anchor_lang::{system_program, Id};
use anchor_spl::metadata::Metadata;
use anyhow::Result;
use bincode::deserialize;
use once_cell::sync::Lazy;
use solana_account::{Account, AccountSharedData};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_program::hash::Hash;
use solana_program_pack::Pack;
use solana_program_test::{ProgramTest, ProgramTestContext};
use solana_sdk::{
    clock::Clock, native_token::LAMPORTS_PER_SOL, pubkey::Pubkey, signature::Keypair,
    signer::Signer, sysvar,
};

use crate::utils::{
    add_external_program_to_genesis, clone_keypair, create_funded_system_program_account,
    patch_program_data_account, RUST_LOG_DEFAULT,
};

const MAINNET_RPC_URL: &str = "https://api.mainnet-beta.solana.com";
static GLOBAL_CACHE: Lazy<Mutex<AccountCache>> = Lazy::new(|| Mutex::new(AccountCache::new()));

struct AccountCache(HashMap<Pubkey, Account>);

impl AccountCache {
    fn new() -> Self { Self(HashMap::new()) }
}

pub struct TestFixture {
    pub context: Rc<RefCell<ProgramTestContext>>,
    pub deployer: Keypair,
    pub rpc_client: RpcClient,
}

impl TestFixture {
    pub async fn new() -> TestFixture {
        let rpc_client = RpcClient::new_with_timeout(
            MAINNET_RPC_URL.to_string(),
            std::time::Duration::from_secs(60),
        );

        let mut program = ProgramTest::default();

        let deployer: Keypair = Keypair::new();
        let deployer_pubkey = deployer.pubkey();
        program.add_upgradeable_program_to_genesis("jup_stable", &jup_stable::ID);
        program.add_upgradeable_program_to_genesis("psm", &psm::ID);

        add_external_program_to_genesis(
            &mut program,
            Metadata::id(),
            "../../dependency/token_metadata.so",
        );

        create_funded_system_program_account(&mut program, &deployer_pubkey);
        let context = Rc::new(RefCell::new(program.start_with_context().await));

        solana_logger::setup_with_default(RUST_LOG_DEFAULT);
        let s = TestFixture {
            context: Rc::clone(&context),
            deployer,
            rpc_client,
        };

        patch_program_data_account(&s, &jup_stable::ID, Some(deployer_pubkey)).await;
        patch_program_data_account(&s, &psm::ID, Some(deployer_pubkey)).await;

        s
    }

    pub async fn fund_account(&self, address: &Pubkey) {
        let account = Account {
            lamports: 1_000_000 * LAMPORTS_PER_SOL,
            data: vec![],
            owner: system_program::ID,
            executable: false,
            rent_epoch: 0,
        };
        self.context
            .borrow_mut()
            .set_account(address, &account.into());
    }

    pub async fn patch_account(&self, address: Pubkey, offset: usize, data: &[u8]) {
        let mut account = self
            .context
            .borrow_mut()
            .banks_client
            .get_account(address)
            .await
            .unwrap()
            .unwrap();

        account.data[offset..offset + data.len()].copy_from_slice(data);

        self.context
            .borrow_mut()
            .set_account(&address, &account.into());
    }

    pub async fn mint_tokens(&self, token_account: &Pubkey, amount: u64) {
        let account = self.get_account(token_account).await;

        let mut token_account_state =
            spl_token_2022::state::Account::unpack(&account.data).unwrap();

        token_account_state.amount = amount;

        let mut buf = vec![0; 165];
        token_account_state.pack_into_slice(&mut buf);
        self.patch_account(*token_account, 0, &buf).await;
    }

    pub async fn load_and_deserialize<T: anchor_lang::AccountDeserialize>(
        &self,
        address: &Pubkey,
    ) -> T {
        let ai = self
            .context
            .borrow_mut()
            .banks_client
            .get_account(*address)
            .await
            .unwrap()
            .unwrap();

        T::try_deserialize(&mut ai.data.as_slice()).unwrap()
    }

    pub async fn get_account(&self, address: &Pubkey) -> solana_sdk::account::Account {
        self.context
            .borrow_mut()
            .banks_client
            .get_account(*address)
            .await
            .unwrap()
            .unwrap()
    }

    pub async fn set_account(&self, address: &Pubkey, account: solana_sdk::account::Account) {
        self.context
            .borrow_mut()
            .set_account(address, &account.into());
    }

    pub fn payer(&self) -> Pubkey { self.context.borrow().payer.pubkey() }

    pub fn payer_keypair(&self) -> Keypair { clone_keypair(&self.context.borrow().payer) }

    pub fn set_time(&self, timestamp: i64) {
        let clock = Clock {
            unix_timestamp: timestamp,
            ..Default::default()
        };
        self.context.borrow_mut().set_sysvar(&clock);
    }

    pub async fn advance_time(&self, seconds: i64) {
        let mut clock: Clock = self
            .context
            .borrow_mut()
            .banks_client
            .get_sysvar()
            .await
            .unwrap();
        clock.unix_timestamp += seconds;
        self.context.borrow_mut().set_sysvar(&clock);
        self.context
            .borrow_mut()
            .warp_forward_force_reward_interval_end()
            .unwrap();
    }

    pub async fn get_minimum_rent_for_size(&self, size: usize) -> u64 {
        self.context
            .borrow_mut()
            .banks_client
            .get_rent()
            .await
            .unwrap()
            .minimum_balance(size)
    }

    pub async fn get_latest_blockhash(&self) -> Hash {
        self.context
            .borrow_mut()
            .banks_client
            .get_latest_blockhash()
            .await
            .unwrap()
    }

    pub async fn get_slot(&self) -> u64 {
        self.context
            .borrow_mut()
            .banks_client
            .get_root_slot()
            .await
            .unwrap()
    }

    pub async fn get_clock(&self) -> Clock {
        deserialize::<Clock>(
            &self
                .context
                .borrow_mut()
                .banks_client
                .get_account(sysvar::clock::ID)
                .await
                .unwrap()
                .unwrap()
                .data,
        )
        .unwrap()
    }

    pub async fn replicate_account_from_mainnet(&self, account_pubkey: &Pubkey) -> Result<()> {
        let mut cache = GLOBAL_CACHE.lock().unwrap();

        if let Some(cached_account) = cache.0.get(account_pubkey) {
            self.context.borrow_mut().set_account(
                account_pubkey,
                &AccountSharedData::from(cached_account.clone()),
            );
            return Ok(());
        }

        let mainnet_account = self.rpc_client.get_account(account_pubkey).await?;
        let test_account = Account {
            lamports: mainnet_account.lamports,
            data: mainnet_account.data.clone(),
            owner: mainnet_account.owner,
            executable: mainnet_account.executable,
            rent_epoch: mainnet_account.rent_epoch,
        };

        cache.0.insert(*account_pubkey, test_account.clone());

        self.context
            .borrow_mut()
            .set_account(account_pubkey, &AccountSharedData::from(test_account));

        Ok(())
    }
}
