# Feature

- Feature Name: im-backend
- Start Date: 2024-03-21

## Summary

A im application backend.

## Motivation

first property: study rust.

## Guide-level explanation

## Reference-level explanation

- abi
  all foundations include: domain models, configuration, etc.
- chat
  business logic
- ws
  message gateway, to handle the message of client sent and send message to client by websocket.
- rpc
  handle the rpc request from other services.
- api
  offer api for other services to call. based on rpc.
- db
  database layer, consumer the mq data to postgres, mongodb and reds.
  ![core flow](images/时序图.awebp)

## Drawbacks

Why should we *not* do this?

## Rationale and alternatives

- Why is this design the best in the space of possible designs?
- What other designs have been considered and what is the rationale for not choosing them?
- What is the impact of not doing this?
- If this is a language proposal, could this be done in a library or macro instead? Does the proposed change make Rust
  code easier or harder to read, understand, and maintain?

## Unresolved questions

- save the message sequence to redis: we don't know if the seq is correct, we just increase it now
- tonic grpc client load balance: it's just basic load balance for now, and it doesn't implement the get new service
  list in the interval time.
- need to design a websocket register center, to achieve the load balance.
- shall we put the method that we get members id from cache into db service?
- friendship need to redesign
- conversation has nothing yet, it's only on the client side.
- partition table for message have not been implemented yet.
- GROUP MESSAGE SEQUENCE: WE INCREASE THE SEQUENCE AT CONSUMER MODULE, AND NEED TO GET SEQUENCE AT WS/MONGODB MODULE. IS
  THERE ANY EFFECTIVE WAY TO PERFORMANT?
- timestamp issue: we use the time millis(i64) as the timestamp in database, but we should use the TimeStamp in the
  future.

## Future possibilities

Think about what the natural extension and evolution of your proposal would
be and how it would affect the language and project as a whole in a holistic
way. Try to use this section as a tool to more fully consider all possible
interactions with the project and language in your proposal.
Also consider how this all fits into the roadmap for the project
and of the relevant sub-team.

This is also a good place to "dump ideas", if they are out of scope for the
RFC you are writing but otherwise related.

If you have tried and cannot think of any future possibilities,
you may simply state that you cannot think of anything.

Note that having something written down in the future-possibilities section
is not a reason to accept the current or a future RFC; such notes should be
in the section on motivation or rationale in this or subsequent RFCs.
The section merely provides additional information.
