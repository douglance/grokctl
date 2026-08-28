<p align="center">
  <img
    src="https://raw.githubusercontent.com/douglance/grokctl/main/assets/grokctl-icon.png"
    width="168"
    alt="grokctl icon: colorful Grok Bot shapes on a black background"
  />
</p>

<h1 align="center">grokctl</h1>

<p align="center">
  <strong>Rust-native control for authorized Grok Bot Sand gateways.</strong>
  <br />
  One typed command graph for terminals, scripts, and MCP clients.
</p>

<p align="center">
  <a href="https://crates.io/crates/grokctl"><img alt="Crates.io version" src="https://img.shields.io/crates/v/grokctl?style=flat-square"></a>
  <a href="https://crates.io/crates/grokctl"><img alt="Crates.io downloads" src="https://img.shields.io/crates/d/grokctl?style=flat-square"></a>
  <a href="https://docs.rs/grokctl"><img alt="docs.rs" src="https://img.shields.io/docsrs/grokctl?style=flat-square"></a>
  <a href="https://github.com/douglance/grokctl/blob/main/rust-toolchain.toml"><img alt="Rust 1.88" src="https://img.shields.io/badge/rust-1.88%2B-f46623?style=flat-square"></a>
  <a href="https://github.com/douglance/grokctl/blob/main/Cargo.toml"><img alt="License: MIT" src="https://img.shields.io/badge/license-MIT-111827?style=flat-square"></a>
</p>

`grokctl` controls an authorized Grok Bot Sand gateway from a Rust command-line interface (CLI)
or Model Context Protocol (MCP) stdio server. It keeps bearer tokens out of output, warns when the live host
drifts from the pinned compatibility seed, records mutation witnesses in
SQLite, and exposes the same command graph to humans and agents.

This project targets a private, undocumented interface. It is not affiliated
with xAI, Anysphere, or Cursor. The checked-in compatibility seed is
source-oriented prior art from the unofficial Grok Bot 0.18 reconstruction and
is not an official API contract.

## Install

Rust 1.88 is pinned by [`rust-toolchain.toml`](rust-toolchain.toml).

```console
cargo install --locked grokctl
grokctl --help
grokctl --llms
```

Build from this checkout when you want the exact repository state:

```console
cargo build --release
./target/release/grokctl --help
```

## Connect

Use a gateway URL and bearer token at runtime:

```console
export GROKCTL_GATEWAY_URL=https://bot-host.example
export GROKCTL_GATEWAY_TOKEN=replace-me
grokctl profile show --format json
grokctl gateway health --format json
grokctl bot list --format json
```

On the Bot host, `grokctl` also discovers
`/home/box/sand-data/gateway.json` and its legacy `agent-data` alias. A wildcard
bind address is normalized to loopback. Unencrypted HTTP to a remote host is rejected
unless `--allow-insecure-http` is explicit.

## Use it

```console
grokctl bot list --format json
grokctl bot create \
  --body '{"name":"Release Captain","description":"Owns release readiness and rollback."}' \
  --idempotency-key create-release-captain-v1 \
  --format json
grokctl bot prompt "Release Captain" "Summarize current work" \
  --idempotency-key prompt-release-captain-20260828 \
  --wait true \
  --format json
grokctl gateway call getHostStatus --format json
grokctl gateway events agents --limit 5 --timeout-seconds 10 --format jsonl
grokctl manifest check --format json
```

The CLI surface is intentionally broad but typed:

| Area | Commands |
| --- | --- |
| Bots | `bot count`, `bot create`, `bot delete`, `bot duplicate`, `bot kickstart`, `bot list`, `bot prompt`, `bot search`, `bot transcript-tail`, `bot update` |
| Gateway | `gateway avatar`, `gateway call`, `gateway events`, `gateway health` |
| Host | `computer`, `host`, `media`, `memory`, `routine`, `workflow` |
| Integrations | `mcp add`, `mcp doctor`, `skills add`, `skills list`, `completions` |
| Evidence | `manifest check`, `profile show`, `receipt get` |

## MCP

Run `grokctl` directly as an MCP stdio server:

```console
grokctl --mcp
```

Register and verify it from the CLI:

```console
grokctl mcp add --agent claude-code
grokctl mcp doctor
```

Agents can inspect the machine-readable large language model (LLM) command
manifest before deciding which operation to call:

```console
grokctl --llms-full
grokctl bot prompt --schema
```

## How it works

```text
 Human terminal       Automation         Agent runtime
       |                   |                   |
       +--------- CLI / structured output / MCP --------+
                               |
                     Incurs command graph
                               |
                      typed Rust services
                         /           \
                Sand gateway     SQLite receipts
                         \
                     compatibility manifest
```

The CLI, schemas, LLM manifest, and MCP tools are projections of the same Rust
definitions. There is no parallel wrapper to drift.

## Safety model

`grokctl` is built for explicit operator authority.

- Fixed mutation commands require `--idempotency-key`.
- Destructive and unknown raw gateway commands require `--unsafe-mode`.
- Bearer tokens are never printed by `profile show`.
- `resolveAutoReviewApproval` and `resolveLocalToolPermission` are blocked
  because approvals remain owned by Grok Bot.
- Remote unencrypted HTTP is rejected unless the caller opts in.

## Compatibility evidence

See [`docs/research.md`](docs/research.md) for the reconstructed and software development kit (SDK)
source lines behind the transport, safety, discovery, and prompt-workflow decisions.

Refresh the compatibility manifest only from an authorized host source snapshot:

```console
cargo xtask manifest /path/to/host-main.cjs HOST_VERSION ./host-manifest.json
```

## Recovery

| Symptom | Action |
| --- | --- |
| `gateway URL was not found` | Set `GROKCTL_GATEWAY_URL` or pass `--discovery-path`. |
| `requires a bearer token` | Set `GROKCTL_GATEWAY_TOKEN`. The health endpoint is the exception. |
| Remote unencrypted HTTP rejection | Use HTTPS or an existing trusted tunnel. Use insecure HTTP only on a network you control. |
| Compatibility mismatch | Run `grokctl manifest check`, then refresh the manifest from an authorized cloud-host source snapshot before adding typed assumptions. |
| Ambiguous receipt | Inspect the host before retrying with a new idempotency key. The request may have reached the host. |

## Quality gates

The workspace keeps linting strict early: `unsafe_code` is forbidden,
`missing_docs` is denied, and Clippy denies `all`, `cargo`, `nursery`,
`pedantic`, cognitive complexity, excessive nesting, panics, unwraps, todos,
wildcard imports, and oversized functions.

```console
cargo fmt --all -- --check
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo doc --workspace --no-deps
cargo xtask quality
```

## License

MIT. See the workspace metadata in [`Cargo.toml`](Cargo.toml).
