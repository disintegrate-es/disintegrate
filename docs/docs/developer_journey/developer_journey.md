---
sidebar_position: 2
---

# Developer Journey
Welcome to your journey with **Disintegrate**, where we introduce a unique approach to **Command and Query Responsibility Segregation** (CQRS). Assuming you have some familiarity with CQRS and Event Sourcing, we'll briefly revisit these concepts. If you're new to them, we recommend exploring literature on the topic to gain a better understanding.

When initiating a new project following domain-driven design (DDD) principles, it's advisable to start with an **Event Storming** session. This helps in modeling the system and clarifying business concepts, enabling the identification of major events, commands, and aggregates. Through this session, stakeholders establish a ubiquitous language to refer to domain concepts and flows. Following Event Storming, the team refines the model and identifies aggregates, sub-domains, and bounded contexts.

Aggregates are the primary components to identify—an aggregation of domain objects treated as a single unit. They handle commands and generate events to update the system's state. Defining aggregates is challenging, as they're designed to handle multiple commands, and their state should accommodate all possible use cases.

:::warning
Incorrectly defining aggregates can lead to issues, whether they become too large, creating contention and slowing down the application, or too small, scattering necessary data across multiple aggregates, complicating system consistency. Evolving aggregates is equally challenging, often requiring modifications to the state and impacting existing commands.
:::

**Disintegrate** shifts focus from aggregates to commands (`Decision`s) the system must handle. This shift simplifies the identification of specific commands and eases the introduction of new commands compared to modifying aggregates originally designed for them.

In the following sections, we'll guide you through creating a new application using Disintegrate. We'll start with an event storming session, outline the flow of events and commands, and then move on to coding the system, focusing on evolution and maintenance using the Shopping Cart example. Additionally, we'll explore Disintegrate's architecture, including our Postgres event store and techniques for handling snapshots and migrating Postgres tables, providing insight into Disintegrate's real-world application capabilities.

