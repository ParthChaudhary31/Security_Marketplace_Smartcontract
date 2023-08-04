#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod token {
    use ink::storage::Mapping;
    use scale::{Decode,Encode};

    /// Storage structure
    #[ink(storage)]
    pub struct Token {
        /// Total token supply.
        pub total_supply: Balance,
        /// Mapping from owner to number of owned token.
        pub balances: Mapping<AccountId, Balance>,
        /// Owner of the contract
        pub owner: AccountId,
    }

    #[derive(Debug, PartialEq, Eq, Encode, Decode, Clone, Copy)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        UnAuthorisedCall,
    }
    

    /// The result type.
    pub type Result<T> = core::result::Result<T, Error>;

    impl Token {
        /// Creates a new token contract with the specified address as owner.
        #[ink(constructor)]
        pub fn new(owner: AccountId) -> Self {
            Self {
                owner,
                total_supply : Default::default(),
                balances : Mapping::default(),
            }
        }

        // Mints 'amount' amount of token to 'owner' address.
        #[ink(message)]
        pub fn mint(&mut self, owner: AccountId, amount: Balance)-> Result<()> {
            let caller= self.env().caller();
            if self.owner != caller {
                return Err(Error::UnAuthorisedCall);
            }
            self.balances.insert(owner, &amount);
            self.total_supply += amount;
            Ok(())
        }

        // Returns the total token supply.
        #[ink(message)]
        pub fn total_supply(&self) -> Balance {
            self.total_supply
        }

        /* Returns the account balance for the specified `owner`.
        Returns `0` if the account is non-existent. */
        #[ink(message)]
        pub fn balance_of(&self, owner: AccountId) -> Balance {
            self.balance_of_impl(&owner)
        }

        //Inline function for total_supply.
        #[inline]
        fn balance_of_impl(&self, owner: &AccountId) -> Balance {
            self.balances.get(owner).unwrap_or_default()
        }

    }
}



#[cfg(test)]
mod test_cases {
    // use ink::primitives::AccountId;

    use super ::*;
    #[cfg(feature = "ink-experimental-engine")]
    use crate::digital_certificate::digital_certificate;
    // fn random_acoount_id() -> AccountId {
    //     AccountId::from([0x42;32])
    // }

    #[test]
    fn test_case_1() {
        let accounts = 
        ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
        ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
        ink::env::test::set_callee::<ink::env::DefaultEnvironment>(accounts.bob);
        let contract = token::Token::new(accounts.alice);
        // contract.mint(accounts.alice,1);
        let contract_owner = contract.owner;
        //Asserting Alice is the owner of contract.
        assert_eq!(contract_owner,accounts.alice);
    }

    #[test]
    fn test_case_2(){
        let accounts = 
        ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
        ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
        ink::env::test::set_callee::<ink::env::DefaultEnvironment>(accounts.bob);
        let mut contract = token::Token::new(accounts.alice);
        let _res = contract.mint(accounts.alice,1);
        let total_contract_supply = contract.total_supply();
        //Asserting Total supply will be auto updated.
        assert_eq!(total_contract_supply,1);
    }

    #[test]
    fn test_case_3(){
        let accounts = 
        ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
        ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
        ink::env::test::set_callee::<ink::env::DefaultEnvironment>(accounts.bob);
        let mut contract = token::Token::new(accounts.alice);
        let _res = contract.mint(accounts.alice,1);
        let balance_of_alice = contract.balance_of(accounts.alice);
        //Asserting Alice's balance will be updated.
        assert_eq!(balance_of_alice,1);
    }

    #[test]
    fn test_case_4(){
        let accounts = 
        ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
        ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
        ink::env::test::set_callee::<ink::env::DefaultEnvironment>(accounts.bob);
        let mut contract = token::Token::new(accounts.alice);
        ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.charlie);
        let error1 = contract.mint(accounts.charlie, 1000);
        //Asserting a Third person (charlie) cannot call mint function.
        assert!(error1.is_err());
    }
}
