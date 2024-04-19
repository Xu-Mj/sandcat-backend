# Feature

- Feature Name: im-backend
- Start Date: 2024-03-21

## Summary

A im application backend.

## Motivation

first purpose: study rust.

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
  ![core flow](images/seq-chart.png)

## Unresolved questions

- save the message sequence to redis: we don't know if the seq is correct, we just increase it now.
- how to handle the message sequence when the message is sent to the database module failed.
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
- axum's routes layer or with_state?
- user table should add login device, used to check if the client need to sync the friend list

## Future possibilities
