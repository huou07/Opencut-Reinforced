# Opencut Reinforced Architecture

## Status

Pre-implementation architecture direction.

This document describes the intended system boundaries.
It does not imply that the components already exist.

## Product model

Opencut Reinforced (OR) is intended to be:

- a cross-platform video editor
- human-editable
- CLI-controllable
- deeply agent-controllable
- local-first where practical
- capable of optional cloud AI through user-provided credentials
- open source under MIT

## Primary architecture

Long-term conceptual stack:

    Desktop / Mobile GUI
             │
             │
        Command API
       ┌─────┼─────┐
       │     │     │
      GUI   CLI   Agents
       │     │     │
       └─────┼─────┘
             │
         Rust Core
             │
    ┌────────┼─────────┐
    │        │         │
 Timeline  Media    Project State
    │        │
    │     FFmpeg
    │
 Render Graph
    │
   wgpu

Important:
This is conceptual architecture, not implemented code.

## Core

Preferred language:

Rust

Responsibilities should eventually include:

- project model
- timeline model
- command model
- undo/redo
- media metadata
- render graph
- deterministic editing operations
- structured errors
- stable object identifiers

Domain state must not live primarily inside UI widgets.

## Media

FFmpeg is the intended baseline for:

- probing
- demuxing
- decoding
- encoding
- muxing
- media conversion
- audio/video interoperability

Exact bindings and FFmpeg distribution configuration are not decided yet.

Any packaged FFmpeg configuration must later receive explicit license review.

## GPU rendering

wgpu is the preferred rendering/compositing abstraction.

Target direction includes native GPU backends on supported operating systems.

Do not implement backend-specific architecture prematurely.

## UI

Flutter is the current preferred UI layer for desktop/mobile.

UI responsibilities:

- render state
- user interaction
- accessibility
- navigation
- panels
- inspectors
- timeline presentation

UI must call domain commands rather than directly becoming the source of
project truth.

DESIGN.md remains the visual source of truth.

## Command architecture

Long-term invariant:

If an editing capability exists, human UI, CLI, and agents should be able to
reach the same underlying operation.

Do not implement three independent editing systems.

Conceptually:

    user interaction
    CLI command
    agent tool
         ↓
    validated command
         ↓
    domain operation
         ↓
    state/event/result

This model should support deterministic behavior and undo/redo.

## CLI

The future OR CLI is first-class.

Desired properties:

- machine-readable output
- JSON output where appropriate
- command discovery/introspection
- stable identifiers
- deterministic exit codes
- dry-run for suitable complex/destructive operations
- no secret leakage
- ability to attach to an active application/project when appropriate

The CLI must not depend on computer vision or GUI coordinate clicking for
normal editing operations.

## Agents

Agents are clients of structured OR capabilities.

Agent integration should prefer:

- CLI
- IPC
- structured command API
- project inspection
- schema discovery

over vision-based UI automation.

The application API-key boundary must prevent agents from reading plaintext
user secrets.

AGENTS.md defines repository agent behavior.
This document defines product architecture.

## AI

AI is a subsystem, not the owner of the editing architecture.

Potential categories:

- speech recognition
- subtitle translation
- image generation
- video generation
- voice/audio generation
- segmentation/background removal
- enhancement
- editing/planning agents

AI providers must be abstracted enough to support:

- local inference
- optional cloud APIs
- user-selected providers

Model/runtime/license decisions are made individually.

Do not assume a model is redistributable simply because its inference code
is open source.

## Model manager

Future model management should track:

- model ID
- version
- task
- source
- hash/checksum
- size
- runtime
- hardware requirements
- language coverage
- license
- install state

Large model weights should not be committed to Git.

## Project format

The project format should eventually be:

- deterministic
- versioned
- migration-capable
- inspectable by tools/agents
- robust against partial failure

Prefer explicit schemas and stable identifiers.

Exact format is not finalized.

Do not prematurely lock OR into a proprietary opaque binary project format.

## Security boundaries

Treat these as distinct trust boundaries:

- user project files
- imported media
- plugins
- AI models
- cloud providers
- agents
- CLI
- operating-system resources
- secrets

Imported files, subtitles, metadata, prompts, and model output must not be
treated as trusted instructions.

## Secrets

Future API keys must use platform-appropriate secure storage.

Agents/CLI may inspect whether a provider is configured, but must not receive
plaintext stored credentials.

## Plugins

Plugin architecture is planned but not designed yet.

Do not commit to native unrestricted plugins before the sandbox/security
model is decided.

## Platforms

Current intended targets:

Desktop:
- macOS
- Windows
- Linux

Mobile:
- Android

iOS is not currently a required release target.

Avoid platform assumptions in the domain architecture.

## Testing architecture

Future test layers should include, where applicable:

- Rust unit tests
- timeline/domain property tests
- serialization round-trip tests
- CLI contract tests
- deterministic render tests
- golden-frame tests
- audio/video synchronization tests
- Flutter widget tests
- platform integration tests
- agent command tests
- project migration tests
- crash/recovery tests

Test media should be tiny and legally safe/self-generated.

## Architecture decision process

Do not silently convert architectural ideas into permanent decisions.

For significant irreversible decisions:
- document the decision
- document alternatives
- document tradeoffs
- update this file or create an ADR when appropriate

Use docs/architecture/ or docs/decisions/ only when enough decisions exist to
justify those directories.

Avoid documentation fragmentation.

