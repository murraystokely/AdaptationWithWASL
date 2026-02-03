# A Guide to using the overall runtime system involving WASL

This guide provides the prerequisites and the steps required to setup the system as per Algorithms 1 and 2 in the paper.

## Code Organization

The following independent modules work in conjunction for the functioning of the overall runtime system (top-down as in Fig.5):

1. [apto-tailbench-apps](./apto-tailbench-apps/) -- Wrappers around an application [TailBench](https://github.com/adaptsyslearn/TailBenchMod)
   that profile specified parameters to the processing/activation layer (Apto).
2. [apto](./apto) -- A layer that application(s) and the system can use to process the profiled parameters, i.e. a rust implementation of the [GOAL](https://dl.acm.org/doi/pdf/10.1145/3563835.3567655) work.
3. [OptimizingController](./OptimizingController) -- Local adaptation module(s) for the system and the application.
   
   Algo.1 involves 1, 2, and 3. 
4. [WASL](./PoleAdaptation) -- The novel multi-module adaptation method as proposed in Algo.2 of the paper.
   

Interactions between the modules:

a. Applications connect to `apto-tailbench-apps` using a linux message queue to report performance information.<br>
b. `apto-tailbench-apps` sets up `apto` with a specific local adaptation method and an application goal. 
`apto-tailbench-apps` communicates the information it receives from the (tailbench) applications to `apto`. <br> 
c. `apto` uses this information along with the `OptimizingController` for resource adjustments to achieve an application's goals. <br> 
d. The `OptimizingController` (adaptation method) invokes WASL (`PoleAdaptation`) as and when needed to address multi-module multi-tenant (global) interference.

## Prerequisites

User either needs *root* access, or provide access to the binaries to read energy consumption data of the system.

1. [Energymon](https://github.com/energymon/energymon): Install the implementation that is appropriate for your system.
2. [Rust](https://rust-lang.org/tools/install/): Use standard configuration that allow using `cargo`.
3. A [modified version](https://github.com/adaptsyslearn/TailBenchMod) of [TailBench](https://tailbench.csail.mit.edu/) provided with
   this repository and related tailBench inputs.
  

## Setup

1. Download all the repositories provided in this project into your chosen location
2. Update `Cargo.toml` files in `apto-tailbench-apps`, `apto` and `OptimizingController` accordingly.
3. Compile tailbench applications using the instructions provided.
4. Update the location of the tailbench binaries in `apto-tailbench-apps/src/apps.rs` to reflect related file paths.
5. Compile `apto-tailbench-apps` using `cargo build --release --bin main` inside `apto-tailbench-apps` directory.

## Profiling applications

`apto` needs a `measuretable` for an application to adapt to an application's goals.<br>
`apto-tailbench-apps` can be used to obtain the `measuretable` (as in Table.2 in the paper) after formulating a suitable `knobtable`. <br>


A `knobtable` is an enumeration of valid configurations (as in Table.3 in the paper) that `apto` can use. 
A sample `knobtable` for an application can be found in [./apto-tailbench-apps/knobtable](./apto-tailbench-apps/knobtable). 
Once this table is formed based on the available system resources, an application can be profiled as follows:

```
cd ./apto-tailbench-apps
$ cargo build --release --bin main
$ sudo RUST_LOG=info PROFILE=1000 ./target/release/main --warmup-time <WARMUP-SECONDS> profile <APPLICATION-NAME>
```

This command outputs a file named `measuretable`. 
It is recommended that the `knobtable` (kt) and `measuretable` (mt) be renamed to `<APPLICATION-NAME>.kt` and `<APPLICATION-NAME>.mt`, respectively. <br>


## Running Adaptation-related Experiments

The overall runtime system executes all application(s) and system as modules. Each module is assigned a tag.<br>
For a single application scenario, the application is always assigned the tag 0, while the system the tag 1. <br>
For a multi application scenario, the applications are always assigned the tag 0 and 1, while the system the tag 2.

### Selecting Adaptation Method
An application or system is executed with specific (local) adaptation method. 
This specification is controlled by environment variables as follows:
```
# Run with RL Reinforcement Learning (RL)-based adaptation
LEARNING_BASED_<TAG>=y CONF_TYPE_<TAG>=multi

# Run with Adaptive Control (AC) Module
CONF_TYPE_<TAG>=multi

# Run with PI control
CONT_TYPE_<TAG>=multi KP_<TAG>=<PROPORTIONAL_GAIN_VALUE>
```

### WASL Invocation by Adaptation Module(s)

An adaptive module can invoke WASL using environment variables as follows:
```
ADAPT_TYPE=linear ADAPT_INST=<LIST-OF-TAGS> DEV_TARGET=<GAMMA>
```

### Experimenting with Adaptation Module(s) for Application(s)

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
The resulting parameters for each iteration are printed to `stdout` and may be dumped to a file.
