# Docker demo

This demo runs the published `0.1.0` resolver and manager images with a local
Midnight node, indexer, and proof server. It is intended for evaluation and
API/UI exploration, not production deployment.

## Prerequisites

- Docker Engine with the Docker Compose v2 plugin
- `curl` for the health command
- Network access to pull the GHCR and Midnight infrastructure images

The application images are public stable release images. If GHCR access is
restricted in your environment, authenticate Docker before starting the demo.
On its first launch, the proof server may download proving parameters; the
manager UI can start before that warm-up is complete, but proof-backed
operations should wait until the proof-server logs show that it is listening.

## Start

From the repository root, run the complete demo with one command:

```bash
just demo
```

The equivalent explicit commands are:

```bash
./demo/run.sh up
./demo/run.sh health
```

Open:

- Resolver API and Swagger UI: <http://127.0.0.1:3001>
- Manager wallet UI: <http://127.0.0.1:3010/wallet>

Stop and remove the demo containers and network:

```bash
./demo/run.sh down
```

Manager state is persisted in `~/.midnight-did/manager` on the host by default,
so wallet/session data survives demo restarts. Override `MANAGER_DATA_DIR` in
`demo/.env` to use another directory. The container runs with the invoking
user's UID/GID so the bind-mounted state remains writable.

Remove the demo containers, network, and any demo volumes:

```bash
just demo-clean
```

Follow logs or inspect the resolved configuration:

```bash
just demo-logs
just demo-config
```

## Configuration

Copy the example overrides before starting:

```bash
cp demo/.env.example demo/.env
```

The file controls the application image references and host ports. Set
`RESOLVER_IMAGE` or `MANAGER_IMAGE` to a locally built image to test changes
without publishing them. The internal application endpoints use Compose service
names and should not be changed to `127.0.0.1`.

## Stable images

The defaults are:

```text
ghcr.io/midnightntwrk/midnight-did-resolver-service:0.1.0
ghcr.io/midnightntwrk/midnight-did-manager-service:0.1.0
```

The stable release workflow publishes both exact-version tags and the
corresponding `latest` tags. Release candidates use exact version tags and do
not move `latest`; override `RESOLVER_IMAGE` and `MANAGER_IMAGE` in `demo/.env`
when testing an RC or a locally built image.
