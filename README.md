# BlackFiles
A modern file server built with Rust and SvelteKit, providing a fast and intuitive
web interface for browsing and downloading files. \
Built on the [Rocket](https://rocket.rs/) web framework with [Tokio](https://tokio.rs/) async runtime for excellent performance.

## Styling
- Now using [Catppuccin](https://catppuccin.com/palette/) themes for better aesthetics!
- 99% of the icons are from Catppuccin's icon set. For any issue with the copyright of the icons, please don't hesitate to contact me.

## Features
- **Modern Stack**: Rust backend with SvelteKit frontend for optimal performance and developer experience
- **Unified Architecture**: Monorepo structure with API and web UI served from a single Rocket server
- **REST API**: Full-featured API for listing directories and file metadata
- **Secure Downloads**: Path validation and sanitization to prevent unauthorized access
- **Responsive UI**: Clean, intuitive interface with Catppuccin theming and file-type icons
- **Real-time Navigation**: Client-side routing for instant directory browsing

## Endpoints
### API Routes
- `GET /api/list` - List root directory contents with file metadata
- `GET /api/list/<path..>` - List contents of any directory path

### File Operations
- `GET /files/<path..>` - Stream file downloads with proper MIME types

### Frontend
- `GET /*` - Serves the SvelteKit application (handles client-side routing)

## Security
The application implements multiple layers of security to protect against common vulnerabilities:
- **Path Traversal Prevention**: All paths are sanitized and validated before file system access
- **Canonical Path Resolution**: Paths are resolved to their absolute form and verified against the storage root
- **Access Control**: Hidden files (starting with `.`) and system files are blocked from access
- **Content Type Validation**: Files are served with appropriate MIME types to prevent content-type attacks
- **Boundary Checking**: All file operations stay within the designated storage directory

## Dependencies
Very light on the backend! Just [Rocket](https://rocket.rs/) and [Tokio](https://tokio.rs/)! \
And pretty light on the frontend too, using [SvelteKit 5](https://svelte.dev/) with
[TailwindCSS](https://tailwindcss.com/) and [DaisyUI](https://daisyui.com/).

## Running in Production
- Ensure the `storage` directory exists and has appropriate permissions
  - The `docker-compose.yml` uses a bind mount by default for ease of access,
  but you can also use a volume
- Setting up a reverse proxy is _recommended_ but not needed. The API's default port is `8000`. 
- No matter what reverse proxy, it will require creating your own bridge network (name it accordingly, default is `caddy_net`) network and adding a new service to the `docker-compose.yml` file. (if you don't have a global or system proxy)

## License
[MIT](https://opensource.org/license/mit)