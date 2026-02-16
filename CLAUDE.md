# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Research artifact for the paper "WASL: Harmonizing Uncoordinated Adaptive Modules in Multi-Tenant Cloud Systems" (ICPE 2026). WASL is a rate-adaptation technique for runtime cross-layer coordination in multi-tenant clouds to mitigate performance interference from colocated adaptive applications.

## Build and Run

The project is pure Rust with 4 independent crates. The main entry point is `apto-tailbench-apps`.

```bash
# Build the main binary (from repo root or apto-tailbench-apps/)
cd ./apto-tailbench-apps
cargo build --release --bin main

# Profile an application
sudo RUST_LOG=info PROFILE=1000 ./target/release/main --warmup-time <SECS> profile <APP-NAME>

# Run monolithic (single app + system, centralized)
sudo <ENV_VARS> ./target/release/main --exec-time <SECS> monolithic <APP> <TARGET>

# Run multimodule (uncoordinated adaptation modules)
sudo <ENV_VARS> ./target/release/main --exec-time <SECS> multimodule <APP> <TARGET>

# Run multi-app multimodule
sudo <ENV_VARS> ./target/release/main --exec-time <SECS> multimodule <APP0> <T0> <APP1> <T1>
```

**Run tests** (standard cargo test in each crate):
```bash
cargo test                    # in any crate directory
cargo test -- --ignored       # run ignored/integration tests
```

**Simulation** (OptimizingController):
```bash
cd OptimizingController && bash run_sim.sh
```

## Architecture

Five-layer stack (top-down, matching Fig.5 in paper):

```
TailBench Applications (dnn, masstree, moses, silo, xapian, sphinx, nginx)
        ↓  Linux message queue
apto-tailbench-apps    — Wrapper/profiler: captures app metrics, CLI entry point
        ↓
apto                   — GOAL processing/activation layer: knobs, measures, profiles
        ↓
OptimizingController   — Local adaptation: PI control, RL, Adaptive Control
        ↓
PoleAdaptation         — WASL global multi-module coordination (novel contribution)
```

**Data flow:** Applications report latency via Linux message queues → `apto-tailbench-apps` feeds metrics to `apto` → `apto` decides configurations using knob tables (kt) and measure tables (mt) → `OptimizingController` applies local adaptation → `PoleAdaptation` (WASL) harmonizes cross-module interference when invoked.

## Crate Dependencies

```
apto-tailbench-apps → apto → OptimizingController → PoleAdaptation
                       ↑                              ↑
                       └──────────────────────────────┘
```

`apto-tailbench-apps` depends on `apto` and `PoleAdaptation`. `apto` depends on `OptimizingController` and `PoleAdaptation`. `OptimizingController` depends on `PoleAdaptation`. Cargo.toml files use local path dependencies that may need updating for your environment.

## Key Modules

- **apto/src/optimize.rs** — Core `Apto` struct orchestrating adaptation loops
- **apto/src/goal.rs** — Goal definitions (constraints, objectives)
- **apto/src/knobs/** — Tunable parameters (CPU freq, cores, cache, hyperthreading)
- **OptimizingController/src/controller/optimizing_controller.rs** — PI and RL control implementations
- **OptimizingController/src/controller/xup_state.rs** — Adaptive control state
- **PoleAdaptation/src/lib.rs** — WASL algorithm (linear, EWMA, model-based strategies)
- **apto-tailbench-apps/src/bin/main.rs** — CLI with subcommands: Monolithic, Multimodule, MultiApp, Profile
- **apto-tailbench-apps/src/apps.rs** — TailBench application wrappers (hardcoded binary paths that need updating)

## Environment Variables for Adaptation

Each module gets a tag (app=0, system=1 for single-app; apps=0,1, system=2 for multi-app):

| Method | Variables |
|--------|-----------|
| RL-based | `LEARNING_BASED_<TAG>=y CONF_TYPE_<TAG>=multi` |
| Adaptive Control | `CONF_TYPE_<TAG>=multi` |
| PI Control | `CONT_TYPE_<TAG>=multi KP_<TAG>=<value>` |
| WASL | `ADAPT_TYPE=linear ADAPT_INST=<tag-list> DEV_TARGET=<gamma>` |

## Profile Data

- Knob tables (`.kt`): Valid configuration space in `apto-tailbench-apps/profiles/`
- Measure tables (`.mt`): Performance profiles in `apto-tailbench-apps/profiles/`
- Multi-app variants in `profiles/multi/`
- Helper scripts in `apto-tailbench-apps/scripts/` (Python)

## Prerequisites

- Root access (recommended) for energy monitoring
- [Energymon](https://github.com/energymon/energymon) library installed
- Rust toolchain with cargo
- [Modified TailBench](https://github.com/adaptsyslearn/TailBenchMod) applications compiled
- Linux with message queue and CPU frequency/core control support (x86)
