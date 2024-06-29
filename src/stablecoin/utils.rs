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
