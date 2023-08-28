#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod voting {
    use ink::prelude::vec::Vec;
    use ink::storage::Mapping;

    #[derive(scale::Decode, scale::Encode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct Arbiter {
        voter_address: AccountId,
        has_voted: bool,
    }

    #[derive(scale::Decode, scale::Encode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    ///VoteInfo will store crucial information about the voting
    /// like the vector of arbiters, how many arbiters/voters are there, decided deadline, and haircut will update
    pub struct VoteInfo {
        audit_id: u32,
        arbiters: Vec<Arbiter>,
        is_active: bool,
        available_votes: u8,
        decided_deadline: u64,
        decided_haircut: Balance,
    }
    pub type Result<T> = core::result::Result<T, Error>;

    #[derive(scale::Decode, scale::Encode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub enum AuditArbitrationResult {
        NoDiscrepencies,
        MinorDiscrepencies,
        ModerateDiscrepencies,
        Reject,
    }

    #[derive(scale::Decode, scale::Encode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub enum Error {
        UnAuthorisedCall,
        AssessmentFailed,
        ResultAlreadyPublished,
    }

    /// Defines the storage of your contract.
    /// Add new fields to the below struct in order
    /// to add new static storage fields to your contract.
    #[ink(storage)]
    pub struct Voting {
        pub current_vote_id: u32,
        pub escrow_address: AccountId,
        pub vote_id_to_info: Mapping<u32, VoteInfo>,
    }

    impl Voting {
        /// Constructor that initializes the `bool` value to the given `init_value`.
        #[ink(constructor)]
        pub fn new(_escrow_address: AccountId) -> Self {
            let current_vote_id = u32::default();
            let vote_id_to_info = Mapping::default();
            let escrow_address = _escrow_address;

            Self {
                current_vote_id,
                vote_id_to_info,
                escrow_address,
            }
        }

        #[ink(message)]
        pub fn vote(&mut self, _vote_id: u32, _result: AuditArbitrationResult) -> Result<()> {
            let mut _x = self.vote_id_to_info.get(_vote_id).unwrap();
            if !_x.is_active {
                return Err(Error::ResultAlreadyPublished);
            }
            let mut index: usize = 0;
            for account in &_x.arbiters {
                if account.voter_address == self.env().caller() && !account.has_voted {
                    //check if it is the last call/result, if yes, then push the transaction,
                    //if not then just add the result to decided_deadline, decided_haircut.
                    _x.available_votes = _x.available_votes + 1;
                    _x.is_active = false;
                    _x.arbiters[index].has_voted = true;
                    if _x.available_votes == (_x.arbiters.len() as u8) {
                        match _result {
                            AuditArbitrationResult::NoDiscrepencies => {}
                            AuditArbitrationResult::MinorDiscrepencies => {
                                //add 7 days to the deadline extension.
                                _x.decided_deadline =
                                    (_x.decided_deadline + 604800) / (_x.available_votes as u64);
                                _x.decided_haircut =
                                    (_x.decided_haircut + 5) / (_x.available_votes as u128);
                            }
                            AuditArbitrationResult::ModerateDiscrepencies => {
                                //add 15 days to the deadline extension.
                                _x.decided_deadline =
                                    (_x.decided_deadline + 1209600) / (_x.available_votes as u64);
                                _x.decided_haircut =
                                    (_x.decided_haircut + 15) / (_x.available_votes as u128);
                            }
                            AuditArbitrationResult::Reject => {
                                //call the function that rejects the audit report.
                                _x.is_active = false;
                                let result_call = ink::env::call::build_call::<Environment>()
                                    .call(self.escrow_address)
                                    .gas_limit(0)
                                    .transferred_value(0)
                                    .exec_input(
                                        ink::env::call::ExecutionInput::new(
                                            ink::env::call::Selector::new(ink::selector_bytes!(
                                                "assess_audit"
                                            )),
                                        )
                                        .push_arg(&_x.audit_id)
                                        .push_arg(false),
                                    )
                                    .returns::<Result<()>>()
                                    .try_invoke();
                                if matches!(result_call.unwrap().unwrap(), Result::Ok(())) {
                                    _x.available_votes = _x.available_votes + 1;
                                    self.vote_id_to_info.insert(_vote_id, &_x);
                                    return Ok(());
                                } else {
                                    return Err(Error::AssessmentFailed);
                                }
                            }
                        }
                        let result_call = ink::env::call::build_call::<Environment>()
                            .call(self.escrow_address)
                            .gas_limit(0)
                            .transferred_value(0)
                            .exec_input(
                                ink::env::call::ExecutionInput::new(ink::env::call::Selector::new(
                                    ink::selector_bytes!("arbiters_extend_deadline"),
                                ))
                                .push_arg(&_x.audit_id)
                                .push_arg(&_x.decided_deadline)
                                .push_arg(&_x.decided_haircut)
                                .push_arg(5),
                            )
                            .returns::<Result<()>>()
                            .try_invoke();
                        if matches!(result_call.unwrap().unwrap(), Result::Ok(())) {
                            self.vote_id_to_info.insert(_vote_id, &_x);
                            //transfer the money to arbiters as well..
                        }
                    } else {
                        match _result {
                            AuditArbitrationResult::NoDiscrepencies => {}
                            AuditArbitrationResult::MinorDiscrepencies => {
                                //add 7 days to the deadline extension.
                                _x.decided_deadline = _x.decided_deadline + 604800;
                                _x.decided_haircut = _x.decided_haircut + 5;
                            }
                            AuditArbitrationResult::ModerateDiscrepencies => {
                                //add 15 days to the deadline extension.
                                _x.decided_deadline = _x.decided_deadline + 1209600;
                                _x.decided_haircut = _x.decided_haircut + 15;
                            }
                            AuditArbitrationResult::Reject => {
                                _x.is_active = false;
                                let result_call = ink::env::call::build_call::<Environment>()
                                    .call(self.escrow_address)
                                    .gas_limit(0)
                                    .transferred_value(0)
                                    .exec_input(
                                        ink::env::call::ExecutionInput::new(
                                            ink::env::call::Selector::new(ink::selector_bytes!(
                                                "assess_audit"
                                            )),
                                        )
                                        .push_arg(&_x.audit_id)
                                        .push_arg(false),
                                    )
                                    .returns::<Result<()>>()
                                    .try_invoke();
                                if matches!(result_call.unwrap().unwrap(), Result::Ok(())) {
                                    _x.available_votes = _x.available_votes + 1;
                                    self.vote_id_to_info.insert(_vote_id, &_x);
                                    return Ok(());
                                } else {
                                    return Err(Error::AssessmentFailed);
                                }
                            }
                        }
                    }
                }
                index = index + 1;
            }
            return Err(Error::UnAuthorisedCall);
        }
    }
}

//not sure if the index is working properly or not.
