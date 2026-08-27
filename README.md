# grokctl

`grokctl` is a Rust CLI and MCP server for an authorized Grok Bot Sand gateway. It uses one
Incurs command graph for humans and agents, keeps bearer tokens out of output, warns when the
live host drifts from the pinned compatibility seed, and records mutation witnesses in SQLite.

This project targets a private, undocumented interface. It is not affiliated with xAI,
Anysphere, or Cursor. The checked-in compatibility seed is source-oriented prior art from the
unofficial Grok Bot 0.18 reconstruction and is not an official API contract.

See [docs/research.md](docs/research.md) for the exact reconstructed and SDK source lines behind
the transport, safety, discovery, and prompt-workflow decisions.

## Install and inspect

Rust 1.88 is pinned by `rust-toolchain.toml`.

```console
cargo build --release
./target/release/grokctl --help
./target/release/grokctl --llms
```

Run the same graph as an MCP server:

```console
./target/release/grokctl --mcp
```

## Connect

For an existing route to a host gateway, set the origin and token at runtime:

```console
export GROKCTL_GATEWAY_URL=https://bot-host.example
export GROKCTL_GATEWAY_TOKEN=replace-me
grokctl profile show --json
grokctl gateway health --json
```

On the Bot host, `grokctl` also discovers `/home/box/sand-data/gateway.json` and its legacy
`agent-data` alias. A wildcard bind address is normalized to loopback. Plain HTTP to a remote
host is rejected unless `--allow-insecure-http` is explicit.

## Common operations

```console
grokctl bot list --json
grokctl bot prompt BOT_ID "Summarize current work" --idempotency-key prompt-2026-08-27 --wait
grokctl manifest check --json
grokctl gateway call getHostStatus --json
grokctl gateway events agents --limit 5 --timeout-seconds 10 --json
grokctl gateway avatar BOT_ID --json
```

Fixed mutation commands require `--idempotency-key`. Destructive and unknown raw commands also
require `--unsafe-mode`. `resolveAutoReviewApproval` and `resolveLocalToolPermission` are always
blocked because approvals remain owned by Grok Bot.

## Recovery

- `gateway URL was not found`: set `GROKCTL_GATEWAY_URL` or provide `--discovery-path`.
- `requires a bearer token`: set `GROKCTL_GATEWAY_TOKEN`; the health endpoint is the exception.
- Remote plaintext rejection: use HTTPS or an existing trusted tunnel. Use insecure HTTP only on
  a network you control.
- Compatibility mismatch: inspect `grokctl manifest check`. Calls continue by design; refresh the
  manifest from an authorized cloud-host source snapshot before adding typed assumptions.
- Ambiguous receipt: the request may have reached the host. Inspect the host before retrying with a
  new key.

Refresh the compatibility manifest only from an authorized host source snapshot:

```console
cargo xtask manifest /path/to/host-main.cjs HOST_VERSION ./host-manifest.json
```

## Quality gates

```console
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo doc --workspace --no-deps
cargo xtask quality
```
