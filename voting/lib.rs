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
        VotingFailed,
    }

    /// Defines the storage of your contract.
    /// Add new fields to the below struct in order
    /// to add new static storage fields to your contract.
    #[ink(storage)]
    pub struct Voting {
        pub current_vote_id: u32,
        pub escrow_address: AccountId,
        pub admin: AccountId,
        pub vote_id_to_info: Mapping<u32, VoteInfo>,
    }

    impl Voting {
        /// Constructor that initializes the `bool` value to the given `init_value`.
        #[ink(constructor)]
        pub fn new(_escrow_address: AccountId, _admin: AccountId) -> Self {
            let current_vote_id = u32::default();
            let vote_id_to_info = Mapping::default();
            let escrow_address = _escrow_address;
            let admin = _admin;

            Self {
                current_vote_id,
                vote_id_to_info,
                escrow_address,
                admin,
            }
        } 

        ///create_new_poll can only be called by the admin of this contract, and will be called when patron rejects a submitted report
        /// the function takes the audit id of the audit under dispute and a list of arbiters who are going to vote on this proposal
        #[ink(message)]
        pub fn create_new_poll(&mut self, _audit_id: u32, _arbiters: Vec<Arbiter>) -> Result<()> {
            if self.env().caller() != self.admin {
                return Err(Error::UnAuthorisedCall);
            }
            let _x = VoteInfo {
                audit_id: _audit_id,
                arbiters: _arbiters,
                is_active: true,
                available_votes: 0,
                decided_deadline: 0,
                decided_haircut: 0,
            };
            self.vote_id_to_info.insert(self.current_vote_id, &_x);
            self.current_vote_id = self.current_vote_id + 1;
            return Ok(());
        }

        /// vote function is the main function of this contract, taking in vote_id and result as input by the arbiters,
        /// it first verifies that the voting is still active, and that the arbiter hasn't already voted.
        /// then it updates the state of this and the other contract according to stage.
        /// so if this is the final vote, it will directly call the other conract, similarly if the arbiter has selected reject,
        /// it will be a rejection without averaging out.
        /// But otherwise it will simply be compounded into decided_deadline and decided_haircut to be averaged out eventually.
        #[ink(message)]
        pub fn vote(&mut self, _vote_id: u32, _result: AuditArbitrationResult) -> Result<()> {
            let mut _x = self.vote_id_to_info.get(_vote_id).unwrap();
            if !_x.is_active {
                return Err(Error::ResultAlreadyPublished);
            }
            let mut index: usize = 0;
            for account in &_x.arbiters {
                if account.voter_address == self.env().caller() {
                    break;
                }
                index = index + 1;
            }
            if index >= _x.arbiters.len() {
                return Err(Error::UnAuthorisedCall);
            } else {
                if _x.arbiters[index].has_voted {
                    return Err(Error::VotingFailed);
                } else {
                    _x.available_votes = _x.available_votes + 1;
                    _x.arbiters[index].has_voted = true;

                    //case when this is the last vote to be done... submit thing..
                    if _x.available_votes == _x.arbiters.len() as u8 {
                        match _result {
                            AuditArbitrationResult::NoDiscrepencies => {
                                if _x.decided_deadline > 0 {
                                    _x.decided_deadline =
                                        (_x.decided_deadline) / (_x.available_votes as u64);
                                    _x.decided_haircut =
                                        (_x.decided_haircut) / (_x.available_votes as u128);
                                    self.vote_id_to_info.insert(_vote_id, &_x);
                                    let _result_call = ink::env::call::build_call::<Environment>()
                                        .call(self.escrow_address)
                                        .gas_limit(0)
                                        .transferred_value(0)
                                        .exec_input(
                                            ink::env::call::ExecutionInput::new(
                                                ink::env::call::Selector::new(
                                                    ink::selector_bytes!(
                                                        "arbiters_extend_deadline"
                                                    ),
                                                ),
                                            )
                                            .push_arg(&_x.audit_id)
                                            .push_arg(&_x.decided_deadline)
                                            .push_arg(&_x.decided_haircut)
                                            .push_arg(5),
                                        )
                                        .returns::<Result<()>>()
                                        .try_invoke();
                                    if matches!(_result_call.unwrap().unwrap(), Result::Ok(())) {
                                        return Ok(());
                                    } else {
                                        return Err(Error::AssessmentFailed);
                                    }
                                } else {
                                    self.vote_id_to_info.insert(_vote_id, &_x);
                                    let _result_call = ink::env::call::build_call::<Environment>()
                                        .call(self.escrow_address)
                                        .gas_limit(0)
                                        .transferred_value(0)
                                        .exec_input(
                                            ink::env::call::ExecutionInput::new(
                                                ink::env::call::Selector::new(
                                                    ink::selector_bytes!("assess_audit"),
                                                ),
                                            )
                                            .push_arg(&_x.audit_id)
                                            .push_arg(true),
                                        )
                                        .returns::<Result<()>>()
                                        .try_invoke();
                                    if matches!(_result_call.unwrap().unwrap(), Result::Ok(())) {
                                        return Ok(());
                                    } else {
                                        return Err(Error::AssessmentFailed);
                                    }
                                }
                            }
                            AuditArbitrationResult::MinorDiscrepencies => {
                                //add 7 days to the deadline extension.
                                _x.decided_deadline =
                                    (_x.decided_deadline + 604800) / (_x.available_votes as u64);
                                _x.decided_haircut =
                                    (_x.decided_haircut + 5) / (_x.available_votes as u128);
                                self.vote_id_to_info.insert(_vote_id, &_x);
                                let _result_call = ink::env::call::build_call::<Environment>()
                                    .call(self.escrow_address)
                                    .gas_limit(0)
                                    .transferred_value(0)
                                    .exec_input(
                                        ink::env::call::ExecutionInput::new(
                                            ink::env::call::Selector::new(ink::selector_bytes!(
                                                "arbiters_extend_deadline"
                                            )),
                                        )
                                        .push_arg(&_x.audit_id)
                                        .push_arg(&_x.decided_deadline)
                                        .push_arg(&_x.decided_haircut)
                                        .push_arg(5),
                                    )
                                    .returns::<Result<()>>()
                                    .try_invoke();
                                if matches!(_result_call.unwrap().unwrap(), Result::Ok(())) {
                                    return Ok(());
                                } else {
                                    return Err(Error::AssessmentFailed);
                                }
                            }
                            AuditArbitrationResult::ModerateDiscrepencies => {
                                //add 15 days to the deadline extension.
                                _x.decided_deadline =
                                    (_x.decided_deadline + 1209600) / (_x.available_votes as u64);
                                _x.decided_haircut =
                                    (_x.decided_haircut + 15) / (_x.available_votes as u128);
                                self.vote_id_to_info.insert(_vote_id, &_x);
                                let _result_call = ink::env::call::build_call::<Environment>()
                                    .call(self.escrow_address)
                                    .gas_limit(0)
                                    .transferred_value(0)
                                    .exec_input(
                                        ink::env::call::ExecutionInput::new(
                                            ink::env::call::Selector::new(ink::selector_bytes!(
                                                "arbiters_extend_deadline"
                                            )),
                                        )
                                        .push_arg(&_x.audit_id)
                                        .push_arg(&_x.decided_deadline)
                                        .push_arg(&_x.decided_haircut)
                                        .push_arg(5),
                                    )
                                    .returns::<Result<()>>()
                                    .try_invoke();
                                if matches!(_result_call.unwrap().unwrap(), Result::Ok(())) {
                                    return Ok(());
                                } else {
                                    return Err(Error::AssessmentFailed);
                                }
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
                    } else {
                        match _result {
                            AuditArbitrationResult::NoDiscrepencies => {
                                self.vote_id_to_info.insert(_vote_id, &_x);
                                return Ok(());
                            }
                            AuditArbitrationResult::MinorDiscrepencies => {
                                //add 7 days to the deadline extension.
                                _x.decided_deadline = _x.decided_deadline + 604800;
                                _x.decided_haircut = _x.decided_haircut + 5;
                                self.vote_id_to_info.insert(_vote_id, &_x);
                                return Ok(());
                            }
                            AuditArbitrationResult::ModerateDiscrepencies => {
                                //add 15 days to the deadline extension.
                                _x.decided_deadline = _x.decided_deadline + 1209600;
                                _x.decided_haircut = _x.decided_haircut + 15;
                                return Ok(());
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
            }
        }

        ///In case when not all arbiters have voted on a particular proposal, the admin has the liberty of forcing the vote by submitting the
        /// current decision, accordingly it will either approve the auditor or extend their deadline.
        #[ink(message)]
        pub fn force_vote(&mut self, _vote_id: u32) -> Result<()> {
            if self.env().caller() != self.admin {
                return Err(Error::UnAuthorisedCall);
            }
            let mut _x = self.vote_id_to_info.get(_vote_id).unwrap();
            if !_x.is_active {
                return Err(Error::ResultAlreadyPublished);
            }
            _x.is_active = false;
            self.vote_id_to_info.insert(_vote_id, &_x);
            if _x.decided_deadline > 0 {
                let _result_call = ink::env::call::build_call::<Environment>()
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
                if matches!(_result_call.unwrap().unwrap(), Result::Ok(())) {
                    return Ok(());
                } else {
                    return Err(Error::AssessmentFailed);
                }
            } else {
                let _result_call = ink::env::call::build_call::<Environment>()
                    .call(self.escrow_address)
                    .gas_limit(0)
                    .transferred_value(0)
                    .exec_input(
                        ink::env::call::ExecutionInput::new(ink::env::call::Selector::new(
                            ink::selector_bytes!("assess_audit"),
                        ))
                        .push_arg(&_x.audit_id)
                        .push_arg(true),
                    )
                    .returns::<Result<()>>()
                    .try_invoke();
                if matches!(_result_call.unwrap().unwrap(), Result::Ok(())) {
                    return Ok(());
                } else {
                    return Err(Error::AssessmentFailed);
                }
            }
        }
    }
}
