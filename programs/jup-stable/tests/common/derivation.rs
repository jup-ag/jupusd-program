use anchor_spl::metadata;
use solana_sdk::pubkey::Pubkey;
use spl_associated_token_account::get_associated_token_address_with_program_id;

pub fn find_config() -> Pubkey {
    let (pubkey, _bump) = Pubkey::find_program_address(&[b"config"], &jup_stable::id());
    pubkey
}

pub fn find_authority() -> Pubkey {
    let (pubkey, _bump) = Pubkey::find_program_address(&[b"authority"], &jup_stable::id());
    pubkey
}

pub fn find_operator(authority: &Pubkey) -> Pubkey {
    let (pubkey, _bump) =
        Pubkey::find_program_address(&[b"operator", authority.as_ref()], &jup_stable::id());
    pubkey
}

pub fn find_vault(stablecoin_mint: &Pubkey) -> Pubkey {
    let (pubkey, _bump) =
        Pubkey::find_program_address(&[b"vault", stablecoin_mint.as_ref()], &jup_stable::id());
    pubkey
}

pub fn find_vault_token_account(stablecoin_mint: &Pubkey) -> Pubkey {
    get_associated_token_address_with_program_id(&find_authority(), stablecoin_mint, &spl_token::ID)
}

pub fn find_benefactor(authority: &Pubkey) -> Pubkey {
    let (pubkey, _bump) =
        Pubkey::find_program_address(&[b"benefactor", authority.as_ref()], &jup_stable::id());
    pubkey
}

pub fn find_metadata(mint: &Pubkey) -> Pubkey {
    let (pubkey, _bump) = Pubkey::find_program_address(
        &[b"metadata", &metadata::ID.to_bytes(), &mint.to_bytes()],
        &metadata::ID,
    );
    pubkey
}

pub fn find_event_authority() -> Pubkey {
    let (pubkey, _bump) = Pubkey::find_program_address(&[b"__event_authority"], &jup_stable::id());
    pubkey
}
