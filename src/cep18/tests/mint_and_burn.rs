#[cfg(test)]
mod mint_and_burn_tests {
    use alloc::string::ToString;
    use alloc::vec;

    use odra::casper_types::U256;
    use odra::host::HostRef;
    use odra::ExecutionError::AdditionOverflow;

    use crate::cep18::errors::Error::{InsufficientBalance, InsufficientRights, MintBurnDisabled};
    use crate::cep18::utils::Cep18Modality;
    use crate::cep18_token::tests::{
        setup, setup_with_args, TOKEN_DECIMALS, TOKEN_NAME, TOKEN_OWNER_AMOUNT_1,
        TOKEN_OWNER_AMOUNT_2, TOKEN_SYMBOL, TOKEN_TOTAL_SUPPLY, TRANSFER_AMOUNT_1,
    };
    use crate::cep18_token::Cep18InitArgs;

    /*#[test]
    fn test_security_minter_rights() {
        // given a token with mint and burn enabled, and alice set as minter
        let env = odra_test::env();
        let alice = env.get_account(1);
        let bob = env.get_account(2);
        let args = Cep18InitArgs {
            symbol: TOKEN_SYMBOL.to_string(),
            name: TOKEN_NAME.to_string(),
            decimals: TOKEN_DECIMALS,
            initial_supply: TOKEN_TOTAL_SUPPLY.into(),
            admin_list: vec![],
            minter_list: vec![alice],
            modality: Some(Cep18Modality::MintAndBurn),
        };
        let mut cep18_token = setup_with_args(&env, args);
        let amount = TRANSFER_AMOUNT_1.into();

        // alice can mint tokens
        cep18_token.env().set_caller(alice);
        cep18_token.mint(&bob, &amount);
        assert_eq!(cep18_token.balance_of(&bob), amount);

        // and bob cannot
        cep18_token.env().set_caller(bob);
        let result = cep18_token.try_mint(&alice, &amount);
        assert_eq!(result.err().unwrap(), InsufficientRights.into());
    }*/
}
