#[cfg(test)]
mod test_permissions {
    use crate::stablecoin::utils::Cep18Modality;
    use crate::stablecoin_contract::tests::{
        setup_with_args, TOKEN_DECIMALS, TOKEN_NAME, TOKEN_SYMBOL, TOKEN_TOTAL_SUPPLY,
    };
    use crate::stablecoin_contract::{Cep18HostRef, Cep18InitArgs};
    use alloc::string::ToString;
    use alloc::vec;
    use odra::casper_types::U256;
    use odra::host::{HostEnv, HostRef};
    use odra::Address;

    fn setup() -> (HostEnv, Address, Address, Address, Address, Address, Address, Cep18HostRef){
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
        (env, master_minter, controller_1, minter_1, blacklister, pauser, user, cep18_token)
    }

    #[test]
    fn test_minter_permissions() {
        let (env, master_minter, controller_1, minter_1, blacklister, pauser, user, mut cep18_token) = setup();
        cep18_token.env().set_caller(master_minter);
        cep18_token.configure_controller(&controller_1, &minter_1);
        assert!(env.emitted(&cep18_token, "ControllerConfigured"), "ControllerConfigured event not emitted");
        cep18_token.env().set_caller(controller_1);
        cep18_token.configure_minter_allowance(U256::from(10));
        assert!(env.emitted(&cep18_token, "MinterConfigured"), "MinterConfigured event not emitted");
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
    
    #[test]
    fn test_revoke_minter_and_controller(){
        let (env, master_minter, controller_1, minter_1, blacklister, pauser, user, mut cep18_token) = setup();
        cep18_token.env().set_caller(master_minter);
        cep18_token.configure_controller(&controller_1, &minter_1);
        cep18_token.env().set_caller(controller_1);
        cep18_token.remove_minter();
        assert!(env.emitted(&cep18_token, "MinterRemoved"), "MinterRemoved event not emitted");
        cep18_token.env().set_caller(master_minter);
        cep18_token.remove_controller(&controller_1);
        assert!(env.emitted(&cep18_token, "ControllerRemoved"), "ControllerRemoved event not emitted");
    }
}
