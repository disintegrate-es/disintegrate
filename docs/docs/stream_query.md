---
sidebar_position: 3
---

# Stream Query

A stream query allows querying events from the event store. When using Disentegrate, you can write a stream query and submit it to the event store to retrieve events. Alternatively, if you define a state query by deriving `StateQuery`, the library will generate a stream query for you. This query selects events from the specified stream and utilizes annotated id fields.

For example:

```rust
#[derive(Debug, StateQuery, Clone, Serialize, Deserialize)]
#[state_query(CourseEvent)]
pub struct Course {
    #[id]
    course_id: CourseId,
    name: String,
    created: bool,
    closed: bool,
}
```

Under the hood, the library generates a stream query that fetches all events from the `CourseEvent` stream, filtering only those with the `course_id` specified in an instance of the `Course` struct.

How does it work if the stream has more than one ID, and you specify only a subset?

If the events included in your stream have multiple IDs, filtering for only a subset of those IDs will result in the query retrieving all the events that match the specified IDs while ignoring the others. For instance, if we filter only for the `course_id`, but the event `StudentSubscribed` has the `student_id`, it will still be selected if the `course_id` matches the one specified in the query.

## Multi State query

Disintegrate automatically implements `StateQuery` for a tuple of `StateQuery`. The stream query of the tuple comprises the union of all its queries: the library retrieves all the queried events and mutates the `StateQuery`s in the tuple based on the specified filters. This feature is particularly useful for reusing the same query for multiple `Decision`s by combining shared `StateQuery`s in complex queries.

```rust
#[derive(Default, StateQuery, Clone, Serialize, Deserialize)]
#[state_query(CartEvent)]
pub struct Cart {
    #[id]
    user_id: String,
    items: HashSet<Item>,
    applied_coupon: Option<String>,
}

impl Cart {
    pub fn new(user_id: &str) -> Self {
        Self {
            user_id: user_id.into(),
            ..Default::default()
        }
    }
}

#[derive(Default, StateQuery, Clone, Serialize, Deserialize)]
#[state_query(CouponEvent)]
pub struct Coupon {
    #[id]
    coupon_id: String,
    quantity: u32,
}

impl Coupon {
    pub fn new(coupon_id: &str) -> Self {
        Self {
            coupon_id: coupon_id.to_string(),
            ..Default::default()
        }
    }
}

fn state_query(&self) -> Self::StateQuery {
    // returns a MultiStateQuery wich is the union of the Cart and Coupon `StateQuery`s
    (Cart::new(&self.user_id), Coupon::new(&self.coupon_id))
}
```
