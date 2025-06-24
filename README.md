# Riftline

Riftline is a distributed messaging system inspired by Kafka. This repository is organized as a Cargo
workspace containing two crates:

- `server`: executable crate hosting the gRPC services
- `common`: shared library for common types and utilities

To build the workspace run:

```bash
cargo build
```


Workspace Layout:
- server/: gRPC server
- common/: shared code
