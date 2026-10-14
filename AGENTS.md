# Repository guidance

## Crate structure

- `crates/mforward-core`: shared `Forwarder` trait and error type.
- `crates/mforward-kubectl`: kubectl value objects in `src/types.rs` and port-forward process in `src/lib.rs`.
- `crates/mforward-gcp-iap`: GCP IAP value objects in `src/types.rs` and gcloud tunnel process in `src/lib.rs`.
- `crates/mforward-config`: TOML parsing, provider defaults, and configured-forwarder factory.
- `crates/mforward-cli`: CLI entry point and concurrent process orchestration.

Keep provider implementations in their own crates. Keep the core forwarding trait provider agnostic.
- Add a dependency to the workspace dependency table when multiple crates use it.

## External crates

- `anyhow`: add context to errors returned by the CLI entry point.
- `clap`: parse the command-line config path.
- `futures-util`: store and join concurrent forwarder tasks.
- `serde`: derive typed TOML deserialization; `toml`: parse config and merge provider defaults.
- `thiserror`: define crate-level error types.
- `tokio`: run async tasks, handle Ctrl-C, and spawn external processes.
- `tracing`: emit structured logs; `tracing-subscriber`: configure the CLI log output and filter.
- Direct `tracing-core` is declared in the workspace but has no current source usage; check before adding code that depends on it.

## Configuration

- Deserialize TOML into typed provider arguments. Keep field names compatible with `mforward.example.toml`; provider defaults apply only to that provider, and explicit values take precedence.
- Model constrained fields as value objects with private inner fields, explicit constructors, and `value()` getters. Put validation in each `new` constructor and route deserialization through the same validation.
- Preserve valid domain cases, including local port `0` where it requests automatic port selection.
- Use typed errors at crate boundaries. Provider errors implement `Error` and convert into `ForwarderError`.

## Style and changes

- Use direct implementations for value objects; do not generate them with macros.
- Prefer borrowing for reads; allocate when an owned value is needed, such as when building process arguments.
- Add unit tests beside the behavior they cover. Configuration changes should cover parsing, defaults, invalid values, and provider selection.
