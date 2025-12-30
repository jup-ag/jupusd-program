use anchor_lang::prelude::*;

#[error_code]
pub enum PSmError {
    #[msg("")]
    SomeError,
    #[msg("Admin Array Full")]
    AdminArrayFull,
    #[msg("Not Authorized")]
    NotAuthorized,
    #[msg("Bad Input")]
    BadInput,
    #[msg("Duplicate Resources")]
    DuplicateRessources,
    #[msg("Protocol Paused")]
    ProtocolPaused,
    #[msg("Pool Not Active")]
    PoolNotActive,
    #[msg("Invalid Authority")]
    InvalidAuthority,
    #[msg("Invalid Redemption Mint")]
    InvalidRedemptionMint,
    #[msg("Invalid Settlement Mint")]
    InvalidSettlementMint,
    #[msg("Invalid Redemption Token Account")]
    InvalidRedemptionTokenAccount,
    #[msg("Invalid Settlement Token Account")]
    InvalidSettlementTokenAccount,
    #[msg("Invalid Token Program")]
    InvalidTokenProgram,
    #[msg("Insufficient Amount")]
    InsufficientAmount,
    #[msg("Insufficient Pool Balance")]
    InsufficientPoolBalance,
    #[msg("Zero Amount")]
    ZeroAmount,
    #[msg("Math Overflow")]
    MathOverflow,
    #[msg("No Admin Left")]
    NoAdminLeft,
}
