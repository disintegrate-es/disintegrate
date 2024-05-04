---
sidebar_position: 2
---

# CQRS 

The development team has decided to adopt **CQRS** (Command Query Responsibility Segregation), a design pattern that advocates for the separation of concerns between commands (actions that change the system's state) and queries (actions that retrieve data from the system). This separation allows for independent scaling, optimization, and maintenance of both the write and read sides of an application.

## Write Model vs Read Model

### Write Model
The write model represents the system's state as it changes over time in response to commands. It encapsulates the business logic and validation rules necessary to process incoming commands. Additionally, the team has chosen to persist the state of the Shopping Cart using event sourcing.

### Read Model
On the other hand, the read model is optimized for querying and presenting data to users. It stores denormalized or precomputed views of the data tailored to specific use cases, improving performance and scalability for read-heavy workloads. In this case, the team has decided to create a table containing the items added to a Shopping Cart. Each user has their own shopping cart, identified by their user ID.

## Event Sourcing
Event sourcing is a pattern closely related to CQRS, where the state of the application is determined by a sequence of immutable events. Instead of directly modifying the state of entities, actions are represented as domain events that are appended to an event log. The current state of the system is then derived by replaying these events.

### Advantages of Event Sourcing

- **Full Auditability**: Since every change to the system is captured as an event, event sourcing provides a complete audit trail of actions taken within the application, enabling detailed historical analysis and debugging.

- **Temporal Querying**: Event sourcing allows for querying the state of the system at any point in time by replaying events up to that moment. This temporal querying capability is useful for implementing features like versioning, time travel, and historical reporting.

- **Improved Resilience**: By decoupling state mutation from state storage, event sourcing enhances the resilience of the system against failures and data corruption. In the event of a failure, the system can be easily restored by replaying events from the event log.

