#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod escrow {
    use ink::prelude::string::String;
    use ink::storage::Mapping;

    #[derive(scale::Decode, scale::Encode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    // stores the status of the audit, e.g. whether it
    // has just been created, assigned, submitted, is awaiting validation,
    // completed, or expired.
    pub enum AuditStatus {
        AuditCreated,
        AuditAssigned,
        AuditSubmitted,
        AuditAwaitingValidation,
        AuditCompleted,
        AuditExpired,
    }

    #[derive(scale::Decode, scale::Encode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    // The payment info struct stores all the
    // important information related to a particular audit. It stores the
    // patron’s, auditor’s, and arbiter provider’s account ID. It also stores
    // the value locked, deadline, start time, and the current status of the
    // audit.
    pub struct PaymentInfo {
        pub patron: AccountId,
        pub auditor: AccountId,
        pub value: Balance,
        pub arbiterprovider: AccountId,
        pub deadline: u64,
        pub starttime: u64,
        pub currentstatus: AuditStatus,
    }

    #[derive(scale::Decode, scale::Encode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub enum Error {
        UnAuthorisedCall,
        InsufficientBalance,
        InsufficientBalanceTest,
        InvalidArgument,
        SubmissionFailed,
        TransferFromContractFailed,
        ArbitersExtendDeadlineConditionsNotMet,
    }

    #[derive(scale::Decode, scale::Encode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    // The structure stores the haircut
    // percentage the auditor is willing to take on the value, and new
    // deadline that s/he is proposing
    // #[ink::storage_item]
    pub struct IncreaseRequest {
        haircut_percentage: Balance,
        newdeadline: u64,
    }
    // emitted when an audit ID is assigned to an
    // auditor.
    #[ink(event)]
    pub struct AuditIdAssigned {
        id: Option<u32>,
        payment_info: Option<PaymentInfo>,
    }
    //emitted when an audit is created
    #[ink(event)]
    pub struct AuditCreated {
        id: u32,
        payment_info: Option<PaymentInfo>,
        salt: u64,
    }
    // emitted when the payment_info of for an audit
    // ID is updated
    #[ink(event)]
    pub struct AuditInfoUpdated {
        id: Option<u32>,
        payment_info: Option<PaymentInfo>,
        updated_by: Option<AccountId>,
    }

    // emitted when an auditor requests
    // additional time, mainly to inform the patron and the backend
    #[ink(event)]
    pub struct DeadlineExtendRequest {
        id: u32,
        newtime: u64,
        haircut: Balance,
    }

    // emitted when audit is submitted, so that the ipfs
    // files can be fetched via the backend and the patron/arbiter
    // provider
    #[ink(event)]
    pub struct AuditSubmitted {
        id: u32,
        ipfs_hash: String,
    }

    //emitted when patron is dissatisfied with audit
    #[ink(event)]
    pub struct AuditRequestsArbitration {
        id: u32,
    }

    // When tokens are locked into the escrow contract
    // for an auditID
    #[ink(event)]
    pub struct TokenIncoming {
        id: u32,
    }

    // emitted when tokens are released from the escrow, maybe
    // as haircut, or completion value, or after the expiration of the audit
    #[ink(event)]
    pub struct TokenOutgoing {
        id: u32,
        receiver: AccountId,
        amount: Balance,
    }

    // emits and informs the retrieval of the audit ID
    #[ink(event)]
    pub struct AuditIdRetrieved {
        id: u32,
    }

    #[ink(storage)]
    pub struct Escrow {
        current_audit_id: u32,
        stablecoin_address: AccountId,
        pub audit_id_to_payment_info: Mapping<u32, PaymentInfo>,
        pub audit_id_to_time_increase_request: ink::storage::Mapping<u32, IncreaseRequest>,
        pub audit_id_to_ipfs_hash: ink::storage::Mapping<u32, String>,
    }

    pub type Result<T> = core::result::Result<T, Error>;

    impl Escrow {
        #[ink(constructor)]
        pub fn new(_stablecoin_address: AccountId) -> Self {
            let current_audit_id = u32::default();
            let stablecoin_address = _stablecoin_address;
            // let current_request_id = u32::default();
            let audit_id_to_payment_info = Mapping::default();
            let audit_id_to_time_increase_request = Mapping::default();
            let audit_id_to_ipfs_hash = Mapping::default();
            Self {
                current_audit_id,
                stablecoin_address,
                audit_id_to_payment_info,
                audit_id_to_time_increase_request,
                audit_id_to_ipfs_hash,
            }
        }

        #[ink(message)]
        pub fn get_current_audit_id(&self) -> u32 {
            self.current_audit_id
        }

        #[ink(message)]
        pub fn know_your_stablecoin(&self) -> AccountId {
            self.stablecoin_address
        }

        #[ink(message)]
        pub fn get_paymentinfo(&self, id: u32) -> Option<PaymentInfo> {
            self.audit_id_to_payment_info.get(&id)
        }

        #[ink(message)]
        pub fn query_timeincreaserequest(&self, id: u32) -> Option<IncreaseRequest> {
            self.audit_id_to_time_increase_request.get(&id)
        }


        //create new payment function is to be called by the patron by depositing the said sum in the contract, and choosing a rough deadline and balance for the audit job.
        //argument: value (Balance) that will be locked in the escrow
        //argument: arbiter_provider (AccountId) the service that will provide with arbiters
        //deadline: amount of time from the assigning of the auditor for successful audit
        //the function will create a new payment, lock in the value amount of payment tokens, and
        // assign it to current_audit_id, increasing the audit_id afterwards
        //and emitting the event for AuditInfoUpdated.
        #[ink(message)]
        pub fn create_new_payment(
            &mut self,
            _value: Balance,
            _arbiter_provider: AccountId,
            _deadline: u64,
            _salt: u64,
            //this deadline is deadline that will be added to current time once the audit is assigned to an auditor.
        ) -> Result<()> {
            let _now = self.env().block_timestamp();
            let x = PaymentInfo {
                value: _value,
                starttime: _now,
                auditor: self.env().caller(),
                arbiterprovider: _arbiter_provider,
                patron: self.env().caller(),
                deadline: _deadline,
                currentstatus: AuditStatus::AuditCreated,
            };
            assert_ne!(_value, 0);
            let xyz = ink::env::call::build_call::<Environment>()
                .call(self.stablecoin_address)
                .gas_limit(0)
                .exec_input(
                    ink::env::call::ExecutionInput::new(ink::env::call::Selector::new(
                        ink::selector_bytes!("transfer_from"),
                    ))
                    .push_arg(self.env().caller())
                    .push_arg(self.env().account_id())
                    .push_arg(_value),
                )
                .returns::<Result<()>>()
                .try_invoke();

            if matches!(xyz.unwrap().unwrap(), Result::Ok(())) {
                self.env().emit_event(TokenIncoming {
                    id: self.current_audit_id,
                });
                self.audit_id_to_payment_info
                    .insert(&self.current_audit_id, &x);
                self.env().emit_event(AuditCreated {
                    id: self.current_audit_id,
                    payment_info: Some(x),
                    salt: _salt,
                });
                self.current_audit_id = self.current_audit_id + 1;
                return Ok(());
            } else {
                return Err(Error::InsufficientBalanceTest);
            }
        }

        
        //argument: id(u32) to access the audit ID.
        //argument: _auditor(AccountId) the id of auditor being assigned for the audit.
        //argument: _new_value (Balance) the new value if off-chain patron and auditor decided to have a new value
        //argument: _new_deadline(u64) new deadline decided by patron and auditor off-chain.
        // the function verifies if the caller is patron of the audit ID in question,
        //and then assigns the auditor, resets the start time, and marks a deadline,
        //emitting the event AuditIdAssigned
        // if however the new deadline or new value are different than the original ones, it will be reflected
        // on the audit info, if more value is needed it would require further pre-approved amount, if less, it
        // will return the subtracted money back to the patron.
        #[ink(message)]
        pub fn assign_audit(
            &mut self,
            id: u32,
            _auditor: AccountId,
            _new_value: Balance,
            _new_deadline: u64,
        ) -> Result<()> {
            let mut payment_info = self.audit_id_to_payment_info.get(id).unwrap();
            let _now = self.env().block_timestamp();
            if payment_info.patron == self.env().caller()
                && matches!(payment_info.currentstatus, AuditStatus::AuditCreated)
            {
                if payment_info.value == _new_value && payment_info.deadline == _new_deadline {
                    payment_info.auditor = _auditor;
                    payment_info.starttime = _now;
                    payment_info.deadline = payment_info.deadline + _now;
                    payment_info.currentstatus = AuditStatus::AuditAssigned;
                    self.audit_id_to_payment_info.insert(id, &payment_info);
                    self.env().emit_event(AuditIdAssigned {
                        id: Some(self.current_audit_id),
                        payment_info: Some(payment_info),
                    });
                    return Ok(());
                } else if payment_info.value == _new_value {
                    payment_info.auditor = _auditor;
                    payment_info.starttime = _now;
                    payment_info.deadline = _new_deadline + _now;
                    payment_info.currentstatus = AuditStatus::AuditAssigned;
                    self.audit_id_to_payment_info.insert(id, &payment_info);
                    self.env().emit_event(AuditIdAssigned {
                        id: Some(self.current_audit_id),
                        payment_info: Some(payment_info),
                    });
                    return Ok(());
                } else {
                    if _new_value > payment_info.value {
                        let xyz = ink::env::call::build_call::<Environment>()
                            .call(self.stablecoin_address)
                            .gas_limit(0)
                            .transferred_value(0)
                            .exec_input(
                                ink::env::call::ExecutionInput::new(ink::env::call::Selector::new(
                                    ink::selector_bytes!("transfer_from"),
                                ))
                                .push_arg(self.env().caller())
                                .push_arg(self.env().account_id())
                                .push_arg(_new_value - payment_info.value),
                            )
                            .returns::<Result<()>>()
                            .try_invoke();
                        if matches!(xyz.unwrap().unwrap(), Result::Ok(())) {
                            payment_info.auditor = _auditor;
                            payment_info.starttime = _now;
                            payment_info.value = _new_value;
                            payment_info.deadline = _new_deadline + _now;
                            payment_info.currentstatus = AuditStatus::AuditAssigned;
                            self.audit_id_to_payment_info.insert(id, &payment_info);
                            self.env().emit_event(AuditIdAssigned {
                                id: Some(self.current_audit_id),
                                payment_info: Some(payment_info),
                            });
                            return Ok(());
                        }
                        return Err(Error::InsufficientBalance);
                    } else {
                        let xyz = ink::env::call::build_call::<Environment>()
                            .call(self.stablecoin_address)
                            .gas_limit(0)
                            .transferred_value(0)
                            .exec_input(
                                ink::env::call::ExecutionInput::new(ink::env::call::Selector::new(
                                    ink::selector_bytes!("transfer"),
                                ))
                                .push_arg(self.env().caller())
                                .push_arg(payment_info.value - _new_value),
                            )
                            .returns::<Result<()>>()
                            .try_invoke();
                        if matches!(xyz.unwrap().unwrap(), Result::Ok(())) {
                            return Ok(());
                        }
                        return Err(Error::TransferFromContractFailed);
                    }
                }
            } else {
                return Err(Error::UnAuthorisedCall);
            }
        }

        //argument: _id (u32) audit Id
        //argument: _time (u64) the new deadline
        //argument: haircut_percentage(Balance) the part of value that will be sent back to the patron for delay
        // the function verifies that the auditor is calling the function, then the request is made,
        //mapping of IncreaseRequest updated, and event is emitted for DeadlineExtendRequest
        #[ink(message)]
        pub fn request_additional_time(
            &mut self,
            _id: u32,
            _time: u64,
            _haircut_percentage: Balance,
        ) -> Result<()> {
            if self.get_paymentinfo(_id).unwrap().auditor == self.env().caller() {
                let x = IncreaseRequest {
                    haircut_percentage: _haircut_percentage,
                    newdeadline: _time,
                };
                self.audit_id_to_time_increase_request.insert(_id, &x);
                self.env().emit_event(DeadlineExtendRequest {
                    id: _id,
                    newtime: _time,
                    haircut: _haircut_percentage,
                });
                return Ok(());
            }
            return Err(Error::UnAuthorisedCall);
        }

        //argument: _id(u32) audit Id for which the additional time will be approved
        // the function verifies that only patron is calling it, and haircut is lesser than 100%,
        // the function assumes the consent for approving the time, transfers the haircut percentage
        //to the patron's address, and changes the time in payment_info along with the new amount
        //  events are emitted for tokenOutgoing and AuditInfoUpdated.
        #[ink(message)]
        pub fn approve_additional_time(&mut self, _id: u32) -> Result<()> {
            if self.get_paymentinfo(_id).unwrap().patron == self.env().caller() {
                let haircut = self
                    .query_timeincreaserequest(_id)
                    .unwrap()
                    .haircut_percentage;
                if haircut < 100 {
                    let new_deadline = self.query_timeincreaserequest(_id).unwrap().newdeadline;

                    let mut payment_info = self.audit_id_to_payment_info.get(_id).unwrap();
                    let value0 = payment_info.value * haircut / 100;
                    let xyz = ink::env::call::build_call::<Environment>()
                        .call(self.stablecoin_address)
                        .gas_limit(0)
                        .transferred_value(0)
                        .exec_input(
                            ink::env::call::ExecutionInput::new(ink::env::call::Selector::new(
                                ink::selector_bytes!("transfer"),
                            ))
                            .push_arg(payment_info.patron)
                            .push_arg(value0), // .push_arg(&[0x10u8; 32]),
                        )
                        .returns::<Result<()>>()
                        .try_invoke();
                    if matches!(xyz.unwrap().unwrap(), Result::Ok(())) {
                        self.env().emit_event(TokenOutgoing {
                            id: _id,
                            receiver: payment_info.patron,
                            amount: value0,
                        });
                        payment_info.value = payment_info.value * (100 - haircut) / 100;
                        payment_info.deadline = new_deadline;
                        self.audit_id_to_payment_info.insert(_id, &payment_info);

                        self.env().emit_event(AuditInfoUpdated {
                            id: Some(_id),
                            payment_info: Some(self.audit_id_to_payment_info.get(_id).unwrap()),
                            updated_by: Some(self.get_paymentinfo(_id).unwrap().patron),
                        });
                        return Ok(());
                    }
                    return Err(Error::TransferFromContractFailed);
                }
                return Err(Error::InvalidArgument);
            }
            Err(Error::UnAuthorisedCall)
        }

        //argument: id (u32) The audit Id for which ipfs hash will be submitted,
        //argument: ipfs_hash (String) the hash for the audit reports
        // the function changes the state of payment_info's audit status, and inserts the ipfs hash for the corresponding id.
        //event is emitted for AuditSubmitted.
        #[ink(message)]
        pub fn mark_submitted(&mut self, _id: u32, _ipfs_hash: String) -> Result<()> {
            let mut payment_info = self.audit_id_to_payment_info.get(_id).unwrap();
            if payment_info.auditor == self.env().caller()
                && matches!(payment_info.currentstatus, AuditStatus::AuditAssigned)
                && payment_info.deadline > self.env().block_timestamp()
            {
                self.audit_id_to_ipfs_hash.insert(_id, &_ipfs_hash);
                self.env().emit_event(AuditSubmitted {
                    id: _id,
                    ipfs_hash: _ipfs_hash,
                });
                payment_info.currentstatus = AuditStatus::AuditSubmitted;
                self.audit_id_to_payment_info.insert(_id, &payment_info);
                return Ok(());
            }
            //this error could incur due to multiple reasons including
            // wrong caller, wrong state, or crossed deadline
            Err(Error::SubmissionFailed)
        }

        //argument: id(u32) the audit id for assessment
        //argument: answer (bool) if the caller is satisfied with audit report or not.
        //broken down into three cases,
        //C1: when patron calls,
        //C2: when arbiterprovider calls,
        //C3: when anything else happens
        //C1 has two parts further, patron can only assess the audit if it is in submitted state, if patron
        //says yes, then transfers happen, if no, then state is changed to awaitingValidation.
        //C2 could have had two parts, and state should be awaitingValidation
        // if true, transfer happens, if false, function sets the audit status to expired, and returns the tokens to patron.
        //only then will the transfers happen.
        #[ink(message)]
        pub fn assess_audit(&mut self, id: u32, answer: bool) -> Result<()> {
            let mut payment_info = self.audit_id_to_payment_info.get(id).unwrap();
            //C1
            if self.env().caller() == payment_info.patron
                && matches!(payment_info.currentstatus, AuditStatus::AuditSubmitted)
            {
                if answer {
                    let xyz = ink::env::call::build_call::<Environment>()
                        .call(self.stablecoin_address)
                        .gas_limit(0)
                        .transferred_value(0)
                        .exec_input(
                            ink::env::call::ExecutionInput::new(ink::env::call::Selector::new(
                                ink::selector_bytes!("transfer"),
                            ))
                            .push_arg(payment_info.auditor)
                            .push_arg(payment_info.value * 98 / 100), // .push_arg(&[0x10u8; 32]),
                        )
                        .returns::<Result<()>>()
                        .try_invoke();
                    let zyx = ink::env::call::build_call::<Environment>()
                        .call(self.stablecoin_address)
                        .gas_limit(0)
                        .transferred_value(0)
                        .exec_input(
                            ink::env::call::ExecutionInput::new(ink::env::call::Selector::new(
                                ink::selector_bytes!("transfer"),
                            ))
                            .push_arg(payment_info.arbiterprovider)
                            .push_arg(payment_info.value * 2 / 100), // .push_arg(&[0x10u8; 32]),
                        )
                        .returns::<Result<()>>()
                        .try_invoke();
                    if matches!(xyz.unwrap().unwrap(), Result::Ok(()))
                        && matches!(zyx.unwrap().unwrap(), Result::Ok(()))
                    {
                        self.env().emit_event(TokenOutgoing {
                            id: id,
                            receiver: payment_info.auditor,
                            amount: payment_info.value * 98 / 100,
                        });

                        self.env().emit_event(TokenOutgoing {
                            id: id,
                            receiver: payment_info.arbiterprovider,
                            amount: payment_info.value * 2 / 100,
                        });
                        payment_info.currentstatus = AuditStatus::AuditCompleted;
                        self.audit_id_to_payment_info.insert(id, &payment_info);
                        return Ok(());
                    }
                    return Err(Error::TransferFromContractFailed);
                } else {
                    payment_info.currentstatus = AuditStatus::AuditAwaitingValidation;
                    self.audit_id_to_payment_info.insert(id, &payment_info);
                    self.env().emit_event(AuditRequestsArbitration {
                        id: self.current_audit_id,
                    });
                    return Ok(());
                }
            }
            //C2
            else if self.env().caller() == payment_info.arbiterprovider
                && matches!(
                    payment_info.currentstatus,
                    AuditStatus::AuditAwaitingValidation
                )
            {
                if answer {
                    let xyz = ink::env::call::build_call::<Environment>()
                        .call(self.stablecoin_address)
                        .gas_limit(0)
                        .transferred_value(0)
                        .exec_input(
                            ink::env::call::ExecutionInput::new(ink::env::call::Selector::new(
                                ink::selector_bytes!("transfer"),
                            ))
                            .push_arg(payment_info.auditor)
                            .push_arg(payment_info.value * 95 / 100), // .push_arg(&[0x10u8; 32]),
                        )
                        .returns::<Result<()>>()
                        .try_invoke();

                    let zyx = ink::env::call::build_call::<Environment>()
                        .call(self.stablecoin_address)
                        .gas_limit(0)
                        .transferred_value(0)
                        .exec_input(
                            ink::env::call::ExecutionInput::new(ink::env::call::Selector::new(
                                ink::selector_bytes!("transfer"),
                            ))
                            .push_arg(payment_info.arbiterprovider)
                            .push_arg(payment_info.value * 5 / 100), // .push_arg(&[0x10u8; 32]),
                        )
                        .returns::<Result<()>>()
                        .try_invoke();
                    if matches!(xyz.unwrap().unwrap(), Result::Ok(()))
                        && matches!(zyx.unwrap().unwrap(), Result::Ok(()))
                    {
                        self.env().emit_event(TokenOutgoing {
                            id: id,
                            receiver: payment_info.auditor,
                            amount: payment_info.value * 95 / 100,
                        });

                        self.env().emit_event(TokenOutgoing {
                            id: id,
                            receiver: payment_info.arbiterprovider,
                            amount: payment_info.value * 5 / 100,
                        });
                        payment_info.currentstatus = AuditStatus::AuditCompleted;
                        self.audit_id_to_payment_info.insert(id, &payment_info);
                        return Ok(());
                    }
                    return Err(Error::TransferFromContractFailed);
                }
                //if arbitersprovider is finally dissatisfied.
                else if !answer {
                    let xyz = ink::env::call::build_call::<Environment>()
                        .call(self.stablecoin_address)
                        .gas_limit(0)
                        .transferred_value(0)
                        .exec_input(
                            ink::env::call::ExecutionInput::new(ink::env::call::Selector::new(
                                ink::selector_bytes!("transfer"),
                            ))
                            .push_arg(payment_info.patron)
                            .push_arg(payment_info.value * 95 / 100),
                        )
                        .returns::<Result<()>>()
                        .try_invoke();
                    let zyx = ink::env::call::build_call::<Environment>()
                        .call(self.stablecoin_address)
                        .gas_limit(0)
                        .transferred_value(0)
                        .exec_input(
                            ink::env::call::ExecutionInput::new(ink::env::call::Selector::new(
                                ink::selector_bytes!("transfer"),
                            ))
                            .push_arg(payment_info.arbiterprovider)
                            .push_arg(payment_info.value * 5 / 100),
                        )
                        .returns::<Result<()>>()
                        .try_invoke();
                    if matches!(xyz.unwrap().unwrap(), Result::Ok(()))
                        && matches!(zyx.unwrap().unwrap(), Result::Ok(()))
                    {
                        payment_info.currentstatus = AuditStatus::AuditExpired;

                        self.env().emit_event(TokenOutgoing {
                            id: id,
                            receiver: payment_info.patron,
                            amount: payment_info.value * 95 / 100,
                        });
                        self.env().emit_event(TokenOutgoing {
                            id: id,
                            receiver: payment_info.arbiterprovider,
                            amount: payment_info.value * 5 / 100,
                        });
                        self.env().emit_event(AuditInfoUpdated {
                            id: Some(id),
                            payment_info: Some(self.audit_id_to_payment_info.get(id).unwrap()),
                            updated_by: Some(self.env().caller()),
                        });
                        self.audit_id_to_payment_info.insert(id, &payment_info);
                        return Ok(());
                    }
                    return Err(Error::TransferFromContractFailed);
                }
            }
            //C3
            Err(Error::UnAuthorisedCall)
        }

        //argument: id(u32) the audit ID for extending deadline
        //argument: new_deadline(u64) the new deadline
        //argument: haircut(Balance) the decided haircut for the auditor
        //argument: arbitersshare(Balance) decided off-chain by the arbitersproivder and the arbiters according to their inputs
        //and work put in for the audit ID.
        // the function is only to be called by the assigned arbitersprovider that too when the auditStatus is awaiting validation
        // the haircut and arbitersshare should be less than 10%, and the deadline should be extended by at least 1 day.
        // then the changes take place, haircut is given to patron, arbitersshare to the arbitersprovider, and payment_info is modified.
        //events for TokenOutgoing and AuditInfoUpdated are emitted.
        #[ink(message)]
        pub fn arbiters_extend_deadline(
            &mut self,
            _id: u32,
            new_deadline: u64,
            haircut: Balance,
            arbitersshare: Balance,
        ) -> Result<()> {
            //checking for the haircut to be lesser than 10% and new deadline to be at least more than 1 day.
            let mut payment_info = self.audit_id_to_payment_info.get(_id).unwrap();
            if haircut <= 90
                && new_deadline > self.env().block_timestamp() + 86400
                && self.env().caller() == payment_info.arbiterprovider
                && arbitersshare <= 10
                && matches!(
                    payment_info.currentstatus,
                    AuditStatus::AuditAwaitingValidation
                )
            {
                let arbitersscut: Balance = payment_info.value * arbitersshare / 100;
                let haircutvalue: Balance = payment_info.value * haircut / 100;
                // Update the value in storage
                payment_info.value = payment_info.value * (100 - (arbitersshare + haircut)) / 100;
                // Update the deadline in storage
                payment_info.deadline = new_deadline;
                // make the respective transfers to arbitersprovider and
                let xyz = ink::env::call::build_call::<Environment>()
                    .call(self.stablecoin_address)
                    .gas_limit(0)
                    .transferred_value(0)
                    .exec_input(
                        ink::env::call::ExecutionInput::new(ink::env::call::Selector::new(
                            ink::selector_bytes!("transfer"),
                        ))
                        .push_arg(payment_info.arbiterprovider)
                        .push_arg(arbitersscut), // .push_arg(&[0x10u8; 32]),
                    )
                    .returns::<Result<()>>()
                    .try_invoke();

                let zyx = ink::env::call::build_call::<Environment>()
                    .call(self.stablecoin_address)
                    .gas_limit(0)
                    .transferred_value(0)
                    .exec_input(
                        ink::env::call::ExecutionInput::new(ink::env::call::Selector::new(
                            ink::selector_bytes!("transfer"),
                        ))
                        .push_arg(payment_info.patron)
                        .push_arg(haircutvalue), // .push_arg(&[0x10u8; 32]),
                    )
                    .returns::<Result<()>>()
                    .try_invoke();

                if matches!(xyz.unwrap().unwrap(), Result::Ok(()))
                    && matches!(zyx.unwrap().unwrap(), Result::Ok(()))
                {
                    self.env().emit_event(TokenOutgoing {
                        id: _id,
                        receiver: payment_info.arbiterprovider,
                        amount: arbitersscut,
                    });
                    self.env().emit_event(TokenOutgoing {
                        id: _id,
                        receiver: payment_info.patron,
                        amount: haircutvalue,
                    });
                    self.audit_id_to_payment_info.insert(_id, &payment_info);
                    self.env().emit_event(AuditInfoUpdated {
                        id: Some(_id),
                        payment_info: Some(self.audit_id_to_payment_info.get(_id).unwrap()),
                        updated_by: Some(self.get_paymentinfo(_id).unwrap().patron),
                    });
                    return Ok(());
                }
            }
            Err(Error::ArbitersExtendDeadlineConditionsNotMet)
        }

        //argument: id(u32) the audit ID to be retrieved
        // the function can only be called by the patron, and only when the state is created or deadline has passed.
        // this updates the status of the audit, fires the event of TokenOutgoing, returns the value to the patron,
        pub fn expire_audit(&mut self, id: u32) -> Result<()> {
            let mut payment_info = self.audit_id_to_payment_info.get(id).unwrap();
            if payment_info.patron == self.env().caller()
                && (matches!(payment_info.currentstatus, AuditStatus::AuditCreated)
                    || payment_info.deadline <= self.env().block_timestamp())
            {
                payment_info.currentstatus = AuditStatus::AuditExpired;
                let xyz = ink::env::call::build_call::<Environment>()
                    .call(self.stablecoin_address)
                    .gas_limit(0)
                    .transferred_value(0)
                    .exec_input(
                        ink::env::call::ExecutionInput::new(ink::env::call::Selector::new(
                            ink::selector_bytes!("transfer"),
                        ))
                        .push_arg(payment_info.patron)
                        .push_arg(payment_info.value),
                    )
                    .returns::<Result<()>>()
                    .try_invoke();
                if matches!(xyz.unwrap().unwrap(), Result::Ok(())) {
                    self.env().emit_event(TokenOutgoing {
                        id: id,
                        receiver: payment_info.patron,
                        amount: payment_info.value,
                    });
                    self.env().emit_event(AuditInfoUpdated {
                        id: Some(id),
                        payment_info: Some(self.audit_id_to_payment_info.get(id).unwrap()),
                        updated_by: Some(self.env().caller()),
                    });
                    self.audit_id_to_payment_info.insert(id, &payment_info);
                    return Ok(());
                }
            }
            Err(Error::UnAuthorisedCall)
        }
    }
}
