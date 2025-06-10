---
sidebar_position: 4
---

# Handling Concurrecy

If your system experiences high usage due to a rush of clients eager to use your new, shiny coupon feature, conflicts may arise because coupons will be concurrently utilized by multiple users. In Disintegrate, this means the `Coupon` state might become outdated for some users when decisions are made, leading to conflicts that need to be resolved by trying the decision again later. However, such challenges are not unique to Disintegrate and are also encountered in traditional aggregate-based systems.

Various approaches exist to address this situation. One option is to introduce a lock mechanism that grants exclusive access to Coupon resources, allowing only one client at a time to decrement the counter. Alternatively, employing a queue can ensure commands with the same coupon ID are executed sequentially, either through dedicated queues for each coupon ID or by grouping commands with the same coupon into partitions if Kafka is utilized.

Another solution involves allowing overbooking, if permissible by your business model. This approach simplifies the system by permitting a limited amount of overbooking. In Disintegrate, implementing this is straightforward: using the `ValidationQuery` to exclude `CouponApplied` events during conflict checking before decision is commited. While still verifying availability in decisions by loading Coupon state, the system does not check for Coupon state staleness. Consequently, concurrent operations may proceed through conflict checks and commit decisions even when the coupon is no longer available. The following example shows you how to exclude `CouponApplied` from the conflict validation and accept overbooking of coupons: 

```rust
impl Decision for ApplyCoupon {
    type Event = DomainEvent;
    type StateQuery = (Cart, Coupon);
    type Error = CartError;

    fn state_query(&self) -> Self::StateQuery {
        (Cart::new(&self.user_id), Coupon::new(&self.coupon_id))
    }

    fn process(&self, (cart, coupon): &Self::StateQuery) -> Result<Vec<Self::Event>, Self::Error> {
        // business logic
        todo!()
    }

    fn validation_query<ID: disintegrate::EventId>(&self) -> Option<StreamQuery<ID, Self::Event>> {
        let (cart, coupon) = self.state_query();
        // the validation query is the union of the two state queries used by the decision
        Some(union!(
            //the original cart state query will be used to validate the decision against user's cart changes.
            &cart,
            // exclude the `AppliedCoupon` event from the coupon state query to allow some overbooking.
            coupon.exclude_events(event_types!(DomainEvent, [CouponApplied]))
        ))
    }
}
```