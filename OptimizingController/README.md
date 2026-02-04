# Optimizing Controller

This folder contains the implementation of the three adaptation modules 
used in the study. 

PI- and RL-module: [./src/controller/optimizing_controller.rs](./src/controller/optimizing_controller.rs). <br> 

Adaptive control (AC)-module: [./src/controller/xup_state.rs](./src/controller/xup_state.rs) and [./src/kalman_filter.rs](./src/kalman_filter.rs). <br>

These adaptation modules perform regular local adaptation at the application- or system-level, slowing down/speeding up, as needed.
