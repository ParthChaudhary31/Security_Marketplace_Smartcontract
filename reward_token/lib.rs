#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod rewardtoken {
    use ink::prelude::string::String;
    use ink::storage::Mapping;
    use scale::{Decode, Encode};

    #[derive(scale::Decode, scale::Encode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct RewardInfo {
        /// recipient's address
        pub recipient: AccountId,
        /// audit id which this reward corresponds to
        pub audit_id: u32,
        /// completion time would be percentage of total time in which report was completed.
        pub completion_time: u8,
        /// extensions
        pub extensions: u8,
        /// Final value amount
        pub amount: Balance,
        ///  submitted audit report ipfs_hash
        pub ipfs_hash: String,
    }

    #[derive(scale::Decode, scale::Encode, Default)]
    #[cfg_attr(
        feature = "std",
        derive( scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct Stats {
        pub successful_audits: u32,
        pub unsuccessful_audits: u32,
    }

    #[ink(storage)]
    pub struct Rewardtoken {
        pub current_id: u32,
        pub balances: Mapping<AccountId, Stats>,
        pub owner: AccountId,
        pub rewarded_tokens: Mapping<u32, RewardInfo>,
    }

    #[derive(Debug, PartialEq, Eq, Encode, Decode, Clone, Copy)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        UnAuthorisedCall,
    }

    pub type Result<T> = core::result::Result<T, Error>;

    impl Rewardtoken {
        #[ink(constructor)]
        pub fn new(_owner: AccountId) ->Self {
            let current_id = u32::default();
            let owner = _owner;
            let balances = Mapping::default();
            let rewarded_tokens = Mapping::default();
            Self {
                current_id,
                owner,
                balances,
                rewarded_tokens,
            }
        }

        /// mint function first checks that only the owner can call the contract,
        /// then it modifies the state of both the auditors_record(if it is a successful audit or unsuccessful one)
        /// and mints the token with auditor as the recipient and all other details like audit_id, completion_time, if it was 
        /// completed with extensions, or in what percent time, the amount, and the ipfs_hash corresponding that audit.
        #[ink(message)]
        pub fn mint(&mut self, _recipient: AccountId, _audit_id: u32, _completion_time: u8, _extensions: u8, _amount: Balance, _ipfs_hash: String, positive_or_not: bool) ->Result<()> {
            let caller = self.env().caller();
            if self.owner != caller {
                return Err(Error::UnAuthorisedCall);
            }
            if positive_or_not {
                let mut _stat = self.balances.get(&_recipient).unwrap_or_default();
                
                _stat.successful_audits = _stat.successful_audits+1;
                self.balances.insert(&_recipient, &_stat);
            }
            else {
                let mut _stat = self.balances.get(_recipient).unwrap_or_default();
                _stat.unsuccessful_audits = _stat.unsuccessful_audits+1;
                self.balances.insert(&_recipient, &_stat);
            }
            let _reward_info = RewardInfo{
                recipient: _recipient,
                audit_id: _audit_id,
                completion_time: _completion_time,
                extensions: _extensions,
                amount: _amount,
                ipfs_hash: _ipfs_hash,
            };
            self.rewarded_tokens.insert(&self.current_id, &_reward_info);
            self.current_id = self.current_id + 1;
            Ok(())
        }

        /// show_auditors_record returns a struct telling how many successful
        /// and unsuccessful audits the auditor has completed.
        #[ink(message)]
        pub fn show_auditors_record(&self, auditor: AccountId) -> Option<Stats> {
            self.balances.get(&auditor)
        }

        /// show_reward_details returns the RewardInfo/the metadata corresponding to the 
        /// reward token entered.
        #[ink(message)]
        pub fn show_reward_details(&self, reward_id: u32) -> Option<RewardInfo> {
            self.rewarded_tokens.get(&reward_id)
        }
    }
}


#[cfg(test)]
mod test_cases {
    use super::*;
    #[cfg(feature = "ink-experimental-engine")]
    use crate::digital_certificate::digital_certificate;

    #[test]
    fn test_assert_owner() {
        //testcase to validate that owner is set in the contract after deployment.
        let accounts = 
        ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
        ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
        ink::env::test::set_callee::<ink::env::DefaultEnvironment>(accounts.bob);
        let contract = rewardtoken::Rewardtoken::new(accounts.alice);
        let contract_owner = contract.owner;
        assert_eq!(contract_owner, accounts.alice);
    }

    #[test]
    fn test_failure_on_non_owner_call(){
        //testcase to validate that only owner can call the contract
        let accounts = 
        ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
        ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
        ink::env::test::set_callee::<ink::env::DefaultEnvironment>(accounts.bob);
        let mut contract = rewardtoken::Rewardtoken::new(accounts.alice);
        ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
        let hash = "asdf";
        let _res = contract.mint(accounts.bob, 1, 100, 0, 100, hash.to_string(), false);
        assert!(_res.is_err());
    }

    #[test]
    fn test_successful_audits_increment(){
        //testcase to validate the successful increment in successful audits variable
        let accounts = 
        ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
        ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
        ink::env::test::set_callee::<ink::env::DefaultEnvironment>(accounts.bob);
        let mut contract = rewardtoken::Rewardtoken::new(accounts.alice);
        let hash = "asdf";
        let _x = contract.mint(accounts.bob, 1, 100, 5, 100, hash.to_string(), true);
        assert_eq!(contract.show_auditors_record(accounts.bob).unwrap().successful_audits, 1);
    }

    #[test]
    fn test_unsuccessful_audits_increment(){
        //testcase to validate the successful increment in unsuccessful audits variable
        let accounts = 
        ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
        ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
        ink::env::test::set_callee::<ink::env::DefaultEnvironment>(accounts.bob);
        let mut contract = rewardtoken::Rewardtoken::new(accounts.alice);
        let hash = "asdf";
        let _x = contract.mint(accounts.bob, 1, 100, 5, 100, hash.to_string(), false);
        assert_eq!(contract.show_auditors_record(accounts.bob).unwrap().unsuccessful_audits, 1);
    }
    
    #[test]
    fn test_successful_entry_in_rewarded_tokens_mapping(){
        //testcase to confirm the modification of values in the mapping of reward details
        let accounts = 
        ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
        ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
        ink::env::test::set_callee::<ink::env::DefaultEnvironment>(accounts.bob);
        let mut contract = rewardtoken::Rewardtoken::new(accounts.alice);
        let hash = "asdf";
        let _x = contract.mint(accounts.bob, 1, 100, 0, 100, hash.to_string(), true);

        assert_eq!(contract.show_reward_details(0).unwrap().amount, 100);
    }
}