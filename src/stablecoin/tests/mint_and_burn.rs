#[cfg(test)]
mod mint_and_burn_tests {
    use crate::stablecoin::utils::StablecoinModality;
    use crate::stablecoin_contract::tests::{
        setup_with_args, TOKEN_DECIMALS, TOKEN_NAME, TOKEN_SYMBOL, TOKEN_TOTAL_SUPPLY,
    };
    use crate::stablecoin_contract::{StablecoinHostRef, StablecoinInitArgs};
    use alloc::string::ToString;
    use alloc::vec;
    use odra::casper_types::U256;
    use odra::host::{HostEnv, HostRef};
    use odra::Address;

    fn setup() -> (
        HostEnv,
        Address,
        Address,
        Address,
        Address,
        Address,
        Address,
        StablecoinHostRef,
    ) {
        let env = odra_test::env();
        let master_minter = env.get_account(1);
        let controller_1 = env.get_account(2);
        let minter_1 = env.get_account(3);
        let blacklister = env.get_account(4);
        let pauser = env.get_account(5);
        let user = env.get_account(6);
        let args = StablecoinInitArgs {
            symbol: TOKEN_SYMBOL.to_string(),
            name: TOKEN_NAME.to_string(),
            decimals: TOKEN_DECIMALS,
            initial_supply: TOKEN_TOTAL_SUPPLY.into(),
            master_minter_list: vec![master_minter],
            owner_list: vec![],
            pauser_list: vec![pauser],
            blacklister: blacklister,
            modality: Some(StablecoinModality::MintAndBurn),
        };
        let cep18_token = setup_with_args(&env, args);
        (
            env,
            master_minter,
            controller_1,
            minter_1,
            blacklister,
            pauser,
            user,
            cep18_token,
        )
    }

    #[test]
    fn test_stablecoin_mint() {
        let (env, master_minter, controller_1, minter_1, .., user, mut cep18_token) = setup();
        cep18_token.env().set_caller(master_minter);
        cep18_token.configure_controller(&controller_1, &minter_1);
        cep18_token.env().set_caller(controller_1);
        cep18_token.configure_minter_allowance(U256::from(10));
        cep18_token.env().set_caller(minter_1);
        cep18_token.mint(&user, U256::from(10));
        assert_eq!(cep18_token.minter_allowance(&minter_1), U256::from(0));
        assert!(env.emitted(&cep18_token, "Mint"), "Mint event not emitted")
    }

    #[test]
    fn test_stablecoin_burn() {
        //todo
    }
}
