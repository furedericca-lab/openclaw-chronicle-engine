# Chronicle Engine Backend Deployment

This folder contains the Docker deployment scaffold for the current Rust backend service used by Chronicle Engine.

## Intended runtime shape

- One backend container per deployment unit.
- LanceDB and SQLite job state mounted on persistent host volumes.
- Static TOML config baked into the image, with optional `CHRONICLE_*` environment overrides.
- OpenClaw connects to the backend over HTTP using a runtime bearer token.
- Admin endpoints stay off the ordinary OpenClaw runtime path.

## Files

- `Dockerfile`: multi-stage Rust build for the backend binary.
- `docker-compose.yml`: single-instance deployment using the published `chronicle-engine-backend` GHCR image.
- `backend.toml.example`: baked default config and schema reference for container deployments.

## Rust source layout

The Docker build and CI workflow assume the Rust backend source will live at:

```text
backend/
  Cargo.toml
  Cargo.lock
  src/main.rs
```

The crate currently builds the binary:

```text
chronicle-engine-rs
```

## Local image build

From the repository root:

```bash
  docker build \
  -f deploy/Dockerfile \
  -t chronicle-engine-backend:local \
  .
```

## Local compose deployment

Prepare persistent data directories:

```bash
mkdir -p data/chronicle-engine-backend/lancedb
mkdir -p data/chronicle-engine-backend/sqlite
```

Set the required runtime secrets in your shell or `.env` file:

```bash
export CHRONICLE_AUTH_RUNTIME_TOKEN='replace-me-runtime'
export CHRONICLE_AUTH_ADMIN_TOKEN='replace-me-admin'
export CHRONICLE_EMBEDDING_API_KEY='replace-me-embedding-key'
```

Run:

```bash
docker compose -f deploy/docker-compose.yml up -d
```

Health check:

```bash
curl -fsS http://127.0.0.1:8080/v1/health
```

## OpenClaw adapter wiring

Point the local OpenClaw adapter at:

```text
http://127.0.0.1:8080
```

Use the runtime token from `CHRONICLE_AUTH_RUNTIME_TOKEN` for data-plane requests only.

## Admin UI Access

The backend provides a bundled Admin UI accessible at:

```text
http://127.0.0.1:8080/admin
```

To login:
- Use the admin token from `CHRONICLE_AUTH_ADMIN_TOKEN`.
- The token is sent in the `Authorization: Bearer <token>` header by the SPA.
- Access is subject to rate limiting and audit logging.

## Environment override naming

The backend loads `/etc/chronicle-engine-backend/backend.toml` first, then applies any environment overrides whose names start with `CHRONICLE_`.

Use double underscores to represent nested TOML tables:

```text
CHRONICLE_AUTH__RUNTIME__TOKEN
CHRONICLE_AUTH__ADMIN__TOKEN
CHRONICLE_SERVER__BIND
CHRONICLE_STORAGE__SQLITE_PATH
CHRONICLE_PROVIDERS__EMBEDDING__API_KEY
```

Override values must match the type of the target TOML key:

- strings stay strings
- booleans use `true` / `false`
- numbers use normal integer / float literals
- arrays use TOML array literals such as `["a","b"]`

## GitHub Actions image build requirements

The workflow at `.github/workflows/docker-backend.yml` assumes:

- the backend crate exists at `backend/Cargo.toml`;
- the crate builds a release binary named `chronicle-engine-rs`;
- GitHub Container Registry is available for the repository;
- the repository `GITHUB_TOKEN` has `packages: write` permission;
- the image name is `ghcr.io/<owner>/chronicle-engine-backend`.

The workflow assumes the checked-in backend crate remains present and buildable.

## Branch and release behavior

- Pull requests build the Docker image for validation, but do not push.
- Pushes to `main` build and push branch/sha tags.
- Git tags matching `v*` also publish semver-style tags.
- After a successful non-PR publish, the workflow keeps only the newest `10` GHCR package versions for `chronicle-engine-backend` and deletes older versions.

If the repository uses a branch name other than `main`, adjust `.github/workflows/docker-backend.yml`.
