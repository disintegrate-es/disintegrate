---
sidebar_position: 3
---

# Add New Features

## Old Approaches vs Disintegrate
Traditional approaches to adding new features in a shopping cart application might involve tightly coupling components, making it challenging to extend or modify the system. However, by adopting Disintegrate, developers can leverage its modular and decoupled architecture to seamlessly integrate new features.

### Example:
Consider the scenario where we aim to enhance our shopping cart application with a new feature: coupons. The business intends to provide customers with a limited number of coupons that they can apply to their purchases. However, traditional aggregate-based implementation of this feature presents challenges, primarily due to the coordination required between multiple aggregates.

Let's imagine we already have a `ShoppingCart` aggregate in our system (there are various ways to design this system, but for simplicity's sake, let's assume we have this aggregate). Now, we could opt to introduce a new aggregate named Coupon to manage the availability of a coupon identified by its unique ID. In the following section, we'll delve into the challenges posed by this approach and how Disintegrate simplifies the integration of this new feature.

## Challenges with Aggregates
To implement coupons using aggregates, we may introduce a new aggregate for coupons to track their availability. When a customer proceeds to checkout, they can choose to apply a coupon. This introduces complexity as we need to ensure that the coupon is available before applying it and update its availability afterward. This may require the following considerations:

* Introduction of a New Aggregate: To manage coupons effectively, a new aggregate should be introduced to track their availability. It's essential to ensure that coupons are available before they are applied and to update their availability afterward.

* Workflow Management: Introducing a `Coupon` aggregate necessitates efficient workflow management between the `Cart` and `Coupon` aggregates. Policies must be devised to coordinate actions such as updating coupon availability and applying coupons during checkout seamlessly.

* Implementation of Compensation Mechanisms: It may be necessary to implement compensation mechanisms or two-phase commit protocols to ensure consistency in case of failures or errors during coupon application. This will help maintain integrity and reliability in coupon management processes.

* Integration of Recovery Mechanisms: Robust recovery mechanisms should be integrated into the system to handle system crashes or failures effectively. These mechanisms ensure that the system can restore itself to a consistent state upon restart, preventing loss or inconsistency between aggregates.

## Disintegrate Approach

With Disintegrate, this new feature can be integrated much more easily. One way to achieve this in Disintegrate is to add a new state that represents coupon availability. We can call this new state Coupon:

```rust
#[derive(Debug, Clone, StateQuery, Default, Deserialize, Serialize)]
#[state_query(CouponEvent)]
struct Coupon {
   #[id]
   coupon_id: String,
   quantity: i32
}
```

Then, in the `ApplyCoupon` decision, we can use this new state query to retrieve the coupon state and check if the coupon is available:

```rust
impl StateMutate for Coupon {
    fn mutate(&mut self, event: Self::Event) {
        match event {
            CouponEvent::CouponEmitted { quantity, .. } => self.quantity += quantity,
            CouponEvent::CouponApplied { .. } => self.quantity -= 1,
        }
    }
}

pub struct ApplyCoupon {
    user_id: String,
    coupon_id: String,
}

impl ApplyCoupon {
    #[allow(dead_code)]
    pub fn new(user_id: String, coupon_id: String) -> Self {
        Self { user_id, coupon_id }
    }
}

impl Decision for ApplyCoupon {
    type Event = DomainEvent;
    type StateQuery = (Cart, Coupon);
    type Error = CartError;

    fn state_query(&self) -> Self::StateQuery {
        (Cart::new(&self.user_id), Coupon::new(&self.coupon_id))
    }

    fn process(&self, (cart, coupon): &Self::StateQuery) -> Result<Vec<Self::Event>, Self::Error> {
        if cart.applied_coupon.is_some() {
            return Err(CartError::CouponAlreadyApplied);
        }
        if coupon.quantity == 0 {
            return Err(CartError::CouponNotAvailable);
        }
        Ok(vec![DomainEvent::CouponApplied {
            coupon_id: self.coupon_id.clone(),
            user_id: self.user_id.clone(),
        }])
    }
}
```


