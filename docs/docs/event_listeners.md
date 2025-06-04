---
sidebar_position: 5
---

# Event Listener

In event-sourced applications, deriving the state of the system directly from events can be quite expensive. That's why it is usually implemented with the CQRS pattern. The read-side of the application is composed of read models, also known as projections. Disintegrate provides Event Listeners, which allow you to build read models or projections of your stream or integrate your application in an event-driven fashion.

A Projection is a materialized view of the stream optimized for queries. So, it contains aggregate data that can be retrieved by a single SQL query.

If you are building an event-driven application, you should also use an event listener to put events in a queue and integrate with other components in your system, such as email and report systems, or other applications. 
Event listeners can also be employed to integrate components within the application. Traditionally, an event-sourced application involves policies and sagas between aggregates. Disintegrate takes this a step further by often allowing you to bypass complex policies and sagas between `Decision`s simply by querying all the needed events. However, in cases where such integration patterns are critical for reducing contention, Event Listeners can seamlessly step in to implement these components.

In Disintegrate, Event Listeners are independent components that can be deployed along with the write side of your main application or in a standalone one. You can launch a new listener by defining a new `PgEventListener` as follows:

```rust
PgEventListener::builder(event_store)
    .register_listener(
        read_model::ReadModelProjection::new(pool.clone()).await?,
        PgEventListenerConfig::poller(Duration::from_millis(5000)).with_notifier(),
    )
    .start_with_shutdown(shutdown())
    .await
    .map_err(|e| anyhow!("event listener exited with error: {}", e))?;
```
This listener will start to handle all the events defined by the `ReadModelProjection`. The `ReadModelProjection` implements the `EventListener` trait to specify:
* the `id` of the EventListener that will be used by Disintegrate to persist its state in the database
* the `query` method that returns the StreamQuery used to query a subset of events from the event store
* the `handle` method that provides the implementation of the event listener

```rust
pub struct ReadModelProjection {
    query: StreamQuery<PgEventId, DomainEvent>,
    pool: PgPool,
}

impl ReadModelProjection {
    pub async fn new(pool: PgPool) -> Result<Self, sqlx::Error> {
        sqlx::query(
            r#"
        CREATE TABLE IF NOT EXISTS course (
           course_id TEXT PRIMARY KEY,
           name TEXT,
           available_seats INT,
           closed BOOL DEFAULT false,
           event_id BIGINT not null
        )"#,
        )
        .execute(&pool)
        .await?;
        Ok(Self {
            query: query!(DomainEvent),
            pool,
        })
    }
}

#[async_trait]
impl EventListener<i64, DomainEvent> for ReadModelProjection {
    type Error = sqlx::Error;
    fn id(&self) -> &'static str {
        "courses"
    }

    fn query(&self) -> &StreamQuery<PgEventId, DomainEvent> {
        &self.query
    }

    async fn handle(&self, event: PersistedEvent<i64, DomainEvent>) -> Result<(), Self::Error> {
        let event_id = event.id();
        match event.into_inner() {
            DomainEvent::CourseCreated {
                course_id,
                name,
                seats,
            } => {
                sqlx::query(
                    "INSERT INTO course (course_id, name, available_seats, event_id) VALUES($1, $2, $3, $4) ON CONFLICT DO NOTHING",
                )
                .bind(course_id)
                .bind(name)
                .bind(seats as i32)
                .bind(event_id)
                .execute(&self.pool)
                .await
                .unwrap();
            }
            DomainEvent::CourseClosed { course_id } => {
                sqlx::query(
                    "UPDATE course SET closed = true, event_id = $2 WHERE course_id = $1 and event_id < $2",
                )
                .bind(course_id)
                .bind(event_id)
                .execute(&self.pool)
                .await
                .unwrap();
            }
            DomainEvent::StudentSubscribed { course_id, .. } => {
                sqlx::query(
                    "UPDATE course SET available_seats = available_seats - 1, event_id = $2 WHERE course_id = $1 and event_id < $2",
                )
                .bind(course_id)
                .bind(event_id)
                .execute(&self.pool)
                .await
                .unwrap();
            }
            DomainEvent::StudentUnsubscribed { course_id, .. } => {
                sqlx::query(
                    "UPDATE course SET available_seats = available_seats + 1, event_id = $2 WHERE course_id = $1 and event_id < $2",
                )
                .bind(course_id)
                .bind(event_id)
                .execute(&self.pool)
                .await
                .unwrap();
            }
            DomainEvent::CourseRenamed { course_id, name } => {
                sqlx::query("UPDATE course SET name = $2, event_id = $2 WHERE course_id = $1 and event_id < $2")
                    .bind(course_id)
                    .bind(name)
                    .bind(event_id)
                    .execute(&self.pool)
                    .await
                    .unwrap();
            }
            _ => {}
        }
        Ok(())
    }
}
```

The `handle` method processes events one at a time, following the order in which they were written in the event store. Each "user" event arrives wrapped within the `PersistedEvent` struct, carrying metadata such as its event_id. Since the event listener ensures at-least-once delivery guarantee, it's possible for the same event to be delivered multiple times. Consequently, it's crucial to implement the event listener to handle potential duplicate deliveries. In the provided example, the `UPDATE` statements are skipped if the `event_id` is found to be less than the one already stored in the read model, effectively preventing redundant updates.

## Reprojection

In some cases, you might find yourself needing to reproject a read-model, perhaps to incorporate a new column exposing data from your events. In Disintegrate, triggering such a reprojection is remarkably straightforward. In the database, there exists a table named `event_listener`, responsible for storing the last processed ID of an Event Listener. By resetting this ID, the event listener will reprocess events starting from that point:

```sql
update table event_listener set last_processed_event_id = 0 where id = 'my-read-model';
```

Reprojection processes can sometimes be sluggish, taking hours or even days to rebuild the read model from events. To understand the intricacies and potential challenges of reprojection, we recommend watching Dennis Doomen's talk, [Slow Event Sourcing reprojections? Just make them faster!](https://www.youtube.com/watch?v=EqVPqInQ6YM).

When reprojecting takes a significant amount of time, employing techniques to prevent outages becomes important. One such technique involves constructing a new read model concurrently and then transitioning the code to query the new read model once the reprojection is complete. This ensures uninterrupted service, allowing the application to continue serving the old projection until the new one is ready.


:::tip
To run your `PgEventListener` as a **background task** that does **not block** your Actix-web routes, you should spawn it in a separate asynchronous task using `tokio::spawn` or `actix_web::rt::spawn` since Actix uses Tokio under the hood.
This lets your server and HTTP handlers start **immediately**, while the event listener runs in the background and continues until the shutdown signal.

:::

```rust
// example for actix-web
    let listener_event_store = event_store.clone();
    let listener_pool = shared_pool.clone();

    tokio::spawn(async move {
        let listener = match ReadModelProjection::new(listener_pool).await {
            Ok(listener) => listener,
            Err(e) => {
                error!("Failed to create ReadModelProjection: {}", e);
                return;
            }
        };

        if let Err(e) = PgEventListener::builder(listener_event_store)
            .register_listener(
                listener,
                PgEventListenerConfig::poller(Duration::from_millis(5000)).with_notifier(),
            )
            .start_with_shutdown(shutdown())
            .await
        {
            error!("event listener exited with error: {}", e);
        }
    });
```