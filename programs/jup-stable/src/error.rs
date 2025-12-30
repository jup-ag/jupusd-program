use anchor_lang::prelude::*;

#[error_code]
pub enum JupStableError {
    #[msg("")]
    SomeError,
    #[msg("Admin Array Full")]
    AdminArrayFull,
    #[msg("Not Authorized")]
    NotAuthorized,
    #[msg("Bad Input")]
    BadInput,
    #[msg("Benefactor Disabled")]
    BenefactorDisabled,
    #[msg("Benefactor Active")]
    BenefactorActive,
    #[msg("Vault Not Active")]
    VaultNotActive,
    #[msg("Insufficient Amount")]
    InsufficientAmount,
    #[msg("Invalid Fee Rate")]
    InvalidFeeRate,
    #[msg("Mint Limit Exceeded")]
    MintLimitExceeded,
    #[msg("Redeem Limit Exceeded")]
    RedeemLimitExceeded,
    #[msg("Slippage Tolerance Exceeded")]
    SlippageToleranceExceeded,
    #[msg("Math Overflow")]
    MathOverflow,
    #[msg("Invalid LP Mint")]
    InvalidLPMint,
    #[msg("Invalid Vault Mint")]
    InvalidVaultMint,
    #[msg("Invalid Authority")]
    InvalidAuthority,
    #[msg("Invalid Vault Token Account")]
    InvalidVaultTokenAccount,
    #[msg("Invalid Token Program")]
    InvalidTokenProgram,
    #[msg("Invalid Vault Fee Token Account")]
    InvalidVaultFeeTokenAccount,
    #[msg("Bad Oracle")]
    BadOracle,
    #[msg("No Valid Price")]
    NoValidPrice,
    #[msg("Invalid Benefactor")]
    InvalidBenefactor,
    #[msg("Invalid Custodian")]
    InvalidCustodian,
    #[msg("Invalid Rate Limit Window")]
    InvalidPeriodLimit,
    #[msg("Missing Oracle Accounts")]
    MissingOracleAccounts,
    #[msg("No Oracles Found")]
    NoOraclesFound,
    #[msg("Zero Amount")]
    ZeroAmount,
    #[msg("Protocol Paused")]
    ProtocolPaused,
    #[msg("Operator Disabled")]
    OperatorDisabled,
    #[msg("Vault Disabled")]
    VaultDisabled,
    #[msg("Vault Enabled")]
    VaultEnabled,
    #[msg("Vault Is Dry")]
    VaultIsDry,
    #[msg("Invalid Peg Price USD")]
    InvalidPegPriceUSD,
    #[msg("No Valid Oracle")]
    NoValidOracle,
    #[msg("Price Confidence Too Wide")]
    PriceConfidenceTooWide,
    #[msg("Operator Cannot Delete Itself")]
    OperatorCannotDeleteItself,
}
