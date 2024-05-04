---
sidebar_position: 1
---

# Event Storming
Event storming is a collaborative workshop technique used to explore and model complex business domains. Let's consider a shopping cart application as an example. During an event storming session for the shopping cart system, we identify various events, commands, and entities:
* Events: `ItemAdded`, `ItemRemoved`, `CartCheckedOut`, ...
* Commands: `AddItem`, `RemoveItem`, `CheckoutCart`, ...
* Entities: `Products`, `Customers`, `Carts`, `Orders`, ...


## Example:
During the event storming session, we map out the flow of events and commands within the shopping cart system. For instance, the `AddItem` command triggers the `ItemAdded` event, updating the contents of the user's shopping cart. Similarly, the `CheckoutCart` command results in the `CartCheckedOut` event, indicating the completion of a purchase.

### Events, Identifiers, and Commands
In the context of the shopping cart system:
* **Events**: Signify important actions within the shopping cart, such as adding items, removing items, applying coupons, and completing purchases.
* **Identifiers**: Uniquely distinguish entities like products, customers, coupons, and orders, ensuring precise tracking and management.
* **Commands**: User-initiated actions that alter the state of the shopping cart, including adding items, removing items, applying coupons, and initiating checkout processes.


## Example:

When a customer adds an item to their shopping cart, the system generates an `ItemAdded` event, capturing details such as the item ID, quantity, and timestamp. This event triggers the updating of the shopping cart's contents, ensuring an accurate reflection of the user's selections. During an event storming session, you identify the aggregates that define transaction boundaries within the system. Each aggregate encapsulates related entities and actions, promoting consistency in data updates.
In the shopping cart scenario, identifying aggregates may involve delineating entities like the shopping cart itself and items. Each aggregate's state should include only data pertinent to the commands it handles, minimizing unnecessary memory usage.

