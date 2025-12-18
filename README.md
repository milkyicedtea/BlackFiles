# BlackFiles
A Rust-based proof-of-concept file server providing a RESTful API and web UI for browsing and downloading files.
Built with the [Rocket](https://rocket.rs/) web framework and [Tokio](https://tokio.rs/)

## Features
- **Monorepo-style**: Backend and frontend in a single repository for easy management, 
as well as API and web UI served from the same server (Rocket)
- **REST API**: List directories and files with metadata
- **File Download**: Secure file serving with path sanitization and validation
- **Web UI**: Very simple and intuitive frontend for browsing and downloading files

## Endpoints
### API Routes
- `GET /api/list` - List root directory contents
- `GET /api/list/<path..>` - List contents of a specific directory

### File Download
- `GET /files/<path..>` - Download a file (returns a sized file stream)

### Web UI
- `GET /*` - Serves the frontend index.html for client-side routing

## Security
Although just a proof-of-concept, the application still implements several security measures
- **Path Sanitization**: All user-provided paths are sanitized to prevent directory traversal attacks
- **Canonicalization**: Paths are resolved to their canonical form and checked against the storage root
- **File Access Control**: Hidden files (starting with .) are blocked for download

## Dependencies
Very light on the backend! Just [Rocket](https://rocket.rs/) and [Tokio](https://tokio.rs/)! \
And pretty light on the frontend too, using [Svelte](https://svelte.dev/) with
[TailwindCSS](https://tailwindcss.com/) and [DaisyUI](https://daisyui.com/).

## Running in Production
- Ensure the `storage` directory exists and has appropriate permissions
  - The `docker-compose.yml` uses a bind mount by default for ease of access,
  but you can also use a volume
- Setting up a reverse proxy is _recommended_ but not needed. The api's default port is `8000`.
  - It currently also binds on host port 4100. This is not neccessary when using a reverse proxy.

## License
[MIT](https://opensource.org/license/mit)