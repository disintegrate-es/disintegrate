---
sidebar_position: 6
---

# PostgreSQL Event Store 

Currently, Disintegrate exclusively supports PostgreSQL database implementation. This section offers insights into how Disintegrate interacts with PostgreSQL, focusing on managing application changes and handling data migrations.

## Postgres Database Schema

Disintegrate automatically generates the following tables when a PostgreSQL event source is created:

* **Event:** Stores all events within the event stream.
  * `event_id`: Global identifier of the event.
  * `event_type`: Type of the event.
  * `payload`: Contains the event's payload.
  * `inserted_at`: Timestamp indicating when the event was written (in UTC time).
  * "Domain identifier" columns: Automatically created by the library when a field in the `Event` is marked as `#[id]`, used for indexing and query optimization.

* **Event Sequence:** This technical table is crucial for implementing optimistic locking and managing conflicts.
  * `event_id`: Generates globally unique identifiers for events sequentially.
  * `event_type`: Specifies the type of the appended event.
  * `consumed`: Boolean column used by the optimistic locking logic.
  * `committed`: Indicates if the event has been written into the event stream.
  * `inserted_at`: Timestamp indicating when the event was written (in UTC time).
  * "Domain identifier" columns: Automatically created by the library when a field in the `Event` is marked as `#[id]`, used for indexing and query optimization.

* **Event Listener:** Maintains records of the last event processed for each listener:
  * `id`: Identifier of the event listener.
  * `last_processed_id`: ID of the last event processed by the event listener.
  * `updated_at`: Timestamp indicating the last time the table was updated.

* **Snapshot:** Stores stream query payloads to speed up loading:
  * `id`: Identifier of the stream query.
  * `name`: Name of the stream query.
  * `query`: String representation of the query.
  * `version`: Last event ID processed by the stream query.
  * `payload`: Payload of the stream query.
  * `inserted_at`: Timestamp indicating the last time the row was inserted.

## Append Events

The append API of the event stream requires three arguments:

* List of new events to be appended.
* Stream query to check if new events have been appended to the event store that would make stale the state used to make the decision.
* `last_event_id` retrieved during the query.

The append process and optimistic lock unfold as follows:

The library adds a row to the event_sequence table for each new event, serializing all writes to reserve a spot for the new events in the stream. It then attempts to update the consumed field to "1" for all events matching the query from the last_event_id to the last inserted event_id. This operation marks all pending events of other concurrent appends as invalidated. If this update fails due to either:

* Another concurrent process invalidating one or more of the new events
* A new event being written that matches the query

a concurrency error is raised, indicating that the state used by the Decision is stale. If the update succeeds, it means events invalidating this decision did not occur, and the new events can be written to the events table.

## Query Events

The query API requires a `StreamQuery` to fetch data from the `event` table, enabling the search and filtering of events based on specified criteria. Domain identifiers are stored in a dedicated column, and indexed to optimize query operations. The library autonomously adds domain identifier columns when an `Event` field is tagged with the `#[id]` attribute. To properly manage the addition and removal of domain identifiers, consult the data migration section.

## Data Migration

Manual data migration is may be needed when the following changes are made to the event structure:

1. **Adding a New Domain Identifier**: If you want to search old event with this new id, a data migration is required to populate the new id column in the existing events.
2. **Declaring an Existing Field as a Domain Identifier**: Migration is necessary to populate this identifier for old events. Even if the event payload contains the domain identifier value, Disintegrate does not automatically populate the domain identifier column for the already persisted events.
3. **Deleting an Existing Domain Identifier**: Disintegrate does not automatically remove the domain identifier column.  This deliberate design choice is made to support a blue-green rollout strategy.

:::warning
For cases 2 and 3, automation may be provided by the library in the future. Currently, users of the library need to manually make these changes in the database using SQL scripts.
:::

## Snapshots

If snapshotting is enabled, the library saves snapshots of stream queries in the `snapshot` table. Snapshots can be configured to store the result of a query at specified intervals, with the frequency determined by the number of events retrieved from the event store.

```rust
let decision_maker =
        disintegrate_postgres::decision_maker_with_snapshot(event_store.clone(), 10).await?;
```

The library can automatically discard a snapshot under certain conditions:
- Changes are made to the queries used to build it.
- The library cannot deserialize the snapshot due to changes in the query state shape.
  - Addition of new fields to the state query.
  - Changes in the data type of existing fields.

:::warning
 There may be situations where the output stays the same even though the computation underneath has changed. For example, a field of type `i32` may still exist but its calculation method has been altered. In such cases, you'll need to manually delete the snapshot.
 :::