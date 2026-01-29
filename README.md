# WASL

This repo contains source code and other artifacts related to the paper 
"WASL: Harmonizing Uncoordinated Adaptive Modules in Multi-Tenant Cloud Systems". 
WASL is a rate-adaptation based technique for runtime multi-module coordination 
in multi-tenant clouds to mitigate performance interference arising due to 
multiple colocated adaptive applications. 

TailBench applications have been used for evaluation. 
Tailbench details are [here](https://github.com/adaptsyslearn/TailBenchMod).

### Code Structure
```bash
/                           : Overall Runtime System
|-- OptimizingController    : Adaptation Module 
|-- PoleAdaptation          : WASL-based Rate Adaptation
|-- apto-tailbench-apps     : Wrapper/Profiler for Application/System with
                              helper scripts for profiling or data collection
|-- apto                    : Processing and Activation,
                              coordination with the Adaptation Module
|-- SetUp.md                : Readme about the High-level setup of the system
|-- Plots                   : Scripts related to some results
```
TailBench                   : Updates to standard TailBench suite used for experiments 
Readme files for each of the folders are being updated for documentation. 

## Citation

The following paper can be cited:

```
@inproceedings{DBLP:conf/icpe/Pervaiz26,
  author       = {Ahsan Pervaiz, Anwesha Das, Vedant Kodagi, 
                  Muhammad Husni Santriaji, Henry Hoffmann},
  title        = {WASL: Harmonizing Uncoordinated Adaptive Modules 
                  in Multi-Tenant Cloud Systems},
  booktitle    = {International Conference on Performance Engineering, {ICPE}},
  publisher    = {{ACM/SPEC}},
  year         = {2026} 
}
```
