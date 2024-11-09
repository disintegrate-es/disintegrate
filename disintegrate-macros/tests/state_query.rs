use disintegrate::{query, Event, StateQuery};

#[allow(dead_code)]
#[derive(Event, Debug, PartialEq, Eq, Clone)]
enum DomainEvent {
    UserCreated {
        #[id]
        user_id: i64,
        name: String,
        email: String,
    },
    OrderCreated {
        #[id]
        user_id: i64,
        #[id]
        order_id: String,
        amount: u32,
    },
}

#[derive(StateQuery, Debug, PartialEq, Eq, Clone)]
#[state_query(DomainEvent)]
struct UserOrders {
    #[id]
    user_id: i64,
}

#[derive(StateQuery, Debug, PartialEq, Eq, Clone)]
#[state_query(DomainEvent, rename = "UserOrderData")]
struct UserOrder {
    #[id]
    user_id: i64,
    #[id]
    order_id: String,
}

#[test]
fn it_sets_the_name_of_a_state_query() {
    assert_eq!(UserOrders::NAME, "UserOrders");
}

#[test]
fn it_renames_a_state_query() {
    assert_eq!(UserOrder::NAME, "UserOrderData");
}

#[test]
fn it_builds_the_stream_query() {
    let user_orders = UserOrders { user_id: 1 };
    assert_eq!(user_orders.query(), query!(DomainEvent; user_id == 1));

    let user_order = UserOrder {
        user_id: 2,
        order_id: "order1".to_string(),
    };
    assert_eq!(
        user_order.query(),
        query!(DomainEvent; user_id == 2, order_id == "order1")
    );
}
