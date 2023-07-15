use disintegrate::macros::Event;
use disintegrate::{events_types, query, Decision, State, StreamQuery};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Event, Serialize, Deserialize)]
#[group(AccountStateEvent, [AccountOpened, AccountClosed])]
pub enum DomainEvent {
    AccountOpened {
        #[id]
        account_id: AccountId,
        amount: i32,
    },
    AccountClosed {
        #[id]
        account_id: AccountId,
    },
    TransferSent {
        #[id]
        account_id: AccountId,
        #[id]
        beneficiary_id: AccountId,
        amount: i32,
    },
    AmountDeposited {
        #[id]
        account_id: AccountId,
        amount: i32,
    },
    AmountWithdrawn {
        #[id]
        account_id: String,
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

pub type AccountId = String;

#[derive(Default, Clone, Debug)]
pub struct AccountBalance {
    id: AccountId,
    balance: i32,
    opened: bool,
    closed: bool,
}
impl AccountBalance {
    pub fn new(id: AccountId) -> Self {
        Self {
            id,
            balance: 0,
            opened: false,
            closed: false,
        }
    }
}

impl State for AccountBalance {
    type Event = DomainEvent;

    fn query(&self) -> StreamQuery<Self::Event> {
        query!(DomainEvent, (account_id == self.id) or (beneficiary_id == self.id))
    }

    fn mutate(&mut self, event: Self::Event) {
        match event {
            DomainEvent::AccountOpened { amount, .. } => {
                self.opened = true;
                self.balance = amount;
            }
            DomainEvent::AccountClosed { .. } => {
                self.closed = true;
            }
            DomainEvent::AmountDeposited { amount, .. } => {
                self.balance += amount;
            }
            DomainEvent::AmountWithdrawn { amount, .. } => {
                self.balance -= amount;
            }
            DomainEvent::TransferSent {
                account_id, amount, ..
            } => {
                if self.id == account_id {
                    self.balance -= amount;
                } else {
                    self.balance += amount;
                }
            }
        }
    }
}

#[derive(Default, Clone, Debug)]
pub struct AccountState {
    id: AccountId,
    opened: bool,
    closed: bool,
}
impl AccountState {
    pub fn new(id: AccountId) -> Self {
        Self {
            id,
            opened: false,
            closed: false,
        }
    }
}

impl State for AccountState {
    type Event = AccountStateEvent;

    fn query(&self) -> StreamQuery<Self::Event> {
        query!(AccountStateEvent, account_id == self.id)
    }

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
    initial_amount: i32,
}

impl OpenAccount {
    pub fn new(id: AccountId, initial_amount: i32) -> Self {
        Self { id, initial_amount }
    }
}

impl Decision for OpenAccount {
    type Event = DomainEvent;

    type State = AccountState;

    type Error = Error;

    fn default_state(&self) -> Self::State {
        AccountState::new(self.id.clone())
    }

    fn validation_query(&self) -> Option<StreamQuery<<Self::State as State>::Event>> {
        None
    }

    fn process(&self, state: &Self::State) -> Result<Vec<Self::Event>, Self::Error> {
        if state.opened {
            return Err(Error::AccountAlreadyOpened);
        }

        Ok(vec![DomainEvent::AccountOpened {
            account_id: self.id.clone(),
            amount: self.initial_amount,
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

    type State = AccountState;

    type Error = Error;

    fn default_state(&self) -> Self::State {
        AccountState::new(self.id.clone())
    }

    fn validation_query(&self) -> Option<StreamQuery<<Self::State as State>::Event>> {
        None
    }

    fn process(&self, state: &Self::State) -> Result<Vec<Self::Event>, Self::Error> {
        if !state.opened {
            return Err(Error::AccountNotFound);
        }

        if state.closed {
            return Err(Error::AccountClosed);
        }

        Ok(vec![DomainEvent::AccountClosed {
            account_id: self.id.clone(),
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
    type State = AccountState;
    type Error = Error;

    fn default_state(&self) -> Self::State {
        AccountState::new(self.id.clone())
    }

    fn validation_query(&self) -> Option<StreamQuery<<Self::State as State>::Event>> {
        None
    }

    fn process(&self, state: &Self::State) -> Result<Vec<Self::Event>, Self::Error> {
        if !state.opened {
            return Err(Error::AccountNotFound);
        }

        if state.closed {
            return Err(Error::AccountClosed);
        }

        Ok(vec![DomainEvent::AmountDeposited {
            account_id: self.id.clone(),
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

    type State = AccountBalance;

    type Error = Error;

    fn default_state(&self) -> Self::State {
        AccountBalance::new(self.id.clone())
    }

    fn validation_query(&self) -> Option<StreamQuery<<Self::State as State>::Event>> {
        Some(
            self.default_state()
                .query()
                .exclude_events(events_types!(DomainEvent, [AmountDeposited])),
        )
    }

    fn process(&self, state: &Self::State) -> Result<Vec<Self::Event>, Self::Error> {
        if !state.opened {
            return Err(Error::AccountNotFound);
        }

        if state.closed {
            return Err(Error::AccountClosed);
        }

        if state.balance < self.amount {
            return Err(Error::InsufficientBalance);
        }

        Ok(vec![DomainEvent::AmountWithdrawn {
            account_id: self.id.clone(),
            amount: self.amount,
        }])
    }
}

#[derive(Default, Clone, Debug)]
pub struct MoneyTransfer {
    account_balance: AccountBalance,
    beneficiary_status: AccountState,
}

impl MoneyTransfer {
    pub fn new(account_id: AccountId, beneficiary_id: AccountId) -> Self {
        Self {
            account_balance: AccountBalance {
                id: account_id,
                ..Default::default()
            },
            beneficiary_status: AccountState {
                id: beneficiary_id,
                ..Default::default()
            },
        }
    }
}

impl State for MoneyTransfer {
    type Event = DomainEvent;

    fn query(&self) -> StreamQuery<Self::Event> {
        query!(DomainEvent,
            ((account_id == self.account_balance.id) or (beneficiary_id == self.account_balance.id))
         or ((account_id == self.beneficiary_status.id) and (events[AccountOpened, AccountClosed]))
        )
    }

    fn mutate(&mut self, event: Self::Event) {
        match &event {
            DomainEvent::AccountOpened { account_id, .. } => {
                if account_id == &self.account_balance.id {
                    self.account_balance.mutate(event);
                } else {
                    self.beneficiary_status.mutate(event.try_into().unwrap());
                }
            }
            DomainEvent::AccountClosed { account_id } => {
                if account_id == &self.account_balance.id {
                    self.account_balance.mutate(event);
                } else {
                    self.beneficiary_status.mutate(event.try_into().unwrap());
                }
            }
            _ => {
                self.account_balance.mutate(event);
            }
        }
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

    type State = MoneyTransfer;

    type Error = Error;

    fn default_state(&self) -> Self::State {
        MoneyTransfer::new(self.account_id.clone(), self.beneficiary_id.clone())
    }

    fn validation_query(&self) -> Option<StreamQuery<<Self::State as State>::Event>> {
        Some(
            query!(DomainEvent,
                (account_id == self.account_id)
             or ((account_id == self.beneficiary_id) and (events[AccountClosed]))
            )
            .exclude_events(events_types!(DomainEvent, [AmountDeposited])),
        )
    }

    fn process(&self, state: &Self::State) -> Result<Vec<Self::Event>, Self::Error> {
        if self.amount < 0 {
            return Err(Error::InvalidAmount);
        }

        if !state.account_balance.opened {
            return Err(Error::AccountNotFound);
        }

        if !state.beneficiary_status.opened {
            return Err(Error::AccountNotFound);
        }

        if state.account_balance.closed {
            return Err(Error::AccountClosed);
        }

        if state.beneficiary_status.closed {
            return Err(Error::AccountClosed);
        }

        if state.account_balance.balance < self.amount {
            return Err(Error::InsufficientBalance);
        }

        Ok(vec![DomainEvent::TransferSent {
            account_id: self.account_id.clone(),
            beneficiary_id: self.beneficiary_id.clone(),
            amount: self.amount,
        }])
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn it_opens_account() {
        disintegrate::TestHarness::given([])
            .when(OpenAccount::new("some account".into(), 10))
            .then([DomainEvent::AccountOpened {
                account_id: "some account".into(),
                amount: 10,
            }]);
    }

    #[test]
    fn it_should_not_open_an_account_that_is_already_opened() {
        disintegrate::TestHarness::given([AccountStateEvent::AccountOpened {
            account_id: "some course".to_string(),
            amount: 20,
        }])
        .when(OpenAccount::new("some account".into(), 10))
        .then_err(Error::AccountAlreadyOpened);
    }

    #[test]
    fn it_closes_account() {
        disintegrate::TestHarness::given([AccountStateEvent::AccountOpened {
            account_id: "some account".into(),
            amount: 10,
        }])
        .when(CloseAccount::new("some account".into()))
        .then(vec![DomainEvent::AccountClosed {
            account_id: "some account".into(),
        }]);
    }

    #[test]
    fn it_should_not_close_an_account_that_does_not_exist() {
        disintegrate::TestHarness::given([])
            .when(CloseAccount::new("some account".into()))
            .then_err(Error::AccountNotFound);
    }

    #[test]
    fn it_should_not_close_an_account_that_is_already_closed() {
        disintegrate::TestHarness::given([
            AccountStateEvent::AccountOpened {
                account_id: "some account".into(),
                amount: 10,
            },
            AccountStateEvent::AccountClosed {
                account_id: "some account".into(),
            },
        ])
        .when(CloseAccount::new("some account".into()))
        .then_err(Error::AccountClosed);
    }

    #[test]
    fn it_deposits_an_amount() {
        disintegrate::TestHarness::given([AccountStateEvent::AccountOpened {
            account_id: "some account".into(),
            amount: 10,
        }])
        .when(DepositAmount::new("some account".into(), 20))
        .then([DomainEvent::AmountDeposited {
            account_id: "some account".into(),
            amount: 20,
        }]);
    }

    #[test]
    fn it_should_not_deposit_an_amount_into_an_account_that_does_not_exist() {
        disintegrate::TestHarness::given([])
            .when(DepositAmount::new("some account".into(), 20))
            .then_err(Error::AccountNotFound);
    }

    #[test]
    fn it_should_not_deposit_an_amount_into_an_account_that_is_closed() {
        disintegrate::TestHarness::given([
            AccountStateEvent::AccountOpened {
                account_id: "some account".into(),
                amount: 10,
            },
            AccountStateEvent::AccountClosed {
                account_id: "some account".into(),
            },
        ])
        .when(DepositAmount::new("some account".into(), 20))
        .then_err(Error::AccountClosed);
    }

    #[test]
    fn it_withdraws_an_amount() {
        disintegrate::TestHarness::given([DomainEvent::AccountOpened {
            account_id: "some account".into(),
            amount: 10,
        }])
        .when(WithdrawAmount::new("some account".into(), 10))
        .then([DomainEvent::AmountWithdrawn {
            account_id: "some account".into(),
            amount: 10,
        }]);
    }

    #[test]
    fn it_should_not_withdraw_an_amount_from_an_account_that_does_not_exist() {
        disintegrate::TestHarness::given([])
            .when(WithdrawAmount::new("some account".into(), 10))
            .then_err(Error::AccountNotFound);
    }

    #[test]
    fn it_should_not_withdraw_an_amount_from_an_account_that_is_closed() {
        disintegrate::TestHarness::given([
            DomainEvent::AccountOpened {
                account_id: "some account".into(),
                amount: 10,
            },
            DomainEvent::AccountClosed {
                account_id: "some account".into(),
            },
        ])
        .when(WithdrawAmount::new("some account".into(), 5))
        .then_err(Error::AccountClosed);
    }

    #[test]
    fn it_should_not_withdraw_an_amount_when_the_balance_is_insufficient() {
        disintegrate::TestHarness::given([
            DomainEvent::AccountOpened {
                account_id: "some account".into(),
                amount: 30,
            },
            DomainEvent::AmountWithdrawn {
                account_id: "some account".into(),
                amount: 26,
            },
        ])
        .when(WithdrawAmount::new("some account".into(), 5))
        .then_err(Error::InsufficientBalance);
    }

    #[test]
    fn it_sends_a_transfer() {
        disintegrate::TestHarness::given([
            DomainEvent::AccountOpened {
                account_id: "some account".into(),
                amount: 5,
            },
            DomainEvent::AmountDeposited {
                account_id: "some account".into(),
                amount: 100,
            },
            DomainEvent::AccountOpened {
                account_id: "some beneficiary".into(),
                amount: 5,
            },
        ])
        .when(SendMoney::new(
            "some account".into(),
            "some beneficiary".into(),
            7,
        ))
        .then(vec![DomainEvent::TransferSent {
            account_id: "some account".to_string(),
            beneficiary_id: "some beneficiary".to_string(),
            amount: 7,
        }])
    }

    #[test]
    fn it_should_not_send_a_transfer_when_the_account_is_not_opened() {
        disintegrate::TestHarness::given([DomainEvent::AccountOpened {
            account_id: "some beneficiary".into(),
            amount: 5,
        }])
        .when(SendMoney::new(
            "some account".into(),
            "some beneficiary".into(),
            7,
        ))
        .then_err(Error::AccountNotFound);
    }

    #[test]
    fn it_should_not_send_a_transfer_when_beneficiary_account_is_not_opened() {
        disintegrate::TestHarness::given([
            DomainEvent::AccountOpened {
                account_id: "some account".into(),
                amount: 5,
            },
            DomainEvent::AmountDeposited {
                account_id: "some account".into(),
                amount: 5,
            },
        ])
        .when(SendMoney::new(
            "some account".into(),
            "some beneficiary".into(),
            7,
        ))
        .then_err(Error::AccountNotFound);
    }

    #[test]
    fn it_should_not_send_a_transfer_when_the_balance_is_insufficient() {
        disintegrate::TestHarness::given([
            DomainEvent::AccountOpened {
                account_id: "some account".into(),
                amount: 5,
            },
            DomainEvent::AccountOpened {
                account_id: "some beneficiary".into(),
                amount: 5,
            },
        ])
        .when(SendMoney::new(
            "some account".into(),
            "some beneficiary".into(),
            7,
        ))
        .then_err(Error::InsufficientBalance);
    }
}
