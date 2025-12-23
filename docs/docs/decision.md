---
sidebar_position: 4
---

# Decision

`Decision` encapsulates a specific action or behavior triggered by external commands or events. To implement a `Decision`, developers must implement the `Decision` trait, which contains the following methods:
* `state_query`:  A state query represents the current state of the system, derived from past events stored in the event store. It provides the necessary context for making decisions and serves as the input for decision logic.
* `process`: It defines business logic based on the queried state, and returns a vector of events representing the changes to be applied to the system.
* `validation_query`: This method provides an optional state query used to determine if the decision is still valid after new events have been applied to the system before writing the decision events. If this method is not implemented, the default implementation uses the state query returned by the state_query method. This ensures that the decision was taken using an updated state. However, sometimes you may want to define a validation query to improve performance by tailoring the validation scope.

`Decision`s provide developers with a structured and scalable approach to implementing business logic. They enable:
* Modularity: `Decision`s embody specific business logic, promoting modularity and enabling the segregation of concerns within the application architecture. This structured approach facilitates the maintenance of the system.
* Testability: `Decision`s facilitate test-driven development (TDD) practices by defining clear boundaries for writing test cases and verifying behavior.

```rust
pub struct WithdrawAmount {
    account_id: String,
    amount: u32,
}

impl WithdrawAmount {
    pub fn new(account_id: String, amount: u32) -> Self {
        Self { account_id, amount }
    }
}

impl Decision for WithdrawAmount {
    type Event = DomainEvent;
    type StateQuery = AccountState;
    type Error = AccountError;

    fn state_query(&self) -> Self::StateQuery {
        AccountState::new(&self.account_id)
    }

    fn process(&self, state: &Self::StateQuery) -> Result<Vec<Self::Event>, Self::Error> {
        // Validate account balance and perform withdrawal logic
        // Construct and return events representing the changes
    }
}
```

## Developing a new Decision

Before implementing a Decision, it's advisable to start by writing tests. Disintegrate offers the `TestHarness`, a utility for writing tests in a given-when-then style. This tool assists you in defining the business logic of your application following a Test-Driven Development (TDD) approach:

```rust
#[test]
fn it_withdraws_an_amount() {
    disintegrate::TestHarness::given([
        DomainEvent::AccountOpened {
            account_id: "some account".into(),
        },
        DomainEvent::AmountDeposited {
            account_id: "some account".into(),
            amount: 10,
        },
    ])
    .when(WithdrawAmount::new("some account".into(), 10))
    .then([DomainEvent::AmountWithdrawn {
        account_id: "some account".into(),
        amount: 10,
    }]);
}

#[test]
fn it_should_not_withdraw_an_amount_when_the_balance_is_insufficient() {
    disintegrate::TestHarness::given([
        DomainEvent::AccountOpened {
            account_id: "some account".into(),
        },
        DomainEvent::AmountDeposited {
            account_id: "some account".into(),
            amount: 10,
        },
        DomainEvent::AmountWithdrawn {
            account_id: "some account".into(),
            amount: 26,
        },
    ])
    .when(WithdrawAmount::new("some account".into(), 5))
    .then_err(Error::InsufficientBalance);
}
```

## Decision Maker

`DecisionMaker` executes decisions and the persistence of resulting events into the event store. It acts as the orchestrator for applying business logic and updating the system state based on the decisions made.

```rust
let decision_maker = disintegrate_postgres::decision_maker(event_store);
decision_maker
    .make(WithdrawAmount::new(id, amount))
    .await?;
```

In this example, the code shows the execution of the `WithdrawAmount` decision.

