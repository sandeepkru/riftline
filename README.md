# Riftline

Riftline is a distributed messaging system inspired by Kafka. This repository is organized as a Cargo
workspace containing two crates:

- `server`: executable crate hosting the gRPC services
- `common`: shared library for common types and utilities

Common development tasks are managed via [`just`](https://github.com/casey/just). To build
the workspace run:

```bash
just build
```

Run the full CI task locally with:

```bash
just ci
```

Generate a coverage report with:

```bash
just coverage
```

Workspace Layout:
- server/: gRPC server
- common/: shared code

## Docker

A multi-stage `Dockerfile` is provided for building a minimal runtime image containing the server.
Build the container with:

```bash
docker build -t riftline-server .
```

Run the server:

```bash
docker run --rm -p 50051:50051 riftline-server
```

The server will start and listen on port `50051` by default.
