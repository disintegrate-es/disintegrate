use disintegrate::serde::json::Json;
use disintegrate::StateStore;
use disintegrate_postgres::PgEventStore;

use crate::domain::{Account, AccountId, DomainEvent, Transfer};

type EventStore = PgEventStore<DomainEvent, Json<DomainEvent>>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    EventStore(#[from] disintegrate_postgres::Error),
    #[error(transparent)]
    Domain(#[from] crate::domain::Error),
}

#[derive(Clone)]
pub struct Application {
    state_store: EventStore,
}

impl Application {
    pub fn new(state_store: EventStore) -> Self {
        Self { state_store }
    }
    pub async fn open_account(&self, id: AccountId, amount: i32) -> Result<(), Error> {
        let account = self.state_store.hydrate(Account::new(id)).await?;
        let changes = account.open(amount)?;
        self.state_store.save(&account, changes).await?;
        Ok(())
    }
    pub async fn close_account(&self, id: AccountId) -> Result<(), Error> {
        let account = self.state_store.hydrate(Account::new(id)).await?;
        let changes = account.close()?;
        self.state_store.save(&account, changes).await?;
        Ok(())
    }
    pub async fn deposit_amount(&self, id: AccountId, amount: i32) -> Result<(), Error> {
        let account = self.state_store.hydrate(Account::new(id)).await?;
        let changes = account.deposit(amount)?;
        self.state_store.save(&account, changes).await?;
        Ok(())
    }
    pub async fn withdraw_amount(&self, id: AccountId, amount: i32) -> Result<(), Error> {
        let account = self.state_store.hydrate(Account::new(id)).await?;
        let changes = account.withdraw(amount)?;
        self.state_store.save(&account, changes).await?;
        Ok(())
    }
    pub async fn transfer_amount(
        &self,
        id: AccountId,
        beneficiary_id: AccountId,
        amount: i32,
    ) -> Result<(), Error> {
        let transfer = self
            .state_store
            .hydrate(Transfer::new(id, beneficiary_id))
            .await?;
        let changes = transfer.send(amount)?;
        self.state_store.save(&transfer, changes).await?;
        Ok(())
    }
}
