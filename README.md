# BlackFiles

BlackFiles is a self-hosted file manager for a single storage root. It combines a Rust/Rocket API, PostgreSQL-backed accounts and roles, and a React web client for browsing, downloading, uploading, deleting, and sharing files through one-time upload links.

The production image builds the Vite client and serves it from the same Rocket process. Persistent data lives outside the image: files in `storage/` and PostgreSQL data in its Docker volume.

## Features

- Browse, search, sort, paginate, preview, and download files from the configured storage root
- Resumable authenticated and public-link uploads using the [tus](https://tus.io/) protocol
- File and directory deletion, controlled by role permissions
- One-time public upload links scoped to a destination directory; interrupted transfers can resume for 24 hours
- Cookie-based JWT authentication with refresh sessions
- Role-based access control for file, user, role, and upload-link operations
- User and role administration from the web UI
- Path-component validation to prevent traversal outside `storage/`

## Stack

- **Server:** Rust, Rocket, Tokio
- **Database:** PostgreSQL with `deadpool-postgres`
- **Client:** React, Vite, TanStack Router/Query, Mantine
- **Deployment:** Docker Compose

## Quick start with Docker Compose

### 1. Configure environment variables

```sh
cp template.env .env
```

Set strong values before exposing the service:

- `POSTGRES_PASSWORD`
- `JWT_SECRET` — a random value of at least 32 characters
- `DEFAULT_ADMIN_PASSWORD`

`template.env` is configured for the Compose service names and exposes the application on port `4000`.

### 2. Start the service

```sh
docker compose up --build -d
```

Open [http://localhost:4000](http://localhost:4000) and sign in as `admin` with the value of `DEFAULT_ADMIN_PASSWORD`. The application creates this account only when no users exist in the database.

Useful commands:

```sh
# Follow application and database logs
docker compose logs -f

# Stop containers while keeping PostgreSQL and storage data
docker compose down
```

Uploaded files are bind-mounted at `./storage`. PostgreSQL data is stored in the named `blackfiles-pgdata` volume. Removing either is destructive.

## Production deployment

Use `template.docker-compose.yml` as a starting point. It keeps the application and PostgreSQL private to Docker networks and includes a database health check. Attach a reverse proxy to the external `caddy_net` network, or change the network and port configuration for your environment.

Before deployment:

1. Copy `template.docker-compose.yml` to `docker-compose.yml` and `template.env` to `.env`.
2. Replace the example database password, JWT secret, and bootstrap-admin password.
3. Ensure the host directory mounted at `./storage` is writable by the container.
4. Terminate TLS at a reverse proxy; the application cookies are `HttpOnly` and `SameSite=Lax`, but HTTPS is still required for an Internet-facing deployment.

Every idempotent feature script in `dbinit/` is embedded in the server and applied at startup. The same directory is mounted into PostgreSQL for fresh-volume initialization, so new and existing installations follow the identical schema path; do not apply scripts manually.

## Local development

Requirements: Rust, Bun, and PostgreSQL 18 (or the PostgreSQL Compose service).

Start only PostgreSQL with Compose:

```sh
docker compose up postgres -d
```

For a locally run server, set `POSTGRES_HOST=localhost` in `.env`; the Compose default (`blackfiles-db`) resolves only inside the Docker network. Then run the server and client in separate terminals:

```sh
cargo run
```

```sh
bun install
bun run dev
```

The Vite development server runs on [http://localhost:3000](http://localhost:3000) and proxies `/api` requests to the Rocket server on port `4000`.

Useful frontend commands:

```sh
bun run build
bun run typecheck
bun run lint
```

## API overview

All API routes are prefixed with `/api`.

| Area | Routes |
| --- | --- |
| Authentication | `POST /auth/login`, `POST /auth/logout`, `POST /auth/refresh`, `GET /auth/me` |
| File browsing | `GET /list`, `GET /list/<path..>`, `GET /files/<path..>` |
| File management | `DELETE /files/<path..>`, authenticated tus uploads at `/uploads` |
| Upload links | `POST /upload-links`, `GET /upload-links`, `DELETE /upload-links/<id>`, and public tus uploads under `/public/upload-links/<token>/uploads` |
| Administration | User, role, and permission endpoints under `/users`, `/roles`, and `/permissions` |

Authenticated operations require the relevant role permission. Public upload-link endpoints are the exception: a valid token authorizes one resumable file transfer to its preconfigured destination. The link is consumed only after that transfer completes successfully.

## Security model

BlackFiles restricts filesystem operations to the `storage/` root. Request paths are normalized as relative path components; absolute paths, parent traversal, and invalid components are rejected. Authorization is enforced separately for listing, downloads, uploads, deletion, user administration, role management, and upload-link management.

This is not a substitute for operational controls. Keep the storage mount and database private, use strong secrets, put the application behind HTTPS, and back up both the storage directory and PostgreSQL volume.
