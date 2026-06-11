# External Browser Test Docker Image

Self-contained Docker image for authentication E2E tests that require a headless
browser (external browser auth, OAuth flows, etc.).

## What's in the image

- **Node.js 20** (Debian Bookworm base)
- **Headless Chromium** on port 9222 via `/chromium-headless.sh`
- **Playwright** + browser credential helper scripts (TOTP, OAuth token, PAT)
- **Python 3** + pip + venv (for building/running the Python connector tests)
- **Rust 1.88.0** toolchain (for compiling `sf_core` from source)
- **C build essentials**, pkg-config, libssl-dev, protobuf-compiler, CMake 4.0.3
- **unixODBC** + dev headers (for building/running ODBC tests)

## Directory layout

```
external-browser/
├── Dockerfile              # Single Dockerfile, platform-neutral
├── build.sh                # Build for CI (linux/amd64)
├── README.md
└── browser-helpers/        # Node.js scripts baked into the image
    ├── package.json
    ├── provideBrowserCredentials.js
    ├── cleanBrowserProcesses.js
    ├── getOauthToken.js
    ├── patHelper.js
    ├── totpGenerator.js
    └── getTOTP.js
```

## Building

### CI (linux/amd64)

```bash
./tests/docker/external-browser/build.sh
docker tag <local-tag> <registry>/<local-tag>
docker push <registry>/<local-tag>
```

### Local development

The local run script builds the image automatically for the host architecture:

```bash
./tests/auth/run_auth_browser_local.sh python
./tests/auth/run_auth_browser_local.sh odbc
```

The platform is passed via `docker build --platform=...`; a single Dockerfile serves both architectures.

## Usage

CI / pre-built image (`DOCKER_IMAGE` must be set):

```bash
DOCKER_IMAGE=<image> ./tests/auth/run_auth_browser.sh python
DOCKER_IMAGE=<image> ./tests/auth/run_auth_browser.sh odbc
```

Local development (builds the image, then runs tests):

```bash
./tests/auth/run_auth_browser_local.sh python
./tests/auth/run_auth_browser_local.sh odbc
```
