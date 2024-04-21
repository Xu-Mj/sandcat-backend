## IM Backend Architecture Description

### Project Overview

This project provides an implementation of a backend for an Instant Messaging (IM) system, developed in the Rust programming language, with a microservice architecture design. Its primary goal is to offer developers an efficient, stable, and scalable IM solution. The focus of this project is to explore and demonstrate the potential and practices of Rust in building high-performance backend services.

### Key Features

- **Microservice Architecture:** The system is split into multiple independent service units, each responsible for a portion of the core business logic and communication with other services.
- **Containerized Deployment:** All services can be packaged with Docker, facilitating deployment and management.
- **Asynchronous Processing:** Utilizes Rust's asynchronous programming capabilities to handle concurrent workloads, enhancing performance and throughput.
- **Data Storage:** Uses PostgreSQL and MongoDB for storing messages permanently and for inbox functionalities, respectively.
- **Message Queue:** Leverages Kafka as a message queue to support high concurrency message pushing and processing.

### Architecture Components

1. **Service Layer**

   - **Authentication Service:** Handles user registration, login, and verification.
   - **Message Service:** Responsible for message sending, receiving, and forwarding.
   - **Friend Service:** Manages the user's friends list and status.

   - **Group Service:** Takes care of group creation, message broadcasting, and member management.

2. **Data Storage Layer**

   - **PostgreSQL:** Storing user information, friendship relations, and message history, along with automated archival through scheduled tasks.
   - **MongoDB:** Acts as a message inbox, handling offline message storage and retrieval.

3. **Middleware Layer**

   - **Kafka:** Provides a high-throughput message queue to decouple services.
   - **Redis:** Implements caching and maintains message status to optimize database load.

4. **Infrastructure Layer**

   - **Docker and Docker-Compose:** Containers for building and deploying services.
   - **Consul:** For service registration and discovery.
   - **MinIO:** An object storage solution for handling file uploads and downloads.



### Performance and Scalability

   The project is designed with high performance and horizontal scalability in mind. Through asynchronous processing and a microservice architecture, the system is capable of scaling effectively by increasing the number of service instances in response to the growing load. Additionally, the project adopts a modular design philosophy that allows developers to customize or replace modules as needed.

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
