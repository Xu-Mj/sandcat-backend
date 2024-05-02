# IM Backend Architecture Description

## Project Overview

This project provides an implementation of a backend for an Instant Messaging (IM) system, developed in the Rust programming language, with a microservice architecture design. Its primary goal is to offer developers an efficient, stable, and scalable IM solution. The focus of this project is to explore and demonstrate the potential and practices of Rust in building high-performance backend services.

## Key Features

- **Microservice Architecture:** The system is split into multiple independent service units, each responsible for a portion of the core business logic and communication with other services.
- **Containerized Deployment:** All services can be packaged with Docker, facilitating deployment and management.
- **Asynchronous Processing:** Utilizes Rust's asynchronous programming capabilities to handle concurrent workloads, enhancing performance and throughput.
- **Data Storage:** Uses PostgreSQL and MongoDB for storing messages permanently and for inbox functionalities, respectively.
- **Message Queue:** Leverages Kafka as a message queue to support high concurrency message pushing and processing.

## Architecture Components

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



## Performance and Scalability

   The project is designed with high performance and horizontal scalability in mind. Through asynchronous processing and a microservice architecture, the system is capable of scaling effectively by increasing the number of service instances in response to the growing load. Additionally, the project adopts a modular design philosophy that allows developers to customize or replace modules as needed.

## Unresolved questions

- **Storing Message Sequences in Redis**: We currently increment the sequence numbers simply, without confirming their accuracy. There's a need to ensure the correctness of this sequence.
- **Handling Message Sequences on Database Module Failure**: When sending messages to the database module fails, we require a mechanism to handle message sequences accordingly.
- **Integrating Member ID Retrieval from Cache into DB Service**: Whether the method for retrieving member IDs from the cache should be integrated into the DB service is under consideration.
- **Friendship Redesign**: The current design for representing friendships is inadequate and requires a thorough redesign.
- **Conversation Feature**: There is currently no implementation of conversations on the server-side, as it exists only client-side.
- **Partition Table for Messages Not Implemented**: The strategy for implementing partitioned tables for messages has not been realized yet.
- **Group Message Sequencing**: The sequence for group messages is incremented at the consumer module, and we need to obtain the sequence in the WebSocket/mongoDB module. Is there a more effective way to do this?
- **Timestamp Issues**: We use Unix time milliseconds (i64) as the timestamp in the database, but we should use a `TimeStamp` type in the future.
- **Axum's Routes Layer or With_State?**: Should we utilize Axum's routes layer with state or the `with_state` method?
- **User Table Should Add Login Device Field**: There should be consideration to add a field for the login device to the user table, which is used to check if clients need to sync the friend list.
- **Friendship Read Status**: we should delete the Friendship related message after user read it.

## Development

1. clone the project

   ```shell
   git clone
   cd sandcat-backend
   ```

2. install `librdkafka`

   **Ubuntu：**

   ```shell
   apt install librdkafka-dev
   ```

   **Windows:**

   this is a painful task...

   ```shell
   # install vcpkg
   git clone https://github.com/microsoft/vcpkg
   cd vcpkg
   .\bootstrap-vcpkg.bat
   # Install librdkafka
   vcpkg install librdkafka
   .\vcpkg integrate install
   ```

   **important:** restart your command console to make sure the environment variable has been modified successfully.

   I'm struggling to configure the `cmake-build` feature for `rdkafka`. If anyone has experience with this, your guidance would be greatly appreciated.

   so let's use the `dynamic-linking` feature

   **modify rdkafka dependence**

   ```shell
   cargo add rdkafka --features dynamic-linking -p consumer
   cargo add rdkafka --features dynamic-linking -p chat
   ```
   If you encounter problems during compilation, please try manually removing the cmake-build feature and then attempt to compile again.

3. run docker compose

   ```shell
   docker-compose up -d
   ```

   **important:** make sure all the third service are running in docker.

4. install sqlx-cli and init the database

   ```shell
   cargo install sqlx-cli
   sqlx migrate run
   ```

5. build

   ```shell
   cargo build --release
   ```

6. copy the binary file to root path

   ```shell
   cp target/release/cmd ./sandcat
   ```

7. run

   ```shell
   ./sancat
   ```

   if you need adjust some configuration, please modify the `config.toml`

**important:** Given that our working environment may differ, should you encounter any errors during your deployment, please do let me know. Together, we'll work towards finding a solution.
