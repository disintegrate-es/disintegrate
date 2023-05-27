# Courses Example

The Courses Example is a sample project that demonstrates the usage of the Disintegrate library. It represents a fictional school system where users can register and subscribe to courses. The example showcases how Disintegrate can be utilized to manage events and implement business logic in an event-driven architecture.

## Project Overview

The project focuses on the following key aspects:

* **Student Registration**: Students can register in the system by providing their information such as name and student ID.

* **Course Subscription**: Students can subscribe to available courses. Each course has a limited number of seats, and students can only subscribe to a maximum of 2 courses.

* **Event-driven Architecture**: The project leverages Disintegrate to implement an event-driven architecture.

## Getting Started

To run the Courses Example, follow these steps:

1. Clone the Disintegrate repository from GitHub: `git clone https://github.com/disintegrate-es/disintegrate.git`

2. Navigate to the `examples/courses` directory: `cd disintegrate/examples/courses`

3. Start the required services using Docker Compose: `docker compose up -d`

4. Run the example using Cargo: `cargo run`

## Example Usage

In the test folder, you can find an Insomnia collection named `course_api_insomnia_collection.json` that contains a set of requests for testing the gRPC API.

To use the Insomnia collection:

1. Import the collection into Insomnia. You can use the Insomnia Desktop App to import the collection.
2. Once imported, you will find the collection with various requests.

### Requests

Here are some of the requests available in the collection:

* **Create Course**: Creates a new course with the specified ID, name, and number of seats.
* **Close Course**: Closes a course with the specified ID, preventing further subscriptions.
* **Rename Course**: Renames a course with the specified ID.
* **Register Student**: Registers a new student with the specified ID and name.
* **Subscribe Student**: Subscribes a student to a course with the specified student ID and course ID.
* **Unsubscribe Student**: Unsubscribes a student from a course with the specified student ID and course ID.

To use these requests, make sure the application is running, and update the request bodies with the desired parameters.
