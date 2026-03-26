# Chronicle Engine Backend Deployment

This folder contains the Docker deployment scaffold for the current Rust backend service used by Chronicle Engine.

## Intended runtime shape

- One backend container per deployment unit.
- LanceDB and SQLite job state mounted on persistent host volumes.
- Static TOML config mounted read-only.
- OpenClaw connects to the backend over HTTP using a runtime bearer token.
- Admin endpoints stay off the ordinary OpenClaw runtime path.

## Files

- `Dockerfile`: multi-stage Rust build for the backend binary.
- `docker-compose.yml`: single-instance deployment using the published `chronicle-engine-backend` GHCR image.
- `backend.toml.example`: example static config file for container deployments.

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

Prepare the runtime config:

```bash
cp deploy/backend.toml.example \
  deploy/backend.toml
mkdir -p data/chronicle-engine-backend/lancedb
mkdir -p data/chronicle-engine-backend/sqlite
chmod 600 deploy/backend.toml
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

Use the runtime token from `backend.toml` for data-plane requests only.

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

If the repository uses a branch name other than `main`, adjust `.github/workflows/docker-backend.yml`.
