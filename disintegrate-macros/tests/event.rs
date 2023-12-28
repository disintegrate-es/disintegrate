use disintegrate::{ident, Event};

#[derive(Event, Clone, Debug, PartialEq, Eq)]
struct UserUpdated {
    #[id]
    user_id: String,
    email: String,
}

#[derive(Event, Debug, PartialEq, Eq)]
#[group(UserEvent, [UserCreated, UserUpdated])]
#[group(OrderEvent, [OrderCreated, OrderCancelled])]
enum DomainEvent {
    UserCreated {
        #[id]
        user_id: String,
        name: String,
        email: String,
    },
    UserUpdated(UserUpdated),
    OrderCreated {
        #[id]
        order_id: String,
        amount: u32,
    },
    OrderCancelled {
        #[id]
        order_id: String,
    },
}

#[test]
fn it_correctly_sets_event_names() {
    assert_eq!(
        DomainEvent::SCHEMA.types,
        &[
            "UserCreated",
            "UserUpdated",
            "OrderCreated",
            "OrderCancelled"
        ]
    );
}

#[test]
fn it_returns_correct_domain_identifiers() {
    let user_id = "user123".to_string();
    let enum_struct_variant_event = DomainEvent::UserCreated {
        user_id: user_id.clone(),
        name: "John Doe".to_string(),
        email: "john@example.com".to_string(),
    };

    let domain_identifiers = enum_struct_variant_event.domain_identifiers();
    assert_eq!(domain_identifiers.get(&ident!(#user_id)), Some(&user_id));

    let enum_unit_variant_event = DomainEvent::UserUpdated(UserUpdated {
        user_id: user_id.clone(),
        email: "john@example.com".to_string(),
    });

    let domain_identifiers = enum_unit_variant_event.domain_identifiers();
    assert_eq!(domain_identifiers.get(&ident!(#user_id)), Some(&user_id));
}

#[test]
fn it_generates_event_groups() {
    let user_event = UserEvent::UserCreated {
        user_id: "user123".to_string(),
        name: "John Doe".to_string(),
        email: "john@example.com".to_string(),
    };

    let user_event: DomainEvent = user_event.into();
    assert_eq!(
        user_event,
        DomainEvent::UserCreated {
            user_id: "user123".to_string(),
            name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
        }
    );

    let order_event = OrderEvent::OrderCreated {
        order_id: "order456".to_string(),
        amount: 100,
    };

    let order_event: DomainEvent = order_event.into();
    assert_eq!(
        order_event,
        DomainEvent::OrderCreated {
            order_id: "order456".to_string(),
            amount: 100,
        }
    );

    assert_eq!(UserEvent::SCHEMA.types, &["UserCreated", "UserUpdated"]);

    assert_eq!(
        OrderEvent::SCHEMA.types,
        &["OrderCreated", "OrderCancelled"]
    );
}

#[test]
fn it_generates_domain_identifiers_schema_set() {
    assert_eq!(OrderEvent::SCHEMA.domain_identifiers, &["order_id"]);

    assert_eq!(UserEvent::SCHEMA.domain_identifiers, &["user_id"]);

    assert_eq!(
        DomainEvent::SCHEMA.domain_identifiers,
        &["order_id", "user_id"]
    );
}
