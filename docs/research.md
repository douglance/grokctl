# Grok Bot gateway research

This document records the source evidence used to design `grokctl`. The reconstruction is
unofficial, targets 0.18.0, and says its boundaries may differ from Anysphere's original source.
It is behavioral evidence, not a supported API or a license grant.

## Reconstructed host behavior

The reconstructed host imports fixed wire paths and headers, including `/api`, `/health`,
`/events`, `/avatars`, request IDs, and slim-avatar negotiation
([gateway-server.ts lines 1-14](https://github.com/b-nnett/grok-bot-0.18-reconstructed/blob/main/source/host/gateway-server.ts#L1-L14)).

The host performs constant-time Bearer token comparison and rejects browser-origin requests
([gateway-server.ts lines 21-25](https://github.com/b-nnett/grok-bot-0.18-reconstructed/blob/main/source/host/gateway-server.ts#L21-L25)).
It only dispatches names present in `SAND_GATEWAY_COMMANDS`, attaches a request ID to completion
telemetry, and returns JSON
([gateway-server.ts lines 27-36](https://github.com/b-nnett/grok-bot-0.18-reconstructed/blob/main/source/host/gateway-server.ts#L27-L36)).

`GET /health` is handled before the protected route check. Events, avatars, bridges, and
`POST /api/<command>` are then identified, authenticated when a token exists, and dispatched
([gateway-server.ts lines 46-54](https://github.com/b-nnett/grok-bot-0.18-reconstructed/blob/main/source/host/gateway-server.ts#L46-L54)).
That ordering is why `grokctl gateway health` omits authorization while every other live surface
requires the runtime token.

The event stream sends `retry: 1000`, `data:` frames, and `:ping` heartbeats. The avatar route
decodes a data URL, supports an ETag/version, and forces non-executable response headers
([gateway-server.ts lines 38-43](https://github.com/b-nnett/grok-bot-0.18-reconstructed/blob/main/source/host/gateway-server.ts#L38-L43)).
The gateway command table parses absent bodies as `{}` and maps `sendPrompt`, prompt acceptance,
widget responses, both approval resolutions, and roster reads directly to the host API
([gateway-protocol.ts lines 1-24](https://github.com/b-nnett/grok-bot-0.18-reconstructed/blob/main/source/host/gateway-protocol.ts#L1-L24)).

Slim-avatar behavior is a server-side projection: selected results and agent events have inline
avatar data removed without changing the underlying records
([gateway-protocol.ts lines 136-149](https://github.com/b-nnett/grok-bot-0.18-reconstructed/blob/main/source/host/gateway-protocol.ts#L136-L149)).
The reconstructed configuration binds to loopback by default and requires or generates a token
for non-loopback binds
([gateway-config.ts lines 41-54](https://github.com/b-nnett/grok-bot-0.18-reconstructed/blob/main/source/host/gateway-config.ts#L41-L54)).

The reconstructed coordinator confirms the client contract: it adds Bearer auth
([gateway-client.ts lines 88-98](https://github.com/b-nnett/grok-bot-0.18-reconstructed/blob/main/source/node-agent-coordinator/gateway/gateway-client.ts#L88-L98)),
asks for slim avatars
([gateway-client.ts lines 135-146](https://github.com/b-nnett/grok-bot-0.18-reconstructed/blob/main/source/node-agent-coordinator/gateway/gateway-client.ts#L135-L146)),
and posts JSON to `/api/<method>`, treating 5xx responses as reachability failures
([gateway-client.ts lines 246-272](https://github.com/b-nnett/grok-bot-0.18-reconstructed/blob/main/source/node-agent-coordinator/gateway/gateway-client.ts#L246-L272)).

## Third-party SDK ideas ported to Rust

The SDK makes the four paths, Bearer scheme, request-ID header, and slim-avatar header explicit
([commands.ts lines 50-56](https://github.com/Adam91holt/grokbot-sdk/blob/main/sdk/src/gateway/commands.ts#L50-L56)).
It keeps destructive operations unsugared and requires an explicit unsafe path
([commands.ts lines 58-103](https://github.com/Adam91holt/grokbot-sdk/blob/main/sdk/src/gateway/commands.ts#L58-L103)).
`grokctl` strengthens that rule: unknown commands are also destructive/open-world, and all
mutations require an idempotency key.

The SDK's useful discovery ideas are preserved: explicit URL wins, wildcard binds become
loopback, the token stays in memory, and public output exposes only `hasToken`
([discovery.ts lines 125-186](https://github.com/Adam91holt/grokbot-sdk/blob/main/sdk/src/gateway/discovery.ts#L125-L186),
[discovery.ts lines 190-224](https://github.com/Adam91holt/grokbot-sdk/blob/main/sdk/src/gateway/discovery.ts#L190-L224)).

The SDK treats health as unauthenticated, avatars and events as authenticated, and events as an
SSE iterator without the unary timeout
([client.ts lines 210-257](https://github.com/Adam91holt/grokbot-sdk/blob/main/sdk/src/gateway/client.ts#L210-L257)).
Its generated manifest distinguishes host command names from locally cited wrapper input keys
because the host exposes neither `listCommands` nor per-command JSON schemas
([host-manifest.generated.ts lines 521-526](https://github.com/Adam91holt/grokbot-sdk/blob/main/sdk/src/gateway/host-manifest.generated.ts#L521-L526)).

`grokctl` ports these ideas without TypeScript or direct database readers:

| SDK idea | Rust implementation |
| --- | --- |
| Gateway-first live state | `grokctl-core::gateway` |
| URL/file/env discovery | `grokctl-core::config` |
| Token-free diagnostics | `GatewaySecret` plus redacted errors |
| Typed common workflows | Bot list and prompt/wait services |
| Raw forward-compatibility | `gateway call` with policy guards |
| Host manifest drift | `grokctl-manifest` plus warn-and-continue verdicts |
| Client-side prompt wait | roster, task, subagent, acceptance, and transcript polling |
| Mutation witnesses | local `SQLite` receipt journal |

## Deliberate boundaries

- No direct `sand-data` database readers. Live host calls remain authoritative.
- No tunnel, VPN, SSH, Docker, or host-process management.
- No approval bypass. `resolveAutoReviewApproval` and `resolveLocalToolPermission` are blocked.
- No claim that the 0.18 command set matches the currently installed desktop app.
- No automatic call to an unknown host command without explicit unsafe and idempotency inputs.

## Grok Bot 0.29 template sharing

The August 28, 2026 announcement introduced public Bot templates. The official management guide
states that a recipient can preview a public link and add an account-local copy; the copy excludes
the owner's computer, logins, and conversation history. It also warns that the shared identity,
description, skills, and routines are public configuration
([Create and manage Bots](https://docs.x.ai/grok-bot/bots#share-a-bot)).

The authorized local Grok Bot 0.29.0 bundle, with `app.asar` integrity hash
`c98ed927a71a5c547617bca79e6b3f94aa6bcfbd3d646763e62ffa63a2ace83e`, exposes these coordinator
methods:

- `createAgentFromTemplate`
- `publishBotTemplate`
- `listBotTemplates`
- `getBotTemplateVersion`
- `getBotTemplateForSourceAgent`
- `deleteBotTemplate`
- `setBotTemplateVisibility`

The same bundle constructs public URLs as `https://x.ai/bot/<shareId>` and validates 21-character
URL-safe share IDs. Its add flow sends the share ID, a stable new Bot ID, name, avatar shape,
avatar color, and expected active version. Publishing sends the share ID and version. Visibility
uses `public` or `team`.

`grokctl` treats list and get operations as reads, adding a copy as an open-world mutation, and
publish, visibility, and delete as unsafe open-world operations. Draft creation is deliberately
absent because Grok Bot mediates it through the approval layer.
