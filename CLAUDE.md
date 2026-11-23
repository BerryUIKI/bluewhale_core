# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Current status

This repository was empty when this file was created on 2026-11-23: no source code, package manifest, build system, tests, or CI. If that no longer matches the repository, update this section and replace the guidance below with the actual architecture and commands.

## Purpose and release boundary

`bluewhale_core` is the shared core for the Bluewhale product line and is intended to be open source. The Windows and Android client applications live in the sibling `bluewhale_clients` repository. `bluewhale_server` is planned to remain closed source, and `bluewhale-ledger` is a proprietary, Apple-native application.

Rules:

- The private Apple application (`bluewhale-ledger`) is the behavioral reference for this core and is not modified by this project. Do not copy its source code, assets, test fixtures, or design documents into this public repository without an explicit, recorded licensing decision. Its `Packages/BerryLedgerCore` Swift package is a private, Apple-oriented implementation, not a source to lift wholesale.
- Keep this repository self-contained and buildable without closed-source components.
- The project is licensed under the Apache License, Version 2.0 (`LICENSE`), with attribution in `NOTICE` (Copyright 2026 BerryUIKI). Keep both files intact and clear any dependency-license question before adding a dependency.
- Do not introduce dependencies on a specific client UI framework or on closed-source server internals.

## Development workflow

Development follows trunk-based development. Keep `main` releasable, commit small focused changes directly to `main`, and avoid long-lived feature branches. Use a short-lived branch only when a change genuinely needs isolation.

## Development guidance

There are no build, lint, or test commands yet. When the project skeleton lands, add to this file:

- The exact build, lint, test, and single-test commands.
- The chosen language, toolchain versions, package layout, and module boundaries.
- The public API and versioning policy, plus the supported consumer platforms.
- How the Windows and Android clients in `bluewhale_clients` consume this package.

Do not invent a stack, architecture, or command set before it is actually decided and recorded here.
