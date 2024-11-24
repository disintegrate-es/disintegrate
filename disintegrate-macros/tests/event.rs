use disintegrate::{ident, DomainIdentifierInfo, Event, IdentifierType, IntoIdentifierValue};

#[derive(Event, Clone, Debug, PartialEq, Eq)]
struct UserUpdatedData {
    #[id]
    user_id: String,
    email: String,
}

#[derive(Event, Clone, Debug, PartialEq, Eq)]
struct UserDeleted {
    #[id]
    user_id: String,
}

#[allow(clippy::enum_variant_names)]
#[derive(Event, Debug, PartialEq, Eq)]
#[stream(UserEvent, [UserCreated, UserUpdated, UserDeleted])]
#[stream(OrderEvent, [OrderCreated, OrderCancelled])]
enum DomainEvent {
    UserCreated {
        #[id]
        user_id: String,
        name: String,
        email: String,
    },
    UserUpdated(UserUpdatedData),
    UserDeleted(Box<UserDeleted>),
    OrderCreated {
        #[id]
        order_id: String,
        amount: u32,
    },
    OrderCancelled {
        #[id]
        order_id: String,
    },
    UserChanged,
}

#[test]
fn it_correctly_sets_event_names() {
    assert_eq!(
        DomainEvent::SCHEMA.events,
        &[
            "UserCreated",
            "UserUpdated",
            "UserDeleted",
            "OrderCreated",
            "OrderCancelled",
            "UserChanged"
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
    assert_eq!(
        domain_identifiers.get(&ident!(#user_id)),
        Some(&user_id.clone().into_identifier_value())
    );

    let enum_unit_variant_event = DomainEvent::UserUpdated(UserUpdatedData {
        user_id: user_id.clone(),
        email: "john@example.com".to_string(),
    });

    let domain_identifiers = enum_unit_variant_event.domain_identifiers();
    assert_eq!(
        domain_identifiers.get(&ident!(#user_id)),
        Some(&user_id.clone().into_identifier_value())
    );

    let enum_boxed_variant_event = DomainEvent::UserDeleted(Box::new(UserDeleted {
        user_id: user_id.clone(),
    }));

    let domain_identifiers = enum_boxed_variant_event.domain_identifiers();
    assert_eq!(
        domain_identifiers.get(&ident!(#user_id)),
        Some(&user_id.into_identifier_value())
    );

    let enum_unit_variant_event = DomainEvent::UserChanged;

    let domain_identifiers = enum_unit_variant_event.domain_identifiers();
    assert!(domain_identifiers.is_empty());
}

#[test]
fn it_generates_event_streams() {
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

    let user_boxed_event = UserEvent::UserDeleted(Box::new(UserDeleted {
        user_id: "user123".to_string(),
    }));

    let user_event: DomainEvent = user_boxed_event.into();
    assert_eq!(
        user_event,
        DomainEvent::UserDeleted(Box::new(UserDeleted {
            user_id: "user123".to_string(),
        }))
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

    assert_eq!(
        UserEvent::SCHEMA.events,
        &["UserCreated", "UserUpdated", "UserDeleted"]
    );

    assert_eq!(
        OrderEvent::SCHEMA.events,
        &["OrderCreated", "OrderCancelled"]
    );
}

#[test]
fn it_generates_domain_identifiers_schema_set() {
    assert_eq!(
        OrderEvent::SCHEMA.domain_identifiers,
        &[&DomainIdentifierInfo {
            ident: ident!(#order_id),
            type_info: IdentifierType::String
        }]
    );

    assert_eq!(
        UserEvent::SCHEMA.domain_identifiers,
        &[&DomainIdentifierInfo {
            ident: ident!(#user_id),
            type_info: IdentifierType::String
        }]
    );

    assert_eq!(
        DomainEvent::SCHEMA.domain_identifiers,
        &[
            &DomainIdentifierInfo {
                ident: ident!(#order_id),
                type_info: IdentifierType::String
            },
            &DomainIdentifierInfo {
                ident: ident!(#user_id),
                type_info: IdentifierType::String
            }
        ]
    );
}
