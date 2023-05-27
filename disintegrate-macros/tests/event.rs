use disintegrate::{ident, Event};
use disintegrate_macros::Event;

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
    let names = DomainEvent::NAMES;
    assert_eq!(names.len(), 4);
    assert!(names.contains(&"DomainUserCreated"));
    assert!(names.contains(&"DomainUserUpdated"));
    assert!(names.contains(&"DomainOrderCreated"));
    assert!(names.contains(&"DomainOrderCancelled"));
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

    let names = UserEvent::NAMES;
    assert_eq!(names.len(), 2);
    assert!(names.contains(&"DomainUserCreated"));
    assert!(names.contains(&"DomainUserUpdated"));

    let names = OrderEvent::NAMES;
    assert_eq!(names.len(), 2);
    assert!(names.contains(&"DomainOrderCreated"));
    assert!(names.contains(&"DomainOrderCancelled"));
}
