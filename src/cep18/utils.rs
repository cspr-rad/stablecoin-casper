/// Security badge that can be assigned to an account to grant it certain permissions.
#[odra::odra_type]
pub enum Role {
    /// The account is a minter.
    Minter = 0,
    /// The account has no special permissions.
    None = 1,
    /// Stablecoin Masterminter.
    MasterMinter = 2,
    /// Stablecoin Blacklister.
    Blacklister = 3,
    /// Stablecoin Blacklisted - held by Addresses that have been Blacklisted.
    Blacklisted = 4,
    /// Stablecoin Pauser.
    Pauser = 5,
    /// Stablecoin Controller.
    Controller = 6,
    /// Stablecoin Owner
    Owner = 7,
}
impl Role {
    pub const VARIANTS: usize = 8;
}

/// Modality of the CEP-18 contract.
#[derive(Default)]
#[odra::odra_type]
pub enum Cep18Modality {
    /// No modailities are set.
    #[default]
    None = 0,
    /// The contract can mint and burn tokens.
    MintAndBurn = 1,
}

impl Cep18Modality {
    /// Returns true if the mint and burn functionality is enabled.
    pub fn mint_and_burn_enabled(&self) -> bool {
        matches!(self, Cep18Modality::MintAndBurn)
    }
}

// implement conversion from modality into u8
impl From<Cep18Modality> for u8 {
    fn from(modality: Cep18Modality) -> u8 {
        modality as u8
    }
}
