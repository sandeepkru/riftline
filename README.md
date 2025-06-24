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
