# BlackFiles
A Rust-based proof-of-concept web server providing a RESTful API and web interface for browsing and downloading files.
Build with the [Rocket](https://rocket.rs/) framework and [Tokio](https://tokio.rs/)

## Features
- **REST API**: List directories and files with metadata
- **File Download**: Secure file serving with path sanitization and validation
- **Web interface**: Simple HTML frontend for browsing and downloading files
- **Static Assets**: Serves CSS/JS files for the web interface

## Endpoints
### API Routes
- `GET /api/list` - List root directory contents
- `GET /api/list/<path..>` - List contents of a specific directory

### File Download
- `GET /files/<path..>` - Download a file (returns a sized file stream)

### Web Interface
- `GET /css/<file..>` - Serve static CSS files
- `GET /js/<file..>` - Serve static JS files
- `GET /*` - Serve the frontend index.html for client-side routing

## Security
Although just a proof-of-concept, the application still implements several security measures
- **Path Sanitization**: All user-provided paths are sanitized to prevent directory traversal attacks
- **Canonicalization**: Paths are resolved to their canonical form and checked against the storage root
- **File Access Control**: Hidden files (starting with .) are blocked for download

## Dependencies
Very light! Just [Rocket](https://rocket.rs/) and [Tokio](https://tokio.rs/)!

## Running in Production
- Ensure the `storage` directory exists and has appropriate permissions
  - The `docker-compose.yml` uses a bind mount by default, but you can also use a volume
- Setting up a reverse proxy is _recommended_ but not needed. The default port is `8000`.

## License
[MIT](https://opensource.org/license/mit)