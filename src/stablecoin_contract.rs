//! CEP-18 Casper Fungible Token standard implementation.
use odra::prelude::*;
use odra::{casper_types::U256, Address, Mapping, SubModule, UnwrapOrRevert, Var};

use crate::stablecoin::errors::Error;

use crate::stablecoin::events::{
    Blacklist, BlacklisterChanged, Burn, ControllerConfigured, ControllerRemoved,
    DecreaseAllowance, IncreaseAllowance, Mint, MinterConfigured, MinterRemoved, Paused,
    SetAllowance, Transfer, TransferFrom, Unblacklist, Unpaused,
};
use crate::stablecoin::storage::Roles::{self, Role};
use crate::stablecoin::storage::{
    Cep18AllowancesStorage, Cep18BalancesStorage, Cep18DecimalsStorage,
    Cep18MinterAllowancesStorage, Cep18NameStorage, Cep18SymbolStorage, Cep18TotalSupplyStorage,
    StablecoinRoles,
};
use crate::stablecoin::utils::Cep18Modality;

/// CEP-18 token module
#[odra::module(events = [Mint, Burn, SetAllowance, IncreaseAllowance, DecreaseAllowance, Transfer, TransferFrom])]
pub struct Cep18 {
    decimals: SubModule<Cep18DecimalsStorage>,
    symbol: SubModule<Cep18SymbolStorage>,
    name: SubModule<Cep18NameStorage>,
    total_supply: SubModule<Cep18TotalSupplyStorage>,
    balances: SubModule<Cep18BalancesStorage>,
    allowances: SubModule<Cep18AllowancesStorage>,
    minter_allowances: SubModule<Cep18MinterAllowancesStorage>,
    roles: SubModule<StablecoinRoles>,
    controllers: Mapping<Address, Address>,
    // The Blacklister for this Contract
    blacklister: Var<Address>,
    paused: Var<bool>,
    /// This stores all Stablecoin Roles (MasterMinters, Owners, Pauser, ...)
    modality: Var<Cep18Modality>,
}

#[odra::module]
impl Cep18 {
    /// Initializes the contract with the given metadata, initial supply, security and modality.
    #[allow(clippy::too_many_arguments)]
    pub fn init(
        &mut self,
        symbol: String,
        name: String,
        decimals: u8,
        initial_supply: U256,
        // the master_minter can't mint
        master_minter_list: Vec<Address>,
        owner_list: Vec<Address>,
        pauser_list: Vec<Address>,
        blacklister: Address,
        modality: Option<Cep18Modality>,
    ) {
        let caller = self.env().caller();
        // set the metadata
        self.symbol.set(symbol);
        self.name.set(name);
        self.decimals.set(decimals);
        self.total_supply.set(initial_supply);

        // mint the initial supply for the caller
        self.balances.set(&caller, initial_supply);
        self.env().emit_event(Mint {
            recipient: caller,
            amount: initial_supply,
        });

        for master_minter in master_minter_list {
            self.roles
                .configure_role(&Roles::MasterMinter, &master_minter);
        }

        for owner in owner_list {
            self.roles.configure_role(&Roles::Owner, &owner);
        }

        for pauser in pauser_list {
            self.roles.configure_role(&Roles::Pauser, &pauser);
        }

        self.roles.configure_role(&Roles::Blacklister, &blacklister);

        // set the modality
        if let Some(modality) = modality {
            self.modality.set(modality);
        }
    }

    /// Returns the name of the token.
    pub fn name(&self) -> String {
        self.name.get()
    }

    /// Returns the symbol of the token.
    pub fn symbol(&self) -> String {
        self.symbol.get()
    }

    /// Returns the number of decimals the token uses.
    pub fn decimals(&self) -> u8 {
        self.decimals.get()
    }

    /// Returns the total supply of the token.
    pub fn total_supply(&self) -> U256 {
        self.total_supply.get()
    }

    /// Returns the balance of the given address.
    pub fn balance_of(&self, address: &Address) -> U256 {
        self.balances.get_or_default(address)
    }

    /// Returns the amount of tokens the owner has allowed the spender to spend.
    pub fn allowance(&self, owner: &Address, spender: &Address) -> U256 {
        self.allowances.get_or_default(owner, spender)
    }

    /// Approves the spender to spend the given amount of tokens on behalf of the caller.
    pub fn approve(&mut self, spender: &Address, amount: &U256) {
        self.require_unpaused();
        self.require_not_role(&self.caller(), &Roles::Blacklisted);
        self.require_not_role(spender, &Roles::Blacklisted);
        let owner = self.env().caller();
        if owner == *spender {
            self.env().revert(Error::CannotTargetSelfUser);
        }

        self.allowances.set(&owner, spender, *amount);
        self.env().emit_event(SetAllowance {
            owner,
            spender: *spender,
            allowance: *amount,
        });
    }

    /// Decreases the allowance of the spender by the given amount.
    pub fn decrease_allowance(&mut self, spender: &Address, decr_by: &U256) {
        let owner = self.env().caller();
        let allowance = self.allowance(&owner, spender);
        self.allowances
            .set(&owner, spender, allowance.saturating_sub(*decr_by));
        self.env().emit_event(DecreaseAllowance {
            owner,
            spender: *spender,
            allowance,
            decr_by: *decr_by,
        });
    }

    /// Increases the allowance of the spender by the given amount.
    pub fn increase_allowance(&mut self, spender: &Address, inc_by: &U256) {
        let owner = self.env().caller();
        if owner == *spender {
            self.env().revert(Error::CannotTargetSelfUser);
        }
        let allowance = self.allowances.get_or_default(&owner, spender);

        self.allowances
            .set(&owner, spender, allowance.saturating_add(*inc_by));
        self.env().emit_event(IncreaseAllowance {
            owner,
            spender: *spender,
            allowance,
            inc_by: *inc_by,
        });
    }

    /// Transfers tokens from the caller to the recipient.
    pub fn transfer(&mut self, recipient: &Address, amount: &U256) {
        let caller = self.env().caller();
        if caller == *recipient {
            self.env().revert(Error::CannotTargetSelfUser);
        }
        self.raw_transfer(&caller, recipient, amount);
    }

    /// Transfers tokens from the owner to the recipient using the spender's allowance.
    pub fn transfer_from(&mut self, owner: &Address, recipient: &Address, amount: &U256) {
        self.require_not_role(owner, &Roles::Blacklisted);
        self.require_not_role(recipient, &Roles::Blacklisted);
        let spender = self.env().caller();
        if owner == recipient {
            self.env().revert(Error::CannotTargetSelfUser);
        }
        if amount.is_zero() {
            return;
        }
        let allowance = self.allowance(owner, &spender);
        self.allowances.set(
            owner,
            recipient,
            allowance
                .checked_sub(*amount)
                .unwrap_or_revert_with(&self.env(), Error::InsufficientAllowance),
        );
        self.env().emit_event(TransferFrom {
            spender,
            owner: *owner,
            recipient: *recipient,
            amount: *amount,
        });
        self.raw_transfer(owner, recipient, amount);
    }

    /// Burns the given amount of tokens from the given address.
    pub fn burn(&mut self, owner: &Address, amount: &U256) {
        self.assert_burn_and_mint_enabled();

        if self.env().caller() != *owner {
            self.env().revert(Error::InvalidBurnTarget);
        }

        if self.balance_of(owner) < *amount {
            self.env().revert(Error::InsufficientBalance);
        }

        self.raw_burn(owner, amount);
    }

    // Functions that are specific to Stablecoin start here

    /// Mints new tokens and assigns them to the given address.
    pub fn mint(&mut self, owner: &Address, amount: U256) {
        self.require_role(&self.caller(), &Roles::Minter);
        self.require_not_role(owner, &Roles::Blacklisted);
        self.assert_burn_and_mint_enabled();
        // minter allowance must be sufficient
        let minter_allowance: U256 = self.minter_allowances.get_or_default(&self.env().caller());
        if minter_allowance < amount {
            self.env().revert(Error::InsufficientMinterAllowance);
        }
        // decrease minter allowance
        self.minter_allowances
            .subtract(&self.env().caller(), amount);
        self.raw_mint(owner, &amount);
    }

    /// Pause this contract
    pub fn pause(&mut self) {
        self.require_role(&self.caller(), &Roles::Pauser);
        self.require_not_role(&self.caller(), &Roles::Blacklisted);
        self.paused.set(true);
        self.env().emit_event(Paused {});
    }

    /// Unpause this contract
    pub fn unpause(&mut self) {
        self.require_role(&self.caller(), &Roles::Pauser);
        self.require_not_role(&self.caller(), &Roles::Blacklisted);
        self.paused.set(false);
        self.env().emit_event(Unpaused {});
    }

    /// Blacklist an account
    pub fn blacklist(&mut self, account: &Address) {
        self.require_role(&self.caller(), &Roles::Blacklister);
        self.roles.configure_role(&Roles::Blacklisted, account);
        self.env().emit_event(Blacklist { account: *account });
    }

    /// Remove an account from the Blacklist
    pub fn unblacklist(&mut self, account: &Address) {
        self.require_role(&self.caller(), &Roles::Blacklister);
        self.roles.revoke_role(&Roles::Blacklisted, account);
        self.env().emit_event(Unblacklist { account: *account });
    }

    /// Update the Blacklister, can only be called by Owner
    pub fn update_blacklister(&mut self, new_blacklister: &Address) {
        self.require_role(&self.caller(), &Roles::Owner);
        self.roles.revoke_role(
            &Roles::Blacklister,
            &self
                .blacklister
                .get()
                // borrow checker is unhappy if we unwrap_or_revert() here.
                .unwrap(),
        );
        self.roles
            .configure_role(&Roles::Blacklister, new_blacklister);
        self.env().emit_event(BlacklisterChanged {
            new_blacklister: *new_blacklister,
        });
        self.env().emit_event(BlacklisterChanged {
            new_blacklister: *new_blacklister,
        });
    }

    /// Configure minter allowance
    pub fn configure_minter_allowance(&mut self, minter_allowance: U256) {
        self.require_role(&self.caller(), &Roles::Controller);
        let minter = self.get_associated_minter(&self.caller());
        self.require_not_role(&minter, &Roles::Blacklisted);
        self.minter_allowances.set(&minter, minter_allowance);
        self.env().emit_event(MinterConfigured {
            minter,
            minter_allowance,
        });
    }

    /// Increase allowance for a minter
    pub fn increase_minter_allowance(&mut self, increment: U256) {
        self.require_role(&self.caller(), &Roles::Controller);
        let minter = self.get_associated_minter(&self.caller());
        self.require_not_role(&minter, &Roles::Blacklisted);
        self.minter_allowances.add(&minter, increment);
        self.env().emit_event(MinterConfigured {
            minter,
            minter_allowance: self.minter_allowance(&minter),
        });
    }

    /// Decrease allowance for a minter
    pub fn decrease_minter_allowance(&mut self, decrement: U256) {
        self.require_role(&self.caller(), &Roles::Controller);
        let minter = self.get_associated_minter(&self.caller());
        self.require_not_role(&minter, &Roles::Blacklisted);
        self.minter_allowances.subtract(&minter, decrement);
        self.env().emit_event(MinterConfigured {
            minter,
            minter_allowance: self.minter_allowance(&minter),
        });
    }

    /// Add a controller, minter pair
    pub fn configure_controller(&mut self, controller: &Address, minter: &Address) {
        self.require_role(&self.caller(), &Roles::MasterMinter);
        self.require_not_role(controller, &Roles::Blacklisted);
        self.require_not_role(minter, &Roles::Blacklisted);
        self.roles.configure_role(&Roles::Controller, controller);
        self.roles.configure_role(&Roles::Minter, minter);
        self.controllers.set(&controller, *minter);
        self.env().emit_event(ControllerConfigured {
            controller: *controller,
            minter: *minter,
        });
    }

    /// Remove a controller
    pub fn remove_controller(&mut self, controller: &Address) {
        self.require_role(&self.caller(), &Roles::MasterMinter);
        self.roles.revoke_role(&Roles::Controller, controller);
        self.env().emit_event(ControllerRemoved {
            controller: *controller,
        });
    }

    /// Remove the minter role from an account
    pub fn remove_minter(&mut self) {
        self.require_role(&self.caller(), &Roles::Controller);
        self.require_not_role(&self.caller(), &Roles::Blacklisted);
        let minter: Address = self.get_associated_minter(&self.env().caller());
        self.roles.revoke_role(&Roles::Minter, &minter);
        self.env().emit_event(MinterRemoved { minter })
    }

    // Queries start here

    /// Query if an account is a minter
    pub fn is_minter(&self, account: &Address) -> bool {
        self.roles.is_minter(account)
    }

    /// Query if an account is blacklisted
    pub fn is_blacklisted(&self, account: &Address) -> bool {
        self.roles.is_blacklisted(account)
    }

    /// Query the owners of this account
    pub fn is_owner(&self, account: &Address) -> bool {
        self.roles.is_owner(account)
    }

    /// Query the owners of this account
    pub fn is_pausers(&self, account: &Address) -> bool {
        self.roles.is_pauser(account)
    }

    /// Query a minter for a controller
    pub fn get_minter(&mut self) -> Address {
        self.require_role(&self.caller(), &Roles::Controller);
        self.require_not_role(&self.caller(), &Roles::Blacklisted);
        self.get_associated_minter(&self.caller())
    }

    /// Query the allowance of a minter
    pub fn minter_allowance(&self, minter: &Address) -> U256 {
        self.minter_allowances.get_or_default(minter)
    }

    fn require_unpaused(&self) {
        if self.paused.get().unwrap_or(false) {
            self.env().revert(Error::ContractIsPaused);
        }
    }

    fn require_role(&mut self, account: &Address, role: &Role) {
        if !self.roles.has_role(&role, account) {
            self.env().revert(Error::InsufficientRights);
        }
    }

    fn require_not_role(&mut self, account: &Address, role: &Role) {
        if self.roles.has_role(&role, account) {
            self.env().revert(Error::InsufficientRights)
        }
    }

    // Get the minter that is associated with the controller
    fn get_associated_minter(&mut self, controller: &Address) -> Address {
        self.controllers
            .get(&controller)
            .unwrap_or_revert_with(&self.env(), Error::MissingController)
    }

    fn caller(&self) -> Address {
        self.env().caller()
    }
}

impl Cep18 {
    /// Transfers tokens from the sender to the recipient without checking the permissions.
    fn raw_transfer(&mut self, sender: &Address, recipient: &Address, amount: &U256) {
        self.require_unpaused();
        self.require_not_role(&self.caller(), &Roles::Blacklisted);
        if *amount > self.balances.get_or_default(sender) {
            self.env().revert(Error::InsufficientBalance)
        }

        if amount > &U256::zero() {
            self.balances.subtract(sender, *amount);
            self.balances.add(recipient, *amount);
        }

        self.env().emit_event(Transfer {
            sender: *sender,
            recipient: *recipient,
            amount: *amount,
        });
    }

    /// Mints new tokens and assigns them to the given address without checking the permissions.
    fn raw_mint(&mut self, owner: &Address, amount: &U256) {
        self.require_unpaused();
        self.require_not_role(&self.caller(), &Roles::Blacklisted);
        self.total_supply.add(*amount);
        self.balances.add(owner, *amount);

        self.env().emit_event(Mint {
            recipient: *owner,
            amount: *amount,
        });
    }

    /// Burns the given amount of tokens from the given address without checking the permissions.
    fn raw_burn(&mut self, owner: &Address, amount: &U256) {
        self.require_unpaused();
        self.require_not_role(&self.caller(), &Roles::Blacklisted);
        self.total_supply.subtract(*amount);
        self.balances.subtract(owner, *amount);

        self.env().emit_event(Burn {
            owner: *owner,
            amount: *amount,
        });
    }

    fn assert_burn_and_mint_enabled(&mut self) {
        // check if mint_burn is enabled
        if !self.modality.get_or_default().mint_and_burn_enabled() {
            self.env().revert(Error::MintBurnDisabled);
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use alloc::string::ToString;
    use alloc::vec;

    use crate::stablecoin::utils::Cep18Modality;
    use odra::casper_types::account::AccountHash;
    use odra::casper_types::ContractPackageHash;
    use odra::host::{Deployer, HostEnv, HostRef};
    use odra::Address;

    use crate::stablecoin_contract::{Cep18HostRef, Cep18InitArgs};

    pub const TOKEN_NAME: &str = "USDCoin";
    pub const TOKEN_SYMBOL: &str = "USDC";
    pub const TOKEN_DECIMALS: u8 = 100;
    pub const TOKEN_TOTAL_SUPPLY: u64 = 1_000_000_000;
    pub const TRANSFER_AMOUNT_1: u64 = 200_001;
    pub const ALLOWANCE_AMOUNT_1: u64 = 456_789;
    pub const ALLOWANCE_AMOUNT_2: u64 = 87_654;

    pub fn setup(enable_mint_and_burn: bool) -> Cep18HostRef {
        let env = odra_test::env();
        let modality = if enable_mint_and_burn {
            Cep18Modality::MintAndBurn
        } else {
            Cep18Modality::None
        };
        let blacklister = env.get_account(3);

        let init_args = Cep18InitArgs {
            symbol: TOKEN_SYMBOL.to_string(),
            name: TOKEN_NAME.to_string(),
            decimals: TOKEN_DECIMALS,
            initial_supply: TOKEN_TOTAL_SUPPLY.into(),
            master_minter_list: vec![],
            owner_list: vec![],
            pauser_list: vec![],
            blacklister: blacklister,
            modality: Some(modality),
        };
        setup_with_args(&env, init_args)
    }

    pub fn setup_with_args(env: &HostEnv, args: Cep18InitArgs) -> Cep18HostRef {
        Cep18HostRef::deploy(env, args)
    }

    pub fn invert_address(address: Address) -> Address {
        match address {
            Address::Account(hash) => Address::Contract(ContractPackageHash::new(hash.value())),
            Address::Contract(hash) => Address::Account(AccountHash(hash.value())),
        }
    }

    #[test]
    fn should_have_queryable_properties() {
        let stablecoin_token = setup(false);

        assert_eq!(stablecoin_token.name(), TOKEN_NAME);
        assert_eq!(stablecoin_token.symbol(), TOKEN_SYMBOL);
        assert_eq!(stablecoin_token.decimals(), TOKEN_DECIMALS);
        assert_eq!(stablecoin_token.total_supply(), TOKEN_TOTAL_SUPPLY.into());

        let admin_key = stablecoin_token.env().caller();
        let admin_balance = stablecoin_token.balance_of(&admin_key);
        assert_eq!(admin_balance, TOKEN_TOTAL_SUPPLY.into());

        let contract_balance = stablecoin_token.balance_of(stablecoin_token.address());
        assert_eq!(contract_balance, 0.into());

        // Ensures that Account and Contract ownership is respected, and we're not keying ownership under
        // the raw bytes regardless of variant.
        let inverted_admin_key = invert_address(admin_key);
        let inverted_admin_balance = stablecoin_token.balance_of(&inverted_admin_key);
        assert_eq!(inverted_admin_balance, 0.into());
    }
}
