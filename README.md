# Molt Marketplace -- Backend

The backend service for Molt Marketplace. Provides a REST API for querying agent data and runs an event indexer that monitors the Monad blockchain for EIP-8004 identity and reputation events.

---

## Tech Stack

| Technology          | Version | Purpose                                         |
|---------------------|---------|--------------------------------------------------|
| Rust                | 1.75+   | Systems programming language                     |
| Axum                | 0.8     | Async web framework for the REST API             |
| tokio               | 1.x     | Async runtime                                    |
| sqlx                | 0.8     | Type-safe async PostgreSQL driver                |
| alloy               | 0.12    | Ethereum/EVM library for RPC and ABI decoding    |
| reqwest             | 0.12    | HTTP client for fetching agent metadata          |
| tower-http          | 0.6     | Middleware (CORS, tracing)                       |
| tracing             | 0.1     | Structured logging                               |
| dotenvy             | 0.15    | Environment variable loading from `.env` files   |
| serde / serde_json  | 1.x     | Serialization and deserialization                |
| chrono              | 0.4     | Date and time handling                           |
| bigdecimal          | 0.4     | Arbitrary-precision decimal for feedback values  |
| thiserror           | 2.x     | Ergonomic error types                            |

---

## Setup

### Prerequisites

- Rust 1.75+ (`rustup` recommended)
- PostgreSQL 15+
- Access to Monad RPC endpoints

### Database Setup

```bash
# Create the database
createdb nad8004

# Run migrations
for f in migrations/*.sql; do psql -d nad8004 -f "$f"; done
```

### Environment Variables

Create a `.env` file in the `backend/` directory:

```env
DATABASE_URL=postgres://your_user:your_password@localhost:5432/nad8004
MONAD_MAINNET_RPC=https://rpc.monad.xyz
MONAD_TESTNET_RPC=https://testnet-rpc.monad.xyz
CORS_ORIGINS=http://localhost:3000
RUST_LOG=info,nad8004=debug
PORT=8080
```

| Variable             | Required | Default                 | Description                                   |
|----------------------|----------|-------------------------|-----------------------------------------------|
| `DATABASE_URL`       | Yes      | --                      | PostgreSQL connection string                  |
| `MONAD_MAINNET_RPC`  | Yes      | --                      | Monad Mainnet RPC URL (chain 143)             |
| `MONAD_TESTNET_RPC`  | Yes      | --                      | Monad Testnet RPC URL (chain 10143)           |
| `CORS_ORIGINS`       | No       | `http://localhost:3000` | Comma-separated CORS allowed origins          |
| `RUST_LOG`           | No       | `info`                  | Log level filter                              |
| `PORT`               | No       | `8080`                  | API server port                               |

### Build and Run

```bash
cd backend

# Build
cargo build

# Run (starts API server + event indexer)
cargo run
```

The API server listens on `http://localhost:8080`.

---

## API Endpoints

For full details with curl examples and response shapes, see the [API Reference](../docs/api-reference.md).

| Method | Endpoint                        | Description                          |
|--------|---------------------------------|--------------------------------------|
| GET    | `/api/agents`                   | List agents with search, filter, sort, pagination |
| GET    | `/api/agents/:id`               | Get agent detail (id = `{chain_id}-{agent_id}`)   |
| GET    | `/api/agents/:id/reputation`    | Get reputation history and feedback list           |
| GET    | `/api/agents/:id/activity`      | Get activity log (identity + reputation events)    |
| GET    | `/api/leaderboard`              | Get agents ranked by reputation score              |
| GET    | `/api/stats`                    | Get global marketplace statistics                    |

---

## Source Structure

```
backend/src/
├── main.rs                       # Entry point: starts Axum server + indexer
├── api/                          # HTTP route handlers
│   ├── mod.rs                    # Router setup
│   ├── agents.rs                 # /api/agents, /api/agents/:id, reputation, activity
│   ├── leaderboard.rs            # /api/leaderboard
│   └── stats.rs                  # /api/stats
├── db/                           # Database query modules
│   ├── mod.rs                    # DB pool setup
│   ├── agents.rs                 # Agent CRUD and search queries
│   ├── feedbacks.rs              # Feedback queries and aggregation
│   ├── activity.rs               # Activity log queries
│   └── indexer_state.rs          # Indexer cursor tracking
├── indexer/                      # Blockchain event indexer
│   ├── mod.rs                    # Indexer orchestration
│   ├── provider.rs               # Monad RPC provider setup (alloy)
│   ├── identity.rs               # IdentityRegistry event parsing
│   ├── reputation.rs             # ReputationRegistry event parsing
│   └── metadata.rs               # Agent metadata URI fetching
├── types/
│   └── mod.rs                    # Shared types (Agent, Feedback, Activity, etc.)
```

---

## Database Schema

The database consists of five tables. Migrations are in `backend/migrations/`.

### agents

Stores registered agents with resolved metadata from their EIP-8004 URI.

| Column      | Type         | Description                           |
|-------------|--------------|---------------------------------------|
| id          | SERIAL PK    | Auto-incrementing primary key         |
| agent_id    | BIGINT       | On-chain agent identifier             |
| chain_id    | INT          | Chain ID (143 or 10143)               |
| owner       | TEXT         | Owner wallet address                  |
| uri         | TEXT         | Metadata URI                          |
| metadata    | JSONB        | Full EIP-8004 metadata                |
| name        | TEXT         | Agent display name                    |
| description | TEXT         | Agent description                     |
| image       | TEXT         | Agent image URL                       |
| x402_support| BOOLEAN      | Whether agent supports x402 payments  |
| active      | BOOLEAN      | Whether agent is currently active     |
| created_at  | TIMESTAMPTZ  | Registration timestamp                |

Unique constraint: `(agent_id, chain_id)`

### feedbacks

Stores individual reputation feedback records.

| Column          | Type         | Description                         |
|-----------------|--------------|-------------------------------------|
| id              | SERIAL PK    | Auto-incrementing primary key       |
| agent_id        | BIGINT       | Target agent                        |
| chain_id        | INT          | Chain ID                            |
| client_address  | TEXT         | Feedback author address             |
| feedback_index  | BIGINT       | Sequential feedback index           |
| value           | NUMERIC      | Feedback score value                |
| value_decimals  | INT          | Decimal precision for value         |
| tag1            | TEXT         | Primary descriptive tag             |
| tag2            | TEXT         | Secondary descriptive tag           |
| endpoint        | TEXT         | Evaluated agent endpoint            |
| feedback_uri    | TEXT         | URI to extended feedback data       |
| feedback_hash   | TEXT         | Hash of feedback content            |
| revoked         | BOOLEAN      | Whether feedback has been revoked   |
| block_number    | BIGINT       | Block number of the event           |
| tx_hash         | TEXT         | Transaction hash                    |
| created_at      | TIMESTAMPTZ  | Event timestamp                     |

### activity_log

Unified timeline of all on-chain events per agent.

| Column      | Type         | Description                           |
|-------------|--------------|---------------------------------------|
| id          | SERIAL PK    | Auto-incrementing primary key         |
| agent_id    | BIGINT       | Associated agent                      |
| chain_id    | INT          | Chain ID                              |
| event_type  | TEXT         | Event name (e.g., Registered)         |
| event_data  | JSONB        | Event-specific data                   |
| block_number| BIGINT       | Block number                          |
| tx_hash     | TEXT         | Transaction hash                      |
| log_index   | INT          | Log index within the block            |
| created_at  | TIMESTAMPTZ  | Event timestamp                       |

### indexer_state

Tracks the indexer cursor for resumable polling.

| Column           | Type         | Description                        |
|------------------|--------------|------------------------------------|
| chain_id         | INT          | Chain ID                           |
| contract_address | TEXT         | Contract being indexed             |
| last_block       | BIGINT       | Last successfully indexed block    |
| updated_at       | TIMESTAMPTZ  | Last update timestamp              |

Primary key: `(chain_id, contract_address)`

---

## Event Indexer

The indexer runs alongside the API server and continuously polls the Monad blockchain for events emitted by the IdentityRegistry and ReputationRegistry contracts.

**Indexed Events:**

| Contract            | Events                                                |
|---------------------|-------------------------------------------------------|
| IdentityRegistry    | `Registered`, `URIUpdated`, `MetadataSet`             |
| ReputationRegistry  | `NewFeedback`, `FeedbackRevoked`, `ResponseAppended`  |

**Behavior:**

- Polls every 2 seconds with a batch size of up to 1000 blocks per `eth_getLogs` call
- Resumes from the last indexed block on restart (tracked in `indexer_state`)
- On `Registered` or `URIUpdated` events, fetches the agent's metadata URI and parses the EIP-8004 schema
- Indexes both Monad Mainnet (chain 143) and Testnet (chain 10143)

**Contract Addresses:**

| Network         | IdentityRegistry                               | ReputationRegistry                              |
|-----------------|------------------------------------------------|-------------------------------------------------|
| Mainnet (143)   | `0x8004A169FB4a3325136EB29fA0ceB6D2e539a432`   | `0x8004BAa17C55a88189AE136b182e5fdA19dE9b63`   |
| Testnet (10143) | `0x8004A818BFB912233c491871b3d84c89A494BD9e`   | `0x8004B663056A597Dffe9eCcC1965A193B7388713`   |
