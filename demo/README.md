# Docker demo

This demo runs the published `0.1.0-rc.1` resolver and manager images with a
local Midnight node, indexer, and proof server. It is intended for evaluation
and API/UI exploration, not production deployment.

## Prerequisites

- Docker Engine with the Docker Compose v2 plugin
- `curl` for the health command
- Network access to pull the GHCR and Midnight infrastructure images

The application images are public release-candidate images. If GHCR access is
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

## Release candidate images

The defaults are:

```text
ghcr.io/midnightntwrk/midnight-did-resolver-service:0.1.0-rc.1
ghcr.io/midnightntwrk/midnight-did-manager-service:0.1.0-rc.1
```

The release workflow publishes exact RC tags and does not move `latest` for a
pre-release. A release promotion to `main` followed by `v0.1.0-rc.1`, or the
manual workflow dispatch documented in the repository README, publishes both
images.
