#[cfg(test)]
mod test_permissions {
    use crate::stablecoin::utils::StablecoinModality;
    use crate::stablecoin::setup_tests::{
        setup_with_args, TOKEN_DECIMALS, TOKEN_NAME, TOKEN_SYMBOL, TOKEN_TOTAL_SUPPLY,
    };
    use crate::stablecoin::{StablecoinHostRef, StablecoinInitArgs};
    use alloc::string::ToString;
    use alloc::vec;
    use odra::casper_types::U256;
    use odra::host::HostEnv;
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
        let stablecoin = setup_with_args(&env, args);
        (
            env,
            master_minter,
            controller_1,
            minter_1,
            blacklister,
            pauser,
            user,
            stablecoin,
        )
    }

    #[test]
    fn test_minter_permissions() {
        let (env, master_minter, controller_1, minter_1, .., user, mut stablecoin) = setup();
        env.set_caller(master_minter);
        stablecoin.configure_controller(&controller_1, &minter_1);
        assert!(
            env.emitted(&stablecoin, "ControllerConfigured"),
            "ControllerConfigured event not emitted"
        );
        env.set_caller(controller_1);
        stablecoin.configure_minter_allowance(U256::from(10));
        assert!(
            env.emitted(&stablecoin, "MinterConfigured"),
            "MinterConfigured event not emitted"
        );
        env.set_caller(minter_1);
        // try to mint legally, but exceed the allowance
        let result: Result<(), odra::OdraError> = stablecoin.try_mint(&user, U256::from(11));
        match result {
            Ok(_) => {
                panic!("Security Incident: Mint that exceeds the Allowance went through!")
            }
            _ => {}
        }
        // try to mint illegally
        env.set_caller(user);
        let result: Result<(), odra::OdraError> = stablecoin.try_mint(&user, U256::from(10));
        match result {
            Ok(_) => {
                panic!("Security Incident: Illegal mint went through!")
            }
            _ => {}
        }
        // remove the minter
        env.set_caller(controller_1);
        stablecoin.remove_minter();
        // try to mint with disabled minter
        env.set_caller(minter_1);
        let result: Result<(), odra::OdraError> = stablecoin.try_mint(&user, U256::from(10));
        match result {
            Ok(_) => {
                panic!("Security Incident: Illegal mint went through!")
            }
            _ => {}
        }
    }

    #[test]
    fn test_revoke_minter_and_controller() {
        let (env, master_minter, controller_1, minter_1, .., mut stablecoin) = setup();
        env.set_caller(master_minter);
        stablecoin.configure_controller(&controller_1, &minter_1);
        env.set_caller(controller_1);
        stablecoin.remove_minter();
        assert!(
            env.emitted(&stablecoin, "MinterRemoved"),
            "MinterRemoved event not emitted"
        );
        env.set_caller(master_minter);
        stablecoin.remove_controller(&controller_1);
        assert!(
            env.emitted(&stablecoin, "ControllerRemoved"),
            "ControllerRemoved event not emitted"
        );
    }

    #[test]
    fn must_not_mint_when_paused() {
        let (env, master_minter, controller_1, minter_1, .., pauser, user, mut stablecoin) =
            setup();
        env.set_caller(master_minter);
        stablecoin.configure_controller(&controller_1, &minter_1);
        env.set_caller(controller_1);
        stablecoin.configure_minter_allowance(U256::from(10));
        env.set_caller(pauser);
        stablecoin.pause();
        env.set_caller(minter_1);
        let result: Result<(), odra::OdraError> = stablecoin.try_mint(&user, U256::from(10));
        match result {
            Ok(_) => {
                panic!("Security Incident: Illegal mint went through!")
            }
            _ => {}
        }
    }
}
