use disintegrate::{event_types, union, Decision, Event, StateMutate, StateQuery, StreamQuery};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Event, Serialize, Deserialize)]
#[stream(AccountStateEvent, [AccountOpened, AccountClosed])]
#[stream(AccountBalanceEvent, [AmountDeposited, AmountWithdrawn, TransferSent, TransferReceived])]
pub enum DomainEvent {
    AccountOpened {
        #[id]
        account_id: AccountId,
    },
    AccountClosed {
        #[id]
        account_id: AccountId,
    },
    TransferSent {
        #[id]
        account_id: AccountId,
        to: AccountId,
        amount: i32,
    },
    TransferReceived {
        #[id]
        account_id: AccountId,
        from: AccountId,
        amount: i32,
    },
    AmountDeposited {
        #[id]
        account_id: AccountId,
        amount: i32,
    },
    AmountWithdrawn {
        #[id]
        account_id: AccountId,
        amount: i32,
    },
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum Error {
    #[error("invalid amount")]
    InvalidAmount,
    #[error("insufficient balance")]
    InsufficientBalance,
    #[error("account already opened")]
    AccountAlreadyOpened,
    #[error("account not found")]
    AccountNotFound,
    #[error("account is closed")]
    AccountClosed,
}

pub type AccountId = i64;

#[derive(Default, StateQuery, Clone, Debug, Serialize, Deserialize)]
#[state_query(AccountBalanceEvent)]
pub struct AccountBalance {
    #[id]
    account_id: AccountId,
    balance: i32,
}
impl AccountBalance {
    pub fn new(account_id: AccountId) -> Self {
        Self {
            account_id,
            balance: 0,
        }
    }
}

impl StateMutate for AccountBalance {
    fn mutate(&mut self, event: Self::Event) {
        match event {
            AccountBalanceEvent::AmountDeposited { amount, .. } => {
                self.balance += amount;
            }
            AccountBalanceEvent::AmountWithdrawn { amount, .. } => {
                self.balance -= amount;
            }
            AccountBalanceEvent::TransferSent { amount, .. } => {
                self.balance -= amount;
            }
            AccountBalanceEvent::TransferReceived { amount, .. } => {
                self.balance += amount;
            }
        }
    }
}

#[derive(Default, StateQuery, Clone, Debug, Serialize, Deserialize)]
#[state_query(AccountStateEvent)]
pub struct AccountState {
    #[id]
    account_id: AccountId,
    opened: bool,
    closed: bool,
}

impl AccountState {
    pub fn new(account_id: AccountId) -> Self {
        Self {
            account_id,
            opened: false,
            closed: false,
        }
    }
}

impl StateMutate for AccountState {
    fn mutate(&mut self, event: Self::Event) {
        match event {
            AccountStateEvent::AccountOpened { .. } => {
                self.opened = true;
            }
            AccountStateEvent::AccountClosed { .. } => {
                self.closed = true;
            }
        }
    }
}
pub struct OpenAccount {
    id: AccountId,
}

impl OpenAccount {
    pub fn new(id: AccountId) -> Self {
        Self { id }
    }
}

impl Decision for OpenAccount {
    type Event = DomainEvent;

    type StateQuery = AccountState;

    type Error = Error;

    fn state_query(&self) -> Self::StateQuery {
        AccountState::new(self.id)
    }

    fn process(&self, state: &Self::StateQuery) -> Result<Vec<Self::Event>, Self::Error> {
        if state.opened {
            return Err(Error::AccountAlreadyOpened);
        }

        Ok(vec![DomainEvent::AccountOpened {
            account_id: self.id,
        }])
    }
}

pub struct CloseAccount {
    id: AccountId,
}

impl CloseAccount {
    pub fn new(id: AccountId) -> Self {
        Self { id }
    }
}

impl Decision for CloseAccount {
    type Event = DomainEvent;

    type StateQuery = AccountState;

    type Error = Error;

    fn state_query(&self) -> Self::StateQuery {
        AccountState::new(self.id)
    }

    fn process(&self, state: &Self::StateQuery) -> Result<Vec<Self::Event>, Self::Error> {
        if !state.opened {
            return Err(Error::AccountNotFound);
        }

        if state.closed {
            return Err(Error::AccountClosed);
        }

        Ok(vec![DomainEvent::AccountClosed {
            account_id: self.id,
        }])
    }
}

pub struct DepositAmount {
    id: AccountId,
    amount: i32,
}

impl DepositAmount {
    pub fn new(id: AccountId, amount: i32) -> Self {
        Self { id, amount }
    }
}

impl Decision for DepositAmount {
    type Event = DomainEvent;
    type StateQuery = (AccountState, AccountBalance);
    type Error = Error;

    fn state_query(&self) -> Self::StateQuery {
        (AccountState::new(self.id), AccountBalance::new(self.id))
    }

    fn process(&self, (state, _): &Self::StateQuery) -> Result<Vec<Self::Event>, Self::Error> {
        if !state.opened {
            return Err(Error::AccountNotFound);
        }

        if state.closed {
            return Err(Error::AccountClosed);
        }

        Ok(vec![DomainEvent::AmountDeposited {
            account_id: self.id,
            amount: self.amount,
        }])
    }
}

pub struct WithdrawAmount {
    id: AccountId,
    amount: i32,
}

impl WithdrawAmount {
    pub fn new(id: AccountId, amount: i32) -> Self {
        Self { id, amount }
    }
}

impl Decision for WithdrawAmount {
    type Event = DomainEvent;

    type StateQuery = (AccountState, AccountBalance);

    type Error = Error;

    fn state_query(&self) -> Self::StateQuery {
        (AccountState::new(self.id), AccountBalance::new(self.id))
    }

    fn validation_query<ID: disintegrate::EventId>(&self) -> Option<StreamQuery<ID, Self::Event>> {
        let (account_state, account_balance) = self.state_query();
        Some(union!(
            &account_state,
            account_balance.exclude_events(event_types!(DomainEvent, [AmountDeposited]))
        ))
    }

    fn process(
        &self,
        (state, balance): &Self::StateQuery,
    ) -> Result<Vec<Self::Event>, Self::Error> {
        if !state.opened {
            return Err(Error::AccountNotFound);
        }

        if state.closed {
            return Err(Error::AccountClosed);
        }

        if balance.balance < self.amount {
            return Err(Error::InsufficientBalance);
        }

        Ok(vec![DomainEvent::AmountWithdrawn {
            account_id: self.id,
            amount: self.amount,
        }])
    }
}

pub struct SendMoney {
    account_id: AccountId,
    beneficiary_id: AccountId,
    amount: i32,
}

impl SendMoney {
    pub fn new(account_id: AccountId, beneficiary_id: AccountId, amount: i32) -> Self {
        Self {
            account_id,
            beneficiary_id,
            amount,
        }
    }
}

impl Decision for SendMoney {
    type Event = DomainEvent;

    type StateQuery = (AccountState, AccountBalance, AccountState);

    type Error = Error;

    fn state_query(&self) -> Self::StateQuery {
        (
            AccountState::new(self.account_id),
            AccountBalance::new(self.account_id),
            AccountState::new(self.beneficiary_id),
        )
    }

    fn validation_query<ID: disintegrate::EventId>(&self) -> Option<StreamQuery<ID, Self::Event>> {
        let (account_state, account_balance, beneficiary_state) = self.state_query();
        Some(union!(
            &account_state,
            account_balance.exclude_events(event_types!(AccountBalanceEvent, [AmountDeposited])),
            &beneficiary_state
        ))
    }

    fn process(
        &self,
        (account_state, account_balance, beneficiary_account_state): &Self::StateQuery,
    ) -> Result<Vec<Self::Event>, Self::Error> {
        if self.amount < 0 {
            return Err(Error::InvalidAmount);
        }

        if !account_state.opened {
            return Err(Error::AccountNotFound);
        }

        if !beneficiary_account_state.opened {
            return Err(Error::AccountNotFound);
        }

        if account_state.closed {
            return Err(Error::AccountClosed);
        }

        if beneficiary_account_state.closed {
            return Err(Error::AccountClosed);
        }

        if account_balance.balance < self.amount {
            return Err(Error::InsufficientBalance);
        }

        Ok(vec![
            DomainEvent::TransferSent {
                account_id: self.account_id,
                to: self.beneficiary_id,
                amount: self.amount,
            },
            DomainEvent::TransferReceived {
                account_id: self.beneficiary_id,
                from: self.account_id,
                amount: self.amount,
            },
        ])
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn it_opens_account() {
        disintegrate::TestHarness::given([])
            .when(OpenAccount::new(1))
            .then([DomainEvent::AccountOpened { account_id: 1 }]);
    }

    #[test]
    fn it_should_not_open_an_account_that_is_already_opened() {
        disintegrate::TestHarness::given([DomainEvent::AccountOpened { account_id: 1 }])
            .when(OpenAccount::new(1))
            .then_err(Error::AccountAlreadyOpened);
    }

    #[test]
    fn it_closes_account() {
        disintegrate::TestHarness::given([DomainEvent::AccountOpened { account_id: 1 }])
            .when(CloseAccount::new(1))
            .then(vec![DomainEvent::AccountClosed { account_id: 1 }]);
    }

    #[test]
    fn it_should_not_close_an_account_that_does_not_exist() {
        disintegrate::TestHarness::given([])
            .when(CloseAccount::new(1))
            .then_err(Error::AccountNotFound);
    }

    #[test]
    fn it_should_not_close_an_account_that_is_already_closed() {
        disintegrate::TestHarness::given([
            DomainEvent::AccountOpened { account_id: 1 },
            DomainEvent::AccountClosed { account_id: 1 },
        ])
        .when(CloseAccount::new(1))
        .then_err(Error::AccountClosed);
    }

    #[test]
    fn it_deposits_an_amount() {
        disintegrate::TestHarness::given([DomainEvent::AccountOpened { account_id: 1 }])
            .when(DepositAmount::new(1, 20))
            .then([DomainEvent::AmountDeposited {
                account_id: 1,
                amount: 20,
            }]);
    }

    #[test]
    fn it_should_not_deposit_an_amount_into_an_account_that_does_not_exist() {
        disintegrate::TestHarness::given([])
            .when(DepositAmount::new(1, 20))
            .then_err(Error::AccountNotFound);
    }

    #[test]
    fn it_should_not_deposit_an_amount_into_an_account_that_is_closed() {
        disintegrate::TestHarness::given([
            DomainEvent::AccountOpened { account_id: 1 },
            DomainEvent::AccountClosed { account_id: 1 },
        ])
        .when(DepositAmount::new(1, 20))
        .then_err(Error::AccountClosed);
    }

    #[test]
    fn it_withdraws_an_amount() {
        disintegrate::TestHarness::given([
            DomainEvent::AccountOpened { account_id: 1 },
            DomainEvent::AmountDeposited {
                account_id: 1,
                amount: 10,
            },
        ])
        .when(WithdrawAmount::new(1, 10))
        .then([DomainEvent::AmountWithdrawn {
            account_id: 1,
            amount: 10,
        }]);
    }

    #[test]
    fn it_should_not_withdraw_an_amount_from_an_account_that_does_not_exist() {
        disintegrate::TestHarness::given([])
            .when(WithdrawAmount::new(1, 10))
            .then_err(Error::AccountNotFound);
    }

    #[test]
    fn it_should_not_withdraw_an_amount_from_an_account_that_is_closed() {
        disintegrate::TestHarness::given([
            DomainEvent::AccountOpened { account_id: 1 },
            DomainEvent::AccountClosed { account_id: 1 },
        ])
        .when(WithdrawAmount::new(1, 5))
        .then_err(Error::AccountClosed);
    }

    #[test]
    fn it_should_not_withdraw_an_amount_when_the_balance_is_insufficient() {
        disintegrate::TestHarness::given([
            DomainEvent::AccountOpened { account_id: 1 },
            DomainEvent::AmountDeposited {
                account_id: 1,
                amount: 10,
            },
            DomainEvent::AmountWithdrawn {
                account_id: 1,
                amount: 26,
            },
        ])
        .when(WithdrawAmount::new(1, 5))
        .then_err(Error::InsufficientBalance);
    }

    #[test]
    fn it_sends_a_transfer() {
        disintegrate::TestHarness::given([
            DomainEvent::AccountOpened { account_id: 1 },
            DomainEvent::AmountDeposited {
                account_id: 1,
                amount: 100,
            },
            DomainEvent::AccountOpened { account_id: 2 },
        ])
        .when(SendMoney::new(1, 2, 7))
        .then(vec![
            DomainEvent::TransferSent {
                account_id: 1,
                to: 2,
                amount: 7,
            },
            DomainEvent::TransferReceived {
                account_id: 2,
                from: 1,
                amount: 7,
            },
        ])
    }

    #[test]
    fn it_should_not_send_a_transfer_when_the_account_is_not_opened() {
        disintegrate::TestHarness::given([DomainEvent::AccountOpened { account_id: 2 }])
            .when(SendMoney::new(1, 2, 7))
            .then_err(Error::AccountNotFound);
    }

    #[test]
    fn it_should_not_send_a_transfer_when_beneficiary_account_is_not_opened() {
        disintegrate::TestHarness::given([
            DomainEvent::AccountOpened { account_id: 1 },
            DomainEvent::AmountDeposited {
                account_id: 1,
                amount: 5,
            },
        ])
        .when(SendMoney::new(1, 2, 7))
        .then_err(Error::AccountNotFound);
    }

    #[test]
    fn it_should_not_send_a_transfer_when_the_balance_is_insufficient() {
        disintegrate::TestHarness::given([
            DomainEvent::AccountOpened { account_id: 1 },
            DomainEvent::AmountDeposited {
                account_id: 1,
                amount: 7,
            },
            DomainEvent::AccountOpened { account_id: 2 },
        ])
        .when(SendMoney::new(1, 2, 8))
        .then_err(Error::InsufficientBalance);
    }
}
