use solana_sdk::pubkey::Pubkey;

pub fn find_config() -> Pubkey {
    let (pubkey, _bump) = Pubkey::find_program_address(&[b"config"], &psm::id());
    pubkey
}

pub fn find_authority() -> Pubkey {
    let (pubkey, _bump) = Pubkey::find_program_address(&[b"authority"], &psm::id());
    pubkey
}

pub fn find_pool(redemption_mint: &Pubkey, settlement_mint: &Pubkey) -> Pubkey {
    let (pubkey, _bump) = Pubkey::find_program_address(
        &[b"pool", redemption_mint.as_ref(), settlement_mint.as_ref()],
        &psm::id(),
    );
    pubkey
}

pub fn find_pool_redemption_token_account(pool: &Pubkey) -> Pubkey {
    let (pubkey, _bump) = Pubkey::find_program_address(
        &[b"pool_redemption_token_account", pool.as_ref()],
        &psm::id(),
    );
    pubkey
}

pub fn find_pool_settlement_token_account(pool: &Pubkey) -> Pubkey {
    let (pubkey, _bump) = Pubkey::find_program_address(
        &[b"pool_settlement_token_account", pool.as_ref()],
        &psm::id(),
    );
    pubkey
}
