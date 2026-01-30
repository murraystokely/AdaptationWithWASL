# A Guide to using WASL

This guide provides the prerequisites and the steps required to setup WASL and the control algorithms presented in the paper.

## Code Organization

This respository provides several independent projects that need to be used together to repeat the experiments presented in the paper.
1. [OptimizingController](./OptimizingController) -- Adaptation modules that provide the configurations to use for the system and the application.
2. [apto](./apto) -- A middle layer that applications and underlying systems can use to monitor metrics and adjust knobs. A rust implementation of [GOAL](https://dl.acm.org/doi/pdf/10.1145/3563835.3567655)
3. [apto-tailbench-apps](./apto-tailbench-apps/) -- Wrappers around [modified version](https://github.com/adaptsyslearn/TailBenchMod) that report metric information to Apto.

At a high-level, Tailbench applications connect to `apto-tailbench-apps` using a linux message queues to report performance information. `apto-tailbench-apps` is responsible for setting up `apto` with the type of adaptation and the goal of the application that needs to be achieved. `apto-tailbench-apps` passes the information that it receives from the tailbench applications to `apto`. `apto` uses this information and the `OptimizingController` to determine the configurations that need to be used to achieve the application's goals. The `OptimizingController` calls into `PoleAdaptation` (WASL) to determine the rate at which adaptation should be performed.

## Prerequisites

To run experiments, the user either needs to have root access, or provide access to the binaries to read energy/power consumption data of the underlying system.

1. [Energymon](https://github.com/energymon/energymon): Install the implementation that is appropriate for your system.
2. [Rust](https://rust-lang.org/tools/install/): Use the default or any sane configurations that allow using `cargo`.
3. A [modified version](https://github.com/adaptsyslearn/TailBenchMod) of [TailBench](https://tailbench.csail.mit.edu/) that is provided with this repository.
  a. Remember to download tailbench inputs.

## Setup

1. Download all of the repositories provided in this project into your chosen locations and update the `Cargo.toml` files in `apto-tailbench-apps`, `apto` and `OptimizingController` accordingly.
2. Update the location of the tailbench binaries in `apto-tailbench-apps/src/apps.rs` to reflect your configuration.
3. Compile tailbench applications using the instructions provided in that project.
4. Compile `apto-tailbench-apps` using `cargo build --release --bin main` inside `apto-tailbench-apps` directory.

## Profiling applications

`apto` needs a `measuretable` for an application for it to adapt to application's goals.

This `apto-tailbench-apps` can be used to obtain this `measuretable` but you must first provide a `knobtable`. A `knobtable` is simply an enumeration of all valid configurations that the `apto` can use. A sample knobtable for the applications can be found in [./apto-tailbench-apps/knobtable](./apto-tailbench-apps/knobtable). Once this knobtable is available an application can be profiled as follows:

```
cd ./apto-tailbench-apps
$ cargo build --release --bin main
$ sudo RUST_LOG=info PROFILE=1000 ./target/release/main --warmup-time <WARMUP-SECONDS> profile <APPLICATION-NAME>
```

This command outputs a file named `measuretable`. It is recommended that the `knobtable` and `measuretable` be renamed to `<APPLICATION-NAME>.kt` and `<APPLICATION-NAME>.mt` respectively.

## Running Experiments

The aforementioned infrastructure runs all applications and systems as modules. Each module is assigned a tag. For a single application scenario, the application is always assigned the tag 0 and the system is always assigned the tag 1. Similarly, for a multi application scenario, the applications in always assigned the tag 0 and 1 and the system is assigned the tag 2.

### Selecting adaptation module type
Each module, can be ran with a different adaptation type. The adaptation type is controlled using environment variables as follows:
```
# Run with learning based adaptation
LEARNING_BASED_<TAG>=y CONF_TYPE_<TAG>=multi

# Run with adaptive control
CONF_TYPE_<TAG>=multi

# Run with PI control
CONT_TYPE_<TAG>=multi KP_<TAG>=<PROPORTIONAL_GAIN_VALUE>
```

### Enabling WASL for adaptation modules

Similarly, WASL can be enabled for each adaptation module using environment variables as follows:
```
ADAPT_TYPE=linear ADAPT_INST=<LIST-OF-TAGS> DEV_TARGET=<GAMMA>
```

### Selecting the applications

Finally, combine the aforementioned environment variables to run an experiment as follows:
```
cd ./apto-tailbench-apps/
cargo build --release --bin main

# Run one application and a system module with a monolithic adaptation
sudo <ENVIRONMENT_VARIABLES> ./target/release/main --exec-time <EXECUTION-TIME-SECS> monolithic \
    <APPLICATION-NAME> <APPLICATION-TARGET>

# Run one application and a system module with multiple uncoordinated adaptation modules
sudo <ENVIRONMENT_VARIABLES> ./target/release/main --exec-time <EXECUTION-TIME-SECS> multimodule \
    <APPLICATION-NAME> <APPLICATION-TARGET>

# Run two applications and a system module with multiple uncoordinated adaptation modules
sudo <ENVIRONMENT_VARIABLES> ./target/release/main --exec-time <EXECUTION-TIME-SECS> multimodule \
    <APPLICATION-0-NAME> <APPLICATION-0-TARGET> \
    <APPLICATION-1-NAME> <APPLICATION-1-TARGET>
```
The resulting metric values per iteration are printed to `stdout` and should be piped to a file.
