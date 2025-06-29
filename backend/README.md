# Backend Setup

## 1. Environment Configuration

1. Copy the example environment file:
   ```sh
   cp .env.example .env
   ```
2. Edit `.env` if needed to match your local setup (the default should work with Docker Compose).

## 2. Start the Database

Start the PostgreSQL database using Docker Compose:
```sh
docker compose up -d
```
This will run the database in the background.

## 3. Run the Backend

Start the backend server locally:
```sh
cargo run
```
You can also use cargo watch which works like nodemon:
```sh
cargo watch -x run
```

The backend will connect to the database running in Docker.

## 4. Stopping Services

To stop the database:
```sh
docker compose down
```

---

**Note:**
- Make sure Docker is running before starting the database.
- The `.env` file must match the credentials in `docker-compose.yml`.
