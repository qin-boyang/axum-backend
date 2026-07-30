# Axum + SQLx + SQLite CRUD API

## Run

```bash
cargo run
```

The server listens on `http://localhost:3000` and stores data in `app.db`.
Set `DATABASE_URL` to use another SQLite database.

The API processes at most 100 requests concurrently by default. Additional
requests wait asynchronously until capacity is available. Change the limit with:

```bash
MAX_CONCURRENT_REQUESTS=200 cargo run
```

## Endpoints

| Method | Path | Description |
| --- | --- | --- |
| POST | `/todos` | Create a todo |
| GET | `/todos` | List todos |
| GET | `/todos/{id}` | Get one todo |
| PUT | `/todos/{id}` | Update a todo |
| DELETE | `/todos/{id}` | Delete a todo |

Create request:

```json
{ "title": "learn Rust" }
```

Update request:

```json
{ "title": "learn Axum", "completed": true }
```

## Test

The integration tests act as an HTTP client, so start the server first:

```bash
cargo test
```

Run the CRUD requests separately and print their JSON responses:

```bash
cargo test --test api_test create -- --nocapture
cargo test --test api_test read -- --nocapture
cargo test --test api_test update -- --nocapture
cargo test --test api_test delete -- --nocapture
```

The default server URL is `http://localhost:3000`. To target another server:

```bash
BASE_URL=http://localhost:8080 cargo test --test api_test -- --nocapture
```

## Load test

With the API already running, measure `GET /todos` throughput:

```bash
cargo test --test load_test -- --ignored --nocapture
```

The default load is 500 concurrent clients for 10 seconds. Configure it with:

```bash
LOAD_TEST_CONCURRENCY=200 \
LOAD_TEST_DURATION_SECS=30 \
MIN_QPS=1000 \
MAX_P99_MS=100 \
cargo test --test load_test -- --ignored --nocapture
```

The report includes average, p50, p95, p99, and maximum latency. `MIN_QPS` and
`MAX_P99_MS` are optional performance assertions.
