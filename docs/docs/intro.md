---
slug: /
sidebar_position: 1
---

# Disintegrate Docs
**Welcome to Disintegrate!** 

**Disintegrate** was inspired by Sare Pellegring's talk, ["Kill the Aggregate,"](https://www.youtube.com/watch?v=DhhxKoOpJe0) which pointed out the challenge of identifying the right aggregates at the beginning of a project. This resonated with us because, from our experience with aggregate and event sourcing applications, we've learned that new features often require accessing information from multiple aggregates. This issue is also well explained in Gregory Young's book, ["Versioning in an Event Sourced System,"](https://leanpub.com/esversioning/read) especially in the chapter titled ["Stream Boundaries Are Wrong."](https://leanpub.com/esversioning/read#leanpub-auto-stream-boundaries-are-wrong) Young discusses the need to potentially **split or join streams** (which is a broader term than aggregates) to meet changing requirements. This splitting or joining can be done at the storage level by duplicating events to create new streams (for split-stream scenarios) or combining streams (for join-stream scenarios) or dynamically by allowing the application to read multiple streams, as explained in the book's section on ["Cheating."](https://leanpub.com/esversioning/read#leanpub-auto-cheating)

What **Disintegrate** provides is an implementation of the ["Cheating"](https://leanpub.com/esversioning/read#leanpub-auto-cheating) chapter of Young's book, offering a way to query the event store to split or join multiple streams dynamically to create states for decision-making. We've developed a mechanism that simplifies writing applications, making it easy to extend them to accommodate new features without imposing strict boundaries on the streams.
