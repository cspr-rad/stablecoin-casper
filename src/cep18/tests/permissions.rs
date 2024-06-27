#[cfg(test)]
mod test_illegal_mint {
    use crate::cep18::utils::Cep18Modality;
    use crate::cep18_token::tests::{
        setup_with_args, TOKEN_DECIMALS, TOKEN_NAME, TOKEN_SYMBOL, TOKEN_TOTAL_SUPPLY,
    };
    use crate::cep18_token::Cep18InitArgs;
    use alloc::string::ToString;
    use alloc::vec;
    use odra::casper_types::U256;
    use odra::host::HostRef;

    #[test]
    fn test_stablecoin_mint() {
        let env = odra_test::env();
        let master_minter = env.get_account(1);
        let controller_1 = env.get_account(2);
        let minter_1 = env.get_account(3);
        let blacklister = env.get_account(4);
        let pauser = env.get_account(5);
        let user = env.get_account(6);
        let args = Cep18InitArgs {
            symbol: TOKEN_SYMBOL.to_string(),
            name: TOKEN_NAME.to_string(),
            decimals: TOKEN_DECIMALS,
            initial_supply: TOKEN_TOTAL_SUPPLY.into(),
            master_minter_list: vec![master_minter],
            owner_list: vec![],
            pauser_list: vec![pauser],
            blacklister: blacklister,
            modality: Some(Cep18Modality::MintAndBurn),
        };
        let mut cep18_token = setup_with_args(&env, args);
        cep18_token.env().set_caller(master_minter);
        cep18_token.configure_controller(&controller_1, &minter_1);
        cep18_token.env().set_caller(controller_1);
        cep18_token.configure_minter_allowance(U256::from(10));
        cep18_token.env().set_caller(minter_1);
        // try to mint legally, but exceed the allowance
        let result: Result<(), odra::OdraError> = cep18_token.try_mint(&user, U256::from(11));
        match result {
            Ok(_) => {
                panic!("Security Incident: Mint that exceeds the Allowance went through!")
            }
            _ => {}
        }
        // try to mint illegally
        cep18_token.env().set_caller(user);
        let result: Result<(), odra::OdraError> = cep18_token.try_mint(&user, U256::from(10));
        match result {
            Ok(_) => {
                panic!("Security Incident: Illegal mint went through!")
            }
            _ => {}
        }
        // remove the minter
        cep18_token.env().set_caller(controller_1);
        cep18_token.remove_minter();
        // try to mint with disabled minter
        cep18_token.env().set_caller(minter_1);
        let result: Result<(), odra::OdraError> = cep18_token.try_mint(&user, U256::from(10));
        match result {
            Ok(_) => {
                panic!("Security Incident: Illegal mint went through!")
            }
            _ => {}
        }
    }
}
