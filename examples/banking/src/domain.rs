use disintegrate::macros::Event;
use disintegrate::{query, State, StreamQuery};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Event, Serialize, Deserialize)]
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
pub struct Account {
    id: AccountId,
    balance: i32,
    opened: bool,
    closed: bool,
}

impl State for Account {
    type Event = DomainEvent;

    fn query(&self) -> StreamQuery<Self::Event> {
        query!(DomainEvent, (account_id == self.id.clone()) or (beneficiary_id == self.id.clone()))
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

impl Account {
    pub fn new(id: AccountId) -> Self {
        Self {
            id,
            balance: 0,
            opened: false,
            closed: false,
        }
    }

    pub fn open(&self, amount: i32) -> Result<Vec<DomainEvent>, Error> {
        if self.opened {
            return Err(Error::AccountAlreadyOpened);
        }
        Ok(vec![DomainEvent::AccountOpened {
            account_id: self.id.clone(),
            amount,
        }])
    }

    pub fn close(&self) -> Result<Vec<DomainEvent>, Error> {
        if !self.opened {
            return Err(Error::AccountNotFound);
        }
        if self.closed {
            return Err(Error::AccountClosed);
        }
        Ok(vec![DomainEvent::AccountClosed {
            account_id: self.id.clone(),
        }])
    }

    pub fn deposit(&self, amount: i32) -> Result<Vec<DomainEvent>, Error> {
        if !self.opened {
            return Err(Error::AccountNotFound);
        }
        if self.closed {
            return Err(Error::AccountClosed);
        }
        Ok(vec![DomainEvent::AmountDeposited {
            account_id: self.id.clone(),
            amount,
        }])
    }

    pub fn withdraw(&self, amount: i32) -> Result<Vec<DomainEvent>, Error> {
        if !self.opened {
            return Err(Error::AccountNotFound);
        }
        if self.closed {
            return Err(Error::AccountClosed);
        }
        if self.balance < amount {
            return Err(Error::InsufficientBalance);
        }
        Ok(vec![DomainEvent::AmountWithdrawn {
            account_id: self.id.clone(),
            amount,
        }])
    }
}

#[derive(Default, Clone, Debug)]
pub struct Transfer {
    account_id: AccountId,
    beneficiary_id: AccountId,
    balance: i32,
    account_open: bool,
    beneficiary_open: bool,
}

impl State for Transfer {
    type Event = DomainEvent;

    fn query(&self) -> StreamQuery<Self::Event> {
        query!(DomainEvent, ((account_id == self.account_id.clone()) or (beneficiary_id == self.account_id.clone())) or ((account_id == self.beneficiary_id.clone()) and (events[AccountOpened, AccountClosed])))
    }

    fn mutate(&mut self, event: Self::Event) {
        match event {
            DomainEvent::AccountOpened { account_id, amount } => {
                if account_id == self.account_id {
                    self.account_open = true;
                    self.balance = amount;
                } else {
                    self.beneficiary_open = true;
                }
            }
            DomainEvent::AccountClosed { account_id } => {
                if account_id == self.account_id {
                    self.account_open = false;
                } else {
                    self.beneficiary_open = false;
                }
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
                if self.account_id == account_id {
                    self.balance -= amount;
                } else {
                    self.balance += amount;
                }
            }
        }
    }
}

impl Transfer {
    pub fn new(account_id: AccountId, beneficiary_id: AccountId) -> Self {
        Self {
            account_id,
            beneficiary_id,
            balance: 0,
            account_open: false,
            beneficiary_open: false,
        }
    }

    pub fn send(&self, amount: i32) -> Result<Vec<DomainEvent>, Error> {
        if amount < 0 {
            return Err(Error::InvalidAmount);
        }
        if !self.account_open {
            return Err(Error::AccountClosed);
        }
        if !self.beneficiary_open {
            return Err(Error::AccountClosed);
        }
        if self.balance < amount {
            return Err(Error::InsufficientBalance);
        }
        Ok(vec![DomainEvent::TransferSent {
            account_id: self.account_id.clone(),
            beneficiary_id: self.beneficiary_id.clone(),
            amount,
        }])
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn it_opens_account() {
        disintegrate::TestHarness::given(Account::new("some account".to_string()), [])
            .when(|s| s.open(10))
            .then(vec![DomainEvent::AccountOpened {
                account_id: "some account".into(),
                amount: 10,
            }]);
    }

    #[test]
    fn it_should_not_open_an_account_that_is_already_opened() {
        disintegrate::TestHarness::given(
            Account::new("some account".to_string()),
            [DomainEvent::AccountOpened {
                account_id: "some course".to_string(),
                amount: 20,
            }],
        )
        .when(|s| s.open(10))
        .then_err(Error::AccountAlreadyOpened);
    }

    #[test]
    fn it_closes_account() {
        disintegrate::TestHarness::given(
            Account::new("some account".to_string()),
            [DomainEvent::AccountOpened {
                account_id: "some account".into(),
                amount: 10,
            }],
        )
        .when(|s| s.close())
        .then(vec![DomainEvent::AccountClosed {
            account_id: "some account".into(),
        }]);
    }

    #[test]
    fn it_should_not_close_an_account_that_does_not_exist() {
        disintegrate::TestHarness::given(Account::new("some account".to_string()), [])
            .when(|s| s.close())
            .then_err(Error::AccountNotFound);
    }

    #[test]
    fn it_should_not_close_an_account_that_is_already_closed() {
        disintegrate::TestHarness::given(
            Account::new("some account".to_string()),
            [
                DomainEvent::AccountOpened {
                    account_id: "some account".into(),
                    amount: 10,
                },
                DomainEvent::AccountClosed {
                    account_id: "some account".into(),
                },
            ],
        )
        .when(|s| s.close())
        .then_err(Error::AccountClosed);
    }

    #[test]
    fn it_deposits_an_amount() {
        disintegrate::TestHarness::given(
            Account::new("some account".to_string()),
            [DomainEvent::AccountOpened {
                account_id: "some account".into(),
                amount: 10,
            }],
        )
        .when(|s| s.deposit(20))
        .then(vec![DomainEvent::AmountDeposited {
            account_id: "some account".into(),
            amount: 20,
        }]);
    }

    #[test]
    fn it_should_not_deposit_an_amount_into_an_account_that_does_not_exist() {
        disintegrate::TestHarness::given(Account::new("some account".to_string()), [])
            .when(|s| s.deposit(10))
            .then_err(Error::AccountNotFound);
    }

    #[test]
    fn it_should_not_deposit_an_amount_into_an_account_that_is_closed() {
        disintegrate::TestHarness::given(
            Account::new("some account".to_string()),
            [
                DomainEvent::AccountOpened {
                    account_id: "some account".into(),
                    amount: 10,
                },
                DomainEvent::AccountClosed {
                    account_id: "some account".into(),
                },
            ],
        )
        .when(|s| s.deposit(20))
        .then_err(Error::AccountClosed);
    }

    #[test]
    fn it_withdraws_an_amount() {
        disintegrate::TestHarness::given(
            Account::new("some account".to_string()),
            [DomainEvent::AccountOpened {
                account_id: "some account".into(),
                amount: 10,
            }],
        )
        .when(|s| s.withdraw(10))
        .then(vec![DomainEvent::AmountWithdrawn {
            account_id: "some account".into(),
            amount: 10,
        }]);
    }

    #[test]
    fn it_should_not_withdraw_an_amount_from_an_account_that_does_not_exist() {
        disintegrate::TestHarness::given(Account::new("some account".to_string()), [])
            .when(|s| s.withdraw(10))
            .then_err(Error::AccountNotFound);
    }

    #[test]
    fn it_should_not_withdraw_an_amount_from_an_account_that_is_closed() {
        disintegrate::TestHarness::given(
            Account::new("some account".to_string()),
            [
                DomainEvent::AccountOpened {
                    account_id: "some account".into(),
                    amount: 10,
                },
                DomainEvent::AccountClosed {
                    account_id: "some account".into(),
                },
            ],
        )
        .when(|s| s.withdraw(5))
        .then_err(Error::AccountClosed);
    }

    #[test]
    fn it_should_not_withdraw_an_amount_when_the_balance_is_insufficient() {
        disintegrate::TestHarness::given(
            Account::new("some account".to_string()),
            [
                DomainEvent::AccountOpened {
                    account_id: "some account".into(),
                    amount: 30,
                },
                DomainEvent::AmountWithdrawn {
                    account_id: "some account".into(),
                    amount: 15,
                },
            ],
        )
        .when(|s| s.withdraw(20))
        .then_err(Error::InsufficientBalance);
    }

    #[test]
    fn it_sends_a_transfer() {
        disintegrate::TestHarness::given(
            Transfer::new("some account".to_string(), "some beneficiary".to_string()),
            [
                DomainEvent::AccountOpened {
                    account_id: "some account".into(),
                    amount: 5,
                },
                DomainEvent::AmountDeposited {
                    account_id: "some account".into(),
                    amount: 5,
                },
                DomainEvent::AccountOpened {
                    account_id: "some beneficiary".into(),
                    amount: 5,
                },
            ],
        )
        .when(|s| s.send(7))
        .then(vec![DomainEvent::TransferSent {
            account_id: "some account".to_string(),
            beneficiary_id: "some beneficiary".to_string(),
            amount: 7,
        }])
    }

    #[test]
    fn it_should_not_send_a_transfer_when_the_account_is_not_opened() {
        disintegrate::TestHarness::given(
            Transfer::new("some account".to_string(), "some beneficiary".to_string()),
            [DomainEvent::AccountOpened {
                account_id: "some beneficiary".into(),
                amount: 5,
            }],
        )
        .when(|s| s.send(7))
        .then_err(Error::AccountClosed);
    }

    #[test]
    fn it_should_not_send_a_transfer_when_beneficiary_account_is_not_opened() {
        disintegrate::TestHarness::given(
            Transfer::new("some account".to_string(), "some beneficiary".to_string()),
            [
                DomainEvent::AccountOpened {
                    account_id: "some account".into(),
                    amount: 5,
                },
                DomainEvent::AmountDeposited {
                    account_id: "some account".into(),
                    amount: 5,
                },
            ],
        )
        .when(|s| s.send(7))
        .then_err(Error::AccountClosed);
    }

    #[test]
    fn it_should_not_send_a_transfer_when_the_balance_is_insufficient() {
        disintegrate::TestHarness::given(
            Transfer::new("some account".to_string(), "some beneficiary".to_string()),
            [
                DomainEvent::AccountOpened {
                    account_id: "some account".into(),
                    amount: 5,
                },
                DomainEvent::AccountOpened {
                    account_id: "some beneficiary".into(),
                    amount: 5,
                },
            ],
        )
        .when(|s| s.send(7))
        .then_err(Error::InsufficientBalance);
    }
}
