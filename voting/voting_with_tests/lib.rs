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
        pub voter_address: AccountId,
        pub has_voted: bool,
    }

    #[derive(scale::Decode, scale::Encode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    ///VoteInfo will store crucial information about the voting
    /// like the vector of arbiters, how many arbiters/voters are there, decided deadline, and haircut will update
    pub struct VoteInfo {
        pub audit_id: u32,
        pub arbiters: Vec<Arbiter>,
        pub is_active: bool,
        pub available_votes: u8,
        pub decided_deadline: u64,
        pub decided_haircut: Balance,
        pub admin_hit_time: u64,
    }
    pub type Result<T> = core::result::Result<T, Error>;

    #[derive(scale::Decode, scale::Encode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub enum AuditArbitrationResult {
        NoDiscrepancies,
        MinorDiscrepancies,
        ModerateDiscrepancies,
        Reject,
    }

    #[ink(event)]
    pub struct PollCreated {
        id: u32,
        vote_info: VoteInfo,
    }

    #[ink(event)]
    pub struct ArbiterVoted {
        id: u32,
        voter: AccountId,
        vote_type: Option<AuditArbitrationResult>,
    }

    #[ink(event)]
    pub struct FinalVotePushed {
        id: u32,
        pusher: AccountId,
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
        RightsNotActivatedYet,
        TransferFailed,
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

        #[ink(message)]
        pub fn get_current_vote_id(&self) -> u32 {
            self.current_vote_id
        }

        #[ink(message)]
        pub fn know_your_escrow(&self) -> AccountId {
            self.escrow_address
        }

        #[ink(message)]
        pub fn know_your_admin(&self) -> AccountId {
            self.admin
        }

        #[ink(message)]
        pub fn get_poll_info(&self, _id: u32) -> Option<VoteInfo> {
            self.vote_id_to_info.get(&_id)
        }

        ///create_new_poll can only be called by the admin of this contract, and will be called when patron rejects a submitted report
        /// the function takes the audit id of the audit under dispute and a list of arbiters who are going to vote on this proposal
        #[ink(message)]
        pub fn create_new_poll(
            &mut self,
            _audit_id: u32,
            _buffer_for_admin: u64,
            _arbiters: Vec<Arbiter>,
        ) -> Result<()> {
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
                admin_hit_time: _buffer_for_admin,
            };
            self.vote_id_to_info.insert(self.current_vote_id, &_x);
            self.env().emit_event(PollCreated {
                id: self.current_vote_id,
                vote_info: _x,
            });
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
        pub fn vote(
            &mut self,
            _vote_id: u32,
            _result: AuditArbitrationResult,
            _pre_determined_ext_call: bool,
        ) -> Result<()> {
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
                    //case when this is the last vote to be done... submit thing..
                    if _x.available_votes + 1 == _x.arbiters.len() as u8 {
                        match _result {
                            AuditArbitrationResult::NoDiscrepancies => {
                                if _x.decided_deadline > 0 {
                                    _x.decided_deadline =
                                        (_x.decided_deadline) / (_x.available_votes as u64 + 1);
                                    _x.decided_haircut =
                                        (_x.decided_haircut) / (_x.available_votes as Balance + 1);
                                    // let _result_call = ink::env::call::build_call::<Environment>()
                                    //     .call(self.escrow_address)
                                    //     .gas_limit(0)
                                    //     .transferred_value(0)
                                    //     .exec_input(
                                    //         ink::env::call::ExecutionInput::new(
                                    //             ink::env::call::Selector::new(
                                    //                 ink::selector_bytes!(
                                    //                     "arbiters_extend_deadline"
                                    //                 ),
                                    //             ),
                                    //         )
                                    //         .push_arg(&_x.audit_id)
                                    //         .push_arg(&_x.decided_deadline)
                                    //         .push_arg(&_x.decided_haircut)
                                    //         .push_arg(5)
                                    //         .push_arg(_vote_id),
                                    //     )
                                    //     .returns::<Result<()>>()
                                    //     .try_invoke();
                                    // if matches!(_result_call.unwrap().unwrap(), Result::Ok(())) {
                                        if _pre_determined_ext_call{

                                        _x.is_active = false;
                                        _x.available_votes = _x.available_votes + 1;
                                        _x.arbiters[index].has_voted = true;
                                        self.vote_id_to_info.insert(_vote_id, &_x);
                                        self.env().emit_event(ArbiterVoted {
                                            id: _vote_id,
                                            voter: self.env().caller(),
                                            vote_type: Some(_result),
                                        });
                                        self.env().emit_event(FinalVotePushed {
                                            id: _vote_id,
                                            pusher: self.env().caller(),
                                        });
                                        return Ok(());
                                    } else {
                                        return Err(Error::AssessmentFailed);
                                    }
                                } else {
                                    // let _result_call = ink::env::call::build_call::<Environment>()
                                    //     .call(self.escrow_address)
                                    //     .gas_limit(0)
                                    //     .transferred_value(0)
                                    //     .exec_input(
                                    //         ink::env::call::ExecutionInput::new(
                                    //             ink::env::call::Selector::new(
                                    //                 ink::selector_bytes!("assess_audit"),
                                    //             ),
                                    //         )
                                    //         .push_arg(&_x.audit_id)
                                    //         .push_arg(true),
                                    //     )
                                    //     .returns::<Result<()>>()
                                    //     .try_invoke();
                                    // if matches!(_result_call.unwrap().unwrap(), Result::Ok(())) {
                                        if _pre_determined_ext_call{

                                        _x.available_votes = _x.available_votes + 1;
                                        _x.arbiters[index].has_voted = true;
                                        _x.is_active = false;
                                        self.vote_id_to_info.insert(_vote_id, &_x);
                                        return Ok(());
                                    } else {
                                        return Err(Error::AssessmentFailed);
                                    }
                                }
                            }
                            AuditArbitrationResult::MinorDiscrepancies => {
                                //add 7 days to the deadline extension.
                                _x.decided_deadline = (_x.decided_deadline + 604800)
                                    / (_x.available_votes as u64 + 1);
                                _x.decided_haircut =
                                    (_x.decided_haircut + 5) / (_x.available_votes as Balance + 1);
                                // let _result_call = ink::env::call::build_call::<Environment>()
                                //     .call(self.escrow_address)
                                //     .gas_limit(0)
                                //     .transferred_value(0)
                                //     .exec_input(
                                //         ink::env::call::ExecutionInput::new(
                                //             ink::env::call::Selector::new(ink::selector_bytes!(
                                //                 "arbiters_extend_deadline"
                                //             )),
                                //         )
                                //         .push_arg(&_x.audit_id)
                                //         .push_arg(&_x.decided_deadline)
                                //         .push_arg(&_x.decided_haircut)
                                //         .push_arg(5)
                                //         .push_arg(_vote_id),
                                //     )
                                //     .returns::<Result<()>>()
                                //     .try_invoke();
                                // if matches!(_result_call.unwrap().unwrap(), Result::Ok(())) {
                                    if _pre_determined_ext_call{

                                    _x.available_votes = _x.available_votes + 1;
                                    _x.arbiters[index].has_voted = true;
                                    _x.is_active = false;
                                    self.vote_id_to_info.insert(_vote_id, &_x);
                                    self.env().emit_event(ArbiterVoted {
                                        id: _vote_id,
                                        voter: self.env().caller(),
                                        vote_type: Some(_result),
                                    });
                                    self.env().emit_event(FinalVotePushed {
                                        id: _vote_id,
                                        pusher: self.env().caller(),
                                    });
                                    return Ok(());
                                } else {
                                    return Err(Error::AssessmentFailed);
                                }
                            }
                            AuditArbitrationResult::ModerateDiscrepancies => {
                                //add 15 days to the deadline extension.
                                _x.decided_deadline = (_x.decided_deadline + 1209600)
                                    / (_x.available_votes as u64 + 1);
                                _x.decided_haircut =
                                    (_x.decided_haircut + 15) / (_x.available_votes as Balance + 1);
                                // let _result_call = ink::env::call::build_call::<Environment>()
                                //     .call(self.escrow_address)
                                //     .gas_limit(0)
                                //     .transferred_value(0)
                                //     .exec_input(
                                //         ink::env::call::ExecutionInput::new(
                                //             ink::env::call::Selector::new(ink::selector_bytes!(
                                //                 "arbiters_extend_deadline"
                                //             )),
                                //         )
                                //         .push_arg(&_x.audit_id)
                                //         .push_arg(&_x.decided_deadline)
                                //         .push_arg(&_x.decided_haircut)
                                //         .push_arg(5)
                                //         .push_arg(_vote_id),
                                //     )
                                //     .returns::<Result<()>>()
                                //     .try_invoke();
                                // if matches!(_result_call.unwrap().unwrap(), Result::Ok(())) {
                                    if _pre_determined_ext_call{

                                    _x.available_votes = _x.available_votes + 1;
                                    _x.arbiters[index].has_voted = true;
                                    _x.is_active = false;
                                    self.vote_id_to_info.insert(_vote_id, &_x);
                                    self.env().emit_event(ArbiterVoted {
                                        id: _vote_id,
                                        voter: self.env().caller(),
                                        vote_type: Some(_result),
                                    });
                                    self.env().emit_event(FinalVotePushed {
                                        id: _vote_id,
                                        pusher: self.env().caller(),
                                    });
                                    return Ok(());
                                } else {
                                    return Err(Error::AssessmentFailed);
                                }
                            }
                            AuditArbitrationResult::Reject => {
                                //call the function that rejects the audit report.
                                // let result_call = ink::env::call::build_call::<Environment>()
                                //     .call(self.escrow_address)
                                //     .gas_limit(0)
                                //     .transferred_value(0)
                                //     .exec_input(
                                //         ink::env::call::ExecutionInput::new(
                                //             ink::env::call::Selector::new(ink::selector_bytes!(
                                //                 "assess_audit"
                                //             )),
                                //         )
                                //         .push_arg(&_x.audit_id)
                                //         .push_arg(false),
                                //     )
                                //     .returns::<Result<()>>()
                                //     .try_invoke();
                                // if matches!(result_call.unwrap().unwrap(), Result::Ok(())) {
                                if _pre_determined_ext_call {
                                    _x.available_votes = _x.available_votes + 1;
                                    _x.arbiters[index].has_voted = true;
                                    _x.is_active = false;
                                    self.vote_id_to_info.insert(_vote_id, &_x);
                                    self.env().emit_event(ArbiterVoted {
                                        id: _vote_id,
                                        voter: self.env().caller(),
                                        vote_type: Some(_result),
                                    });
                                    self.env().emit_event(FinalVotePushed {
                                        id: _vote_id,
                                        pusher: self.env().caller(),
                                    });
                                    return Ok(());
                                } else {
                                    return Err(Error::AssessmentFailed);
                                }
                            }
                        }
                    } else {
                        match _result {
                            AuditArbitrationResult::NoDiscrepancies => {
                                _x.available_votes = _x.available_votes + 1;
                                _x.arbiters[index].has_voted = true;
                                self.vote_id_to_info.insert(_vote_id, &_x);
                                self.env().emit_event(ArbiterVoted {
                                    id: _vote_id,
                                    voter: self.env().caller(),
                                    vote_type: Some(_result),
                                });
                                return Ok(());
                            }
                            AuditArbitrationResult::MinorDiscrepancies => {
                                _x.available_votes = _x.available_votes + 1;
                                _x.arbiters[index].has_voted = true;
                                //add 7 days to the deadline extension.
                                _x.decided_deadline = _x.decided_deadline + 604800;
                                _x.decided_haircut = _x.decided_haircut + 5;
                                self.vote_id_to_info.insert(_vote_id, &_x);
                                self.env().emit_event(ArbiterVoted {
                                    id: _vote_id,
                                    voter: self.env().caller(),
                                    vote_type: Some(_result),
                                });
                                return Ok(());
                            }
                            AuditArbitrationResult::ModerateDiscrepancies => {
                                _x.available_votes = _x.available_votes + 1;
                                _x.arbiters[index].has_voted = true;
                                //add 15 days to the deadline extension.
                                _x.decided_deadline = _x.decided_deadline + 1209600;
                                _x.decided_haircut = _x.decided_haircut + 15;
                                self.vote_id_to_info.insert(_vote_id, &_x);
                                self.env().emit_event(ArbiterVoted {
                                    id: _vote_id,
                                    voter: self.env().caller(),
                                    vote_type: Some(_result),
                                });
                                return Ok(());
                            }
                            AuditArbitrationResult::Reject => {
                                // let result_call = ink::env::call::build_call::<Environment>()
                                //     .call(self.escrow_address)
                                //     .gas_limit(0)
                                //     .transferred_value(0)
                                //     .exec_input(
                                //         ink::env::call::ExecutionInput::new(
                                //             ink::env::call::Selector::new(ink::selector_bytes!(
                                //                 "assess_audit"
                                //             )),
                                //         )
                                //         .push_arg(&_x.audit_id)
                                //         .push_arg(false),
                                //     )
                                //     .returns::<Result<()>>()
                                //     .try_invoke();
                                // if matches!(result_call.unwrap().unwrap(), Result::Ok(())) {
                                    if _pre_determined_ext_call{

                                    _x.available_votes = _x.available_votes + 1;
                                    _x.arbiters[index].has_voted = true;
                                    _x.is_active = false;
                                    self.vote_id_to_info.insert(_vote_id, &_x);
                                    self.env().emit_event(ArbiterVoted {
                                        id: _vote_id,
                                        voter: self.env().caller(),
                                        vote_type: Some(_result),
                                    });
                                    self.env().emit_event(FinalVotePushed {
                                        id: _vote_id,
                                        pusher: self.env().caller(),
                                    });
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
        pub fn force_vote(&mut self, _vote_id: u32, _pre_determined_ext_call: bool) -> Result<()> {
            if self.env().caller() != self.admin {
                return Err(Error::UnAuthorisedCall);
            }
            if self.vote_id_to_info.get(_vote_id).unwrap().admin_hit_time
                > self.env().block_timestamp()
            {
                return Err(Error::RightsNotActivatedYet);
            }
            let mut _x = self.vote_id_to_info.get(_vote_id).unwrap();

            if !_x.is_active {
                return Err(Error::ResultAlreadyPublished);
            }
            if _x.decided_deadline > 0 {
                // let _result_call = ink::env::call::build_call::<Environment>()
                //     .call(self.escrow_address)
                //     .gas_limit(0)
                //     .transferred_value(0)
                //     .exec_input(
                //         ink::env::call::ExecutionInput::new(ink::env::call::Selector::new(
                //             ink::selector_bytes!("arbiters_extend_deadline"),
                //         ))
                //         .push_arg(&_x.audit_id)
                //         .push_arg(&_x.decided_deadline)
                //         .push_arg(&_x.decided_haircut)
                //         .push_arg(5)
                //         .push_arg(_vote_id),
                //     )
                //     .returns::<Result<()>>()
                //     .try_invoke();
                // if matches!(_result_call.unwrap().unwrap(), Result::Ok(())) {
                if _pre_determined_ext_call {
                    _x.is_active = false;
                    _x.decided_deadline = (_x.decided_deadline) / (_x.available_votes as u64);
                    _x.decided_haircut = (_x.decided_haircut) / (_x.available_votes as Balance);
                    self.vote_id_to_info.insert(_vote_id, &_x);
                    self.env().emit_event(FinalVotePushed {
                        id: _vote_id,
                        pusher: self.env().caller(),
                    });
                    return Ok(());
                } else {
                    return Err(Error::AssessmentFailed);
                }
            } else {
                // let _result_call = ink::env::call::build_call::<Environment>()
                //     .call(self.escrow_address)
                //     .gas_limit(0)
                //     .transferred_value(0)
                //     .exec_input(
                //         ink::env::call::ExecutionInput::new(ink::env::call::Selector::new(
                //             ink::selector_bytes!("assess_audit"),
                //         ))
                //         .push_arg(&_x.audit_id)
                //         .push_arg(true),
                //     )
                //     .returns::<Result<()>>()
                //     .try_invoke();
                // if matches!(_result_call.unwrap().unwrap(), Result::Ok(())) {
                if _pre_determined_ext_call {
                    _x.is_active = false;
                    _x.decided_deadline = (_x.decided_deadline) / (_x.available_votes as u64);
                    _x.decided_haircut = (_x.decided_haircut) / (_x.available_votes as Balance);
                    self.vote_id_to_info.insert(_vote_id, &_x);
                    return Ok(());
                } else {
                    return Err(Error::AssessmentFailed);
                }
            }
        }

        #[ink(message)]
        pub fn flush_out_tokens(
            &mut self,
            _token_address: AccountId,
            _value: Balance, _pre_determined_ext_call: bool
        ) -> Result<()> {
            // let _result_call = ink::env::call::build_call::<Environment>()
            //     .call(_token_address)
            //     .gas_limit(0)
            //     .transferred_value(0)
            //     .exec_input(
            //         ink::env::call::ExecutionInput::new(ink::env::call::Selector::new(
            //             ink::selector_bytes!("transfer"),
            //         ))
            //         .push_arg(&self.admin)
            //         .push_arg(_value),
            //     )
            //     .returns::<Result<()>>()
            //     .try_invoke();
            // if matches!(_result_call.unwrap().unwrap(), Result::Ok(())) {
            if _pre_determined_ext_call {
                return Ok(());
            } else {
                return Err(Error::TransferFailed);
            }
        }
    }
}

#[cfg(test)]
mod test_cases {
    use super::*;
    #[cfg(feature = "ink-experimental-engine")]
    use crate::digital_certificate::digital_certificate;

    #[test]
    fn test_1_successful_creation_of_vote_poll() {
        //testcase to validate that owner is set in the contract after deployment.
        let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
        ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
        ink::env::test::set_callee::<ink::env::DefaultEnvironment>(accounts.bob);
        let mut contract = voting::Voting::new(accounts.charlie, accounts.alice);
        let audit_id: u32 = 1;
        let buffer_for_admin: u64 = 100000000000;
        let mut arbiters: Vec<voting::Arbiter> = Vec::new();
        let voter1 = voting::Arbiter {
            voter_address: accounts.alice,
            has_voted: false,
        };
        let voter2 = voting::Arbiter {
            voter_address: accounts.bob,
            has_voted: false,
        };
        let voter3 = voting::Arbiter {
            voter_address: accounts.charlie,
            has_voted: false,
        };
        arbiters.push(voter1);
        arbiters.push(voter2);
        arbiters.push(voter3);

        let _x = contract.create_new_poll(audit_id, buffer_for_admin, arbiters);
        let ans = contract.get_poll_info(0);
        assert!(ans.unwrap().is_active);
    }
    #[test]
    fn test_2_successful_increment_of_poll_no() {
        let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
        ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
        ink::env::test::set_callee::<ink::env::DefaultEnvironment>(accounts.bob);
        let mut contract = voting::Voting::new(accounts.charlie, accounts.alice);
        let audit_id: u32 = 1;
        let buffer_for_admin: u64 = 100000000000;
        let mut arbiters: Vec<voting::Arbiter> = Vec::new();
        let voter1 = voting::Arbiter {
            voter_address: accounts.alice,
            has_voted: false,
        };
        let voter2 = voting::Arbiter {
            voter_address: accounts.bob,
            has_voted: false,
        };
        let voter3 = voting::Arbiter {
            voter_address: accounts.charlie,
            has_voted: false,
        };
        arbiters.push(voter1);
        arbiters.push(voter2);
        arbiters.push(voter3);

        let _x = contract.create_new_poll(audit_id, buffer_for_admin, arbiters);
        let ans = contract.get_current_vote_id();
        assert_eq!(ans, 1);
    }
    #[test]
    fn test_3_successful_know_your_escrow() {
        let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
        ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
        ink::env::test::set_callee::<ink::env::DefaultEnvironment>(accounts.bob);
        let contract = voting::Voting::new(accounts.charlie, accounts.alice);
        let ans = contract.know_your_escrow();
        assert_eq!(ans, accounts.charlie);
    }
    #[test]
    fn test_4_successful_know_your_admin() {
        let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
        ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
        ink::env::test::set_callee::<ink::env::DefaultEnvironment>(accounts.bob);
        let contract = voting::Voting::new(accounts.charlie, accounts.alice);
        let ans = contract.know_your_admin();
        assert_eq!(ans, accounts.alice);
    }
    #[test]
    fn test_5_failure_on_non_admin_creating_poll() {
        let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
        ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
        ink::env::test::set_callee::<ink::env::DefaultEnvironment>(accounts.bob);
        let mut contract = voting::Voting::new(accounts.charlie, accounts.alice);
        let audit_id: u32 = 1;
        let buffer_for_admin: u64 = 100000000000;
        let mut arbiters: Vec<voting::Arbiter> = Vec::new();
        let voter1 = voting::Arbiter {
            voter_address: accounts.alice,
            has_voted: false,
        };
        let voter2 = voting::Arbiter {
            voter_address: accounts.bob,
            has_voted: false,
        };
        let voter3 = voting::Arbiter {
            voter_address: accounts.charlie,
            has_voted: false,
        };
        arbiters.push(voter1);
        arbiters.push(voter2);
        arbiters.push(voter3);
        ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
        let _x = contract.create_new_poll(audit_id, buffer_for_admin, arbiters);
        assert!(matches!(_x, Err(voting::Error::UnAuthorisedCall)));
    }
    #[test]
    fn test_6_successful_first_or_mid_voters_voting() {
        let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
        ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
        ink::env::test::set_callee::<ink::env::DefaultEnvironment>(accounts.bob);
        let mut contract = voting::Voting::new(accounts.charlie, accounts.alice);
        let audit_id: u32 = 1;
        let buffer_for_admin: u64 = 100000000000;
        let mut arbiters: Vec<voting::Arbiter> = Vec::new();
        let voter1 = voting::Arbiter {
            voter_address: accounts.alice,
            has_voted: false,
        };
        let voter2 = voting::Arbiter {
            voter_address: accounts.bob,
            has_voted: false,
        };
        let voter3 = voting::Arbiter {
            voter_address: accounts.charlie,
            has_voted: false,
        };
        arbiters.push(voter1);
        arbiters.push(voter2);
        arbiters.push(voter3);
        let _x = contract.create_new_poll(audit_id, buffer_for_admin, arbiters);
        ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
        let _y = contract.vote(0, voting::AuditArbitrationResult::NoDiscrepancies, true);
        let _z = contract.get_poll_info(0);
        assert_eq!(_z.unwrap().available_votes, 1);
    }
    #[test]
    fn test_7_successful_final_vote() {
        let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
        ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
        ink::env::test::set_callee::<ink::env::DefaultEnvironment>(accounts.bob);
        let mut contract = voting::Voting::new(accounts.charlie, accounts.alice);
        let audit_id: u32 = 1;
        let buffer_for_admin: u64 = 100000000000;
        let mut arbiters: Vec<voting::Arbiter> = Vec::new();
        // let voter1 = voting::Arbiter{voter_address: accounts.alice, has_voted: false};
        let voter2 = voting::Arbiter {
            voter_address: accounts.bob,
            has_voted: false,
        };
        let voter3 = voting::Arbiter {
            voter_address: accounts.charlie,
            has_voted: false,
        };
        // arbiters.push(voter1);
        arbiters.push(voter2);
        arbiters.push(voter3);
        let _x = contract.create_new_poll(audit_id, buffer_for_admin, arbiters);
        ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
        let _y = contract.vote(0, voting::AuditArbitrationResult::NoDiscrepancies, true);
        ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.charlie);
        let _y = contract.vote(0, voting::AuditArbitrationResult::NoDiscrepancies, true);
        let _z = contract.get_poll_info(0);
        assert_eq!(_z.unwrap().available_votes, 2);
    }
    #[test]
    fn test_8_failure_when_wrong_escrow_id_provided() {
        let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
        ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
        ink::env::test::set_callee::<ink::env::DefaultEnvironment>(accounts.bob);
        let mut contract = voting::Voting::new(accounts.charlie, accounts.alice);
        let audit_id: u32 = 1;
        let buffer_for_admin: u64 = 100000000000;
        let mut arbiters: Vec<voting::Arbiter> = Vec::new();
        // let voter1 = voting::Arbiter{voter_address: accounts.alice, has_voted: false};
        let voter2 = voting::Arbiter {
            voter_address: accounts.bob,
            has_voted: false,
        };
        let voter3 = voting::Arbiter {
            voter_address: accounts.charlie,
            has_voted: false,
        };
        // arbiters.push(voter1);
        arbiters.push(voter2);
        arbiters.push(voter3);
        let _x = contract.create_new_poll(audit_id, buffer_for_admin, arbiters);
        ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
        let _y = contract.vote(0, voting::AuditArbitrationResult::NoDiscrepancies, true);
        ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.charlie);
        let _y = contract.vote(0, voting::AuditArbitrationResult::NoDiscrepancies, false);
        assert!(matches!(_y, Err(voting::Error::AssessmentFailed)));
        let _z = contract.get_poll_info(0);
        assert_eq!(_z.unwrap().available_votes, 1);
    }
}
