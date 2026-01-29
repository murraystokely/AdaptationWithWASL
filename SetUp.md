
************** Overall Setup Guide *******************


In general, sudo/root access needed for seamless execution. 

1. Install RUST with all its dependencies, and test Rust compilation

   curl --proto '=https' --tlsv1.3 https://sh.rustup.rs -sSf | sh <br>
   (default standard installation)

   rustup update

3. Successfully precompile updated version of [TailBench](https://github.com/adaptsyslearn/TailBenchMod) applications

4. Download the directories:

	a. apto <br>
	b. apto-tailbench-apps <br>
	c. OptimizingController <br>
    d. PoleAdaptation <br>
	e. tailbench and tailbench inputs

5. Update relative/absolute paths and links in Cargo.toml files and 
   apto-tailbench-apps/src/apps.rs as needed. 
   The .mt and .kt file paths may need to be also updated. 

6. Execute:

cd apto-tailbench-apps
Run: cargo build --release --bin main


6. For data collection and interference mitigation, profiling and adaptation 
experiments have to be run separately in different modes,
e.g., single module, multi-module, RL, PI, monolithic etc. 

Sample scripts are in apto-tailbench-apps/experiment_scripts folder.

e.g., bash experiment_scripts/run.sh single

