# Optimizing Controller

Implementation of the adaptation modules that choose are used to adapt to changes in inputs and operating environment.

The implementation of the adaptation modules is found in [./src/controller/optimizing_controller.rs](./src/controller/optimizing_controller.rs). Adaptive control-specific details are found in [./src/controller/xup_state.rs](./src/controller/xup_state.rs) and [./src/kalman_filter.rs](./src/kalman_filter.rs).

At a high-level the controllers are responsible for determining the rate at which the application/system needs to be slowed down or sped up.
