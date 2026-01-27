use crate::measures::Measurement;
use crate::profile::ActiveModel;
use crate::system_measures::Energymon;
use crate::AptoConfig as Configurations;
use crate::NAME_REGEX;
use crate::{Goal, Perturbation};
use itertools::Itertools;
use log::{info, trace, warn};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::Path;
use std::time::Instant;
use OptimizingController::Controller;

type Schedule = (u64, u64, u64);

pub(crate) enum AptoMode {
    Profile(u64),
    Adaptive,
    NonAdaptive,
}

struct AptoState {
    measurements: HashMap<String, Measurement>,
    active_model: ActiveModel,
    controller: Controller,
    sched: Schedule,
    mode: AptoMode,
}

impl AptoState {
    fn new(config: &Configurations<u64>) -> AptoState {
        // We don't really need this because
        // energyDelta, powerConsumption and windowLatency
        // will be reported only once per window any ways

        println!("MeasureTable:");
        println!("Names: {:?}", config.measure_table.names);
        println!("Profile: {:?}", config.measure_table.profile);


        let window_size = config.window_size;
        let measurements = config
            .measure_table
            .names
            .iter()
            .filter_map(|name| match name.as_str() {
                "id" => None,
                "powerConsumption" | "harmonicMean" | "geometricMean" | "harmonicMeanABS" => {
                    Some((
                        String::from(name),
                        Measurement::new(
                            config.window_size,
                            Some(Box::new(|vals| *vals.last().unwrap())),
                        ),
                    ))
                }
                "windowLatency" => Some((
                    String::from(name),
                    Measurement::new(config.window_size, None),
                )),
                "energyDelta" => Some((
                    String::from(name),
                    Measurement::new(
                        config.window_size,
                        Some(Box::new(move |vals| {
                            vals.last().unwrap() / (window_size as f64)
                        })),
                    ),
                )),
                name => Some((
                    String::from(name),
                    Measurement::new(config.window_size, None),
                )),
            })
            .collect();
        trace!(
            "Initialized measuring devices for instance {}.",
            config.instance_id
        );
        println!(" Checking constraint '{}' in {:?}", config.goal.constraint, config.measure_table.names);

        let constraint_idx = config.measure_table.constraint_idx(&config.goal.constraint);

        let mut active_model = ActiveModel::new(&config.measure_table, &config.knob_table);
        
        println!("Config Knobs:");
        for (key, _value) in &config.knobs {
            println!("Knob Name: {}", key);
        }

        let nr_configs_removed = active_model.restrict_model(&config.knobs);
        warn!(
            "{} configs were filtered out (remaining {}) for instance {}",
            nr_configs_removed,
            active_model.configs.len(),
            config.instance_id
        );
        active_model.sort_by_constraint(constraint_idx);

        let mode = if let Ok(num_iterations) = std::env::var("PROFILE") {
            let num_iterations = num_iterations.parse().unwrap_or_else(|e| {
                panic!(
                    "Could not convert profiling iterations ({}) : {:?}",
                    num_iterations, e
                )
            });
            AptoMode::Profile(num_iterations)
        } else {
            AptoMode::Adaptive
        };

        let controller = AptoState::init_controller(config, &active_model, constraint_idx);

        let initial_config_idx = active_model.find_id(&config.knobs).unwrap_or(0) as u64;
        let sched = (initial_config_idx, initial_config_idx, config.window_size);

        AptoState {
            measurements,
            active_model,
            controller,
            sched,
            mode,
        }
    }

    fn init_controller(
        config: &Configurations<u64>,
        active_model: &ActiveModel,
        constraint_idx: usize,
    ) -> Controller {
        let obj_measures: Vec<String> = NAME_REGEX
            .find_iter(&config.goal.opt_func)
            .map(|f| String::from(f.as_str()))
            .collect();
        let obj_measure_indices: Vec<usize> = obj_measures
            .iter()
            .map(|needle| {
                config
                    .measure_table
                    .names
                    .iter()
                    .position(|haystack| needle == haystack)
                    .unwrap_or_else(|| {
                        panic!(
                            "Measure ({}) not found in measure table header",
                            needle.as_str()
                        )
                    })
            })
            .collect();

        let filtered_model = active_model.cost_model(&obj_measure_indices);
        let initial_config_idx = active_model.find_id(&config.knobs).unwrap_or(0) as u64;

        
        Controller::new(
            config.instance_id as u64,
            active_model.measure_values(),
            filtered_model,
            config.goal.target,
            constraint_idx,
            config.window_size as usize,
            config.goal.opt_type,
            &config.goal.opt_func,
            obj_measures,
            initial_config_idx as usize,
        )
    }
}

pub struct Apto {
    state: AptoState,
    configurations: Configurations<u64>,
    outfiles: HashMap<&'static str, BufWriter<File>>,
}

impl Apto {
    pub fn new(configs: Configurations<u64>) -> Apto {
        let state = AptoState::new(&configs);
        info!(
            "Initialized Apto (instance {}) to {} with window {}",
            configs.instance_id, configs.goal, configs.window_size
        );
        let mut new_apto = Apto {
            state,
            configurations: configs,
            outfiles: HashMap::new(),
        };
        new_apto.apply_knob_settings(u64::MAX, 0);
        new_apto
    }

    pub fn optimize<'a>(
        &'a mut self,
        mut stream_initializer: Option<Box<dyn Fn() + 'a>>,
        mut stream_deinitializer: Option<Box<dyn Fn() + 'a>>,
        mut main_loop: Box<dyn FnMut(&mut Apto) -> bool + 'a>,
    ) -> bool {
        if let AptoMode::Profile(num_iterations) = self.state.mode {
            return self.profile(
                num_iterations as usize,
                stream_initializer,
                stream_deinitializer,
                main_loop,
            );
        }

        let mut energy_monitor = Energymon::new();

        let mut iteration = 0u64;
        let mut current_config = u64::MAX;

        if let Some(stream_init) = stream_initializer.as_mut() {
            stream_init();
        }

        loop {
            let output = self.run_application_body(
                iteration,
                current_config,
                &mut energy_monitor,
                &mut main_loop,
            );

            let should_terminate = output.0;
            iteration = output.1;
            current_config = output.2;

            if should_terminate {
                break;
            }
        }

        if let Some(stream_deinit) = stream_deinitializer.as_mut() {
            stream_deinit();
        }

        true
    }

    fn write_knob_table(&self, knob_names: &[String], configs: &[Vec<(String, u64)>]) {
        if Path::new("knobtable").exists() {
            let _ = std::fs::remove_file("knobtable");
        }
        let mut kt_file = OpenOptions::new()
            .write(true)
            .create(true)
            .open("knobtable")
            .map(BufWriter::new)
            .unwrap_or_else(|e| panic!("Could not open knobtable: {:?}", e));
        // Write knob table header
        let kt_header: String = knob_names.iter().join(",");
        let _ = kt_file.write(b"id,");
        let _ = kt_file.write(kt_header.as_bytes());
        let _ = kt_file.write(b"\n");

        for (idx, config) in configs.iter().enumerate() {
            let _ = kt_file.write(format!("{},", idx).as_bytes());
            let _ = kt_file.write(
                config
                    .iter()
                    .map(|(_, val)| val.to_string())
                    .join(",")
                    .as_bytes(),
            );
            let _ = kt_file.write(b"\n");
        }
        let _ = kt_file.flush();
    }

    fn prepare_profile_tables(&self) -> (Vec<Vec<(String, u64)>>, Vec<String>, BufWriter<File>) {
        let knob_names: Vec<String> = self.configurations.knobs.keys().cloned().sorted().collect();

        let all_knob_values_iter = knob_names.iter().map(|name| {
            self.configurations.knobs[name]
                .possible_values()
                .iter()
                .map(|&value| (name.clone(), value))
                .collect::<Vec<(String, u64)>>()
        });
        let all_configurations: Vec<Vec<(String, u64)>> = all_knob_values_iter
            .map(|e| e.into_iter())
            .multi_cartesian_product()
            .collect();

        self.write_knob_table(&knob_names, &all_configurations);

        let measure_names: Vec<String> = self
            .configurations
            .measure_table
            .names
            .iter()
            .filter(|&name| name != "id")
            .cloned()
            .sorted()
            .collect();

        let mt_file = if Path::new("measuretable").exists() {
            OpenOptions::new()
                .append(true)
                .open("measuretable")
                .map(BufWriter::new)
                .unwrap_or_else(|e| panic!("Could not open measuretable: {:?}", e))
        } else {
            let mut mt_file = OpenOptions::new()
                .write(true)
                .create(true)
                .open("measuretable")
                .map(BufWriter::new)
                .unwrap_or_else(|e| panic!("Could not open measuretable: {:?}", e));

            // Write measure table header
            let _ = mt_file.write(b"id,");
            let _ = mt_file.write(measure_names.iter().join(",").as_bytes());
            let _ = mt_file.write(b"\n");
            let _ = mt_file.flush();

            mt_file
        };

        (all_configurations, measure_names, mt_file)
    }

    fn apply_static_profiling_config(&'_ mut self, config: &Vec<(String, u64)>) {
        for (name, value) in config {
            if name == "hyperthreading" || name == "cacheCOS" {
                continue;
            }
            self.configurations.knobs.get(name).unwrap().set(*value);
        }
        let special_knobs = config.iter().fold([None, None], |mut acc, (name, value)| {
            match name.as_str() {
                "hyperthreading" => acc[0] = Some((name, value)),
                "cacheCOS" => acc[1] = Some((name, value)),
                _ => (),
            };
            acc
        });
        for &(name, value) in special_knobs.iter().flatten() {
            self.configurations.knobs.get(name).unwrap().set(*value);
        }
    }

    fn profile<'a>(
        &'a mut self,
        num_iterations: usize,
        mut stream_initializer: Option<Box<dyn Fn() + 'a>>,
        mut stream_deinitializer: Option<Box<dyn Fn() + 'a>>,
        mut main_loop: Box<dyn FnMut(&mut Apto) -> bool + 'a>,
    ) -> bool {
        let (configurations, ordered_measure_names, mut mt_file) = self.prepare_profile_tables();

        // Incase the application has been partially profiled
        let nr_to_skip = std::env::var("PROFILE_SKIP").map(|nr| nr.parse::<usize>().unwrap());

        let mut energy_monitor = Energymon::new();

        for (idx, config) in configurations.iter().enumerate() {
            if let Ok(nr) = nr_to_skip {
                if idx <= nr {
                    println!("Continuing for configuration {}", idx);
                    continue;
                }
            }

            println!("Profiling: ({}) {:?}", idx, config);

            // Re-initialize stream for every configuration
            if let Some(stream_init) = stream_initializer.as_mut() {
                stream_init();
            }

            // MAYBE: We should read start and end energy here and then compute power consumption separately
            //        we're not doing this up front because we want any bias to be included in the readings we
            //        give to the controller.

            self.apply_static_profiling_config(config);

            for i in 0..num_iterations {
                let _ = self.run_application_body(i as u64, 0, &mut energy_monitor, &mut main_loop);
            }

            if let Some(stream_deinit) = stream_deinitializer.as_mut() {
                stream_deinit();
            }

            let mut measured_values: HashMap<&str, f64> = ordered_measure_names
                .iter()
                .map(|name| {
                    (
                        name.as_str(),
                        self.state
                            .measurements
                            .get(name)
                            .unwrap_or_else(|| panic!("No measure found for {}", name))
                            .total_average(),
                    )
                })
                .collect();
            // Update performance and power consumption from total averages
            *measured_values.get_mut("performance").unwrap() = 1.0 / measured_values["latency"];
            *measured_values.get_mut("powerConsumption").unwrap() =
                measured_values["energyDelta"] / measured_values["windowLatency"];

            // Only take the last value for the harmonic and geometric means
            if measured_values.contains_key("harmonicMean") {
                *measured_values.get_mut("harmonicMean").unwrap() = self
                    .state
                    .measurements
                    .get("harmonicMean")
                    .unwrap()
                    .aggregate();
                *measured_values.get_mut("geometricMean").unwrap() = self
                    .state
                    .measurements
                    .get("geometricMean")
                    .unwrap()
                    .aggregate();
                *measured_values.get_mut("geometricMean").unwrap() = self
                    .state
                    .measurements
                    .get("harmonicMeanABS")
                    .unwrap()
                    .aggregate();
            }

            info!("Writing measuretable line.");
            // Write total averages of observed measures
            let _ = mt_file.write(format!("{},", idx).as_bytes());
            let _ = mt_file.write(
                ordered_measure_names
                    .iter()
                    .map(|name| measured_values[name.as_str()])
                    .join(",")
                    .as_bytes(),
            );
            let _ = mt_file.write(b"\n");
            let _ = mt_file.flush();

            // Reset all averages
            println!("Resetting measures");
            for device in self.state.measurements.values_mut() {
                device.reset_complete();
            }
        }

        let _ = mt_file.flush();
        true
    }

    fn run_application_body<F>(
        &mut self,
        mut iteration: u64,
        mut current_config: u64,
        energy_monitor: &mut Energymon,
        main_loop: &mut F,
    ) -> (bool, u64, u64)
    where
        F: FnMut(&mut Apto) -> bool,
    {
        let start_instant = Instant::now();

        // Update knobs
        iteration = iteration.wrapping_add(1);

        // Record power numbers across windows during execution
        if iteration == 1 || iteration % self.configurations.window_size == 0 {
            if iteration > 1 {
                energy_monitor.stop();

                self.measure("energy", energy_monitor.energy());
                let energy_delta = match energy_monitor.energy_delta() {
                    Ok(energy_delta) => energy_delta,
                    Err(e) => {
                        trace!("{} (instance {})", e, self.configurations.instance_id);
                        self.state.measurements["energyDelta"]
                            .prev_value()
                            .unwrap_or(0.0)
                    }
                };

                self.measure("energyDelta", energy_delta);

                let power_consumption = match energy_monitor.power_consumption() {
                    Ok(power_consumption) => power_consumption,
                    Err(e) => {
                        trace!("{} (instance {})", e, self.configurations.instance_id);
                        self.state.measurements["powerConsumption"]
                            .prev_value()
                            .unwrap_or(0.0)
                    }
                };
                self.measure("powerConsumption", power_consumption);
                self.measure("windowLatency", energy_monitor.duration().unwrap());
                info!(
                    "REPLACE: instance:{},powerConsumption:{},energyDelta:{},windowLatency:{}",
                    self.configurations.instance_id,
                    power_consumption,
                    energy_delta,
                    energy_monitor.duration().unwrap()
                );
            }

            energy_monitor.start();
        }

        current_config = self.actuate_knobs(iteration, current_config);

        // Execute application loop
        let should_terminate = !main_loop(self);

        self.measure("iteration", iteration as f64);

        // Report all non-degenerate measures
        let latency = start_instant.elapsed().as_secs_f64();
        if latency > 0.0 {
            let performance = 1.0 / latency;
            self.measure("latency", latency);
            self.measure("performance", performance);
        } else {
            trace!(
                "Zero time spent in interation {} (instance id: {})",
                iteration,
                self.configurations.instance_id
            );
        }

        self.log_state();

        (should_terminate, iteration, current_config)
    }

/*
 try to call the tool here
*/

    fn actuate_knobs(&mut self, iteration: u64, current_config: u64) -> u64 {
        match self.state.mode {
            AptoMode::Profile(_) => 0,
            AptoMode::Adaptive => {
                if iteration % self.configurations.window_size == 0 {
                    let constraint_average = self
                        .state
                        .measurements
                        .get(&self.configurations.goal.constraint)
                        .expect("Could not read constraint measurement for computing schedule.")
                        .aggregate();
                    let measurement_difference = (self.state.controller.sched_xup*(1.0/self.state.controller.kf.x_hat)) - constraint_average;
                    let x_hat = self.state.controller.kf.x_hat;
                    let multiplier = self.state.controller.pole_adaptation.calculate_multiplier(measurement_difference, constraint_average, x_hat);
                    info!("New multiplier {}",multiplier);
                    
                    let sched = self.state.controller.compute_schedule(constraint_average, multiplier);
    
                    info!(
                        "Obtained new schedule {:?} for window average {} (instance {})",
                        sched, constraint_average, self.configurations.instance_id
                    );
                    self.state.sched = sched;
                    for (_, device) in self.state.measurements.iter_mut() {
                        device.reset_window();
                    }
                }
                self.apply_knob_settings(current_config, iteration)
            }
            AptoMode::NonAdaptive => self.apply_knob_settings(current_config, iteration),
        }
    }
    

    pub fn measure(&mut self, name: &str, value: f64) {
        if !self.state.measurements.contains_key(name) {
            self.state.measurements.insert(
                name.to_string(),
                Measurement::new(self.configurations.window_size, None),
            );
            warn!(
                "Initialized measurement for {} in instance {}",
                name, self.configurations.instance_id
            );
        }
        self.state
            .measurements
            .get_mut(name)
            .unwrap_or_else(|| panic!("Expected measure {} to be in measurements", name))
            .register_value(value, &self.state.mode);
    }

    fn apply_knob_settings(&mut self, current: u64, iteration: u64) -> u64 {
        let sched = self.state.sched;
        let idx = if iteration % self.configurations.window_size >= sched.2 {
            sched.1
        } else {
            sched.0
        };

        if current == idx {
            return current;
        }

        let knob_settings = self.state.active_model.get_knob_settings(idx as usize);
        info!(
            "Setting Knobs to ({}){:?} based on sched {:?} (instance {})",
            idx, knob_settings, sched, self.configurations.instance_id
        );

        for (name, knob) in &self.configurations.knobs {
            if name == "hyperthreading" || name == "cacheCOS" {
                // Special case for hyperthreading and cacheCOS
                continue;
            }
            knob.set(*knob_settings.get(name).unwrap());
        }

        // Special case: If we have hyperthreading then we must readjust it
        //               to make sure that affinity masks conforms with the
        //               hyperthreading knob
        // Special case: If we have cacheCos then we need to apply masks to cores
        //               in use by the process
        for name in ["hyperthreading", "cacheCOS"] {
            // if let Some(knob) = self.configurations.knobs.get(name) {
            //     knob.set(*knob_settings.get(name).unwrap());
            // }
        }

        idx
    }

    pub fn change_goal(&mut self, goal: Goal) {
        let difference = &goal - &self.configurations.goal;
        info!(
            "Perturbing goal to {} (instance {})",
            difference, self.configurations.instance_id
        );
        match difference {
            Perturbation::NoChange => info!(
                "Change goal applied with the same goal (instance {})",
                self.configurations.instance_id
            ),
            Perturbation::ChangeObjective(opt_type, opt_func) => {
                self.configurations.goal.opt_func = opt_func;
                self.configurations.goal.opt_type = opt_type;
                let obj_measures: Vec<String> = NAME_REGEX
                    .find_iter(&self.configurations.goal.opt_func)
                    .map(|f| String::from(f.as_str()))
                    .collect();
                let obj_measure_indices: Vec<usize> = obj_measures
                    .iter()
                    .map(|needle| {
                        self.configurations
                            .measure_table
                            .names
                            .iter()
                            .position(|haystack| needle == haystack)
                            .unwrap_or_else(|| {
                                panic!(
                                    "Measure ({}) not found in measure table header",
                                    needle.as_str()
                                )
                            })
                    })
                    .collect();
                let filtered_cost_model = self.state.active_model.cost_model(&obj_measure_indices);
                self.state.controller.change_objective(
                    self.configurations.goal.opt_type,
                    &self.configurations.goal.opt_func,
                    obj_measures,
                    filtered_cost_model,
                );
            }
            Perturbation::ChangeConstraintValue(new_value) => {
                self.configurations.goal.target = new_value;
                self.state.controller.change_target(new_value);
            }
            Perturbation::ChangeEntireGoal(new_goal) => {
                self.configurations.goal = new_goal;
                let constraint_idx = self
                    .configurations
                    .measure_table
                    .constraint_idx(&self.configurations.goal.constraint);
                let mut active_model = ActiveModel::new(
                    &self.configurations.measure_table,
                    &self.configurations.knob_table,
                );
                let _ = active_model.restrict_model(&self.configurations.knobs);
                active_model.sort_by_constraint(constraint_idx);

                self.state.controller =
                    AptoState::init_controller(&self.configurations, &active_model, constraint_idx);
            }
        }
    }

    pub fn freeze_adaptation(&mut self) {
        warn!(
            "Instance {} changed to NonAdaptive mode.",
            self.configurations.instance_id
        );
        self.state.mode = AptoMode::NonAdaptive;
    }

    pub fn unfreeze_adaptation(&mut self) {
        info!(
            "Instance {} changed to Adaptive mode.",
            self.configurations.instance_id
        );
        self.state.mode = AptoMode::Adaptive;
    }

    pub fn set_controller_gain(&mut self, val: f64) {
        self.state.controller.set_gain(val);
        warn!(
            "Updated gain of controller to {} (instance {})",
            val, self.configurations.instance_id
        );
    }

    pub fn set_error_multiplier(&mut self, val: f64) {
        self.state.controller.set_multiplier(val);
        warn!(
            "Updated multiplier of the controller to {} (instance {})",
            val, self.configurations.instance_id
        );
    }

    pub fn set_derivative_multiplier(&mut self, val: f64) {
        self.state.controller.set_derivative_multiplier(val);
        warn!(
            "Updated derivate gain of the controller to {} (instance {})",
            val, self.configurations.instance_id
        );
    }

    fn log_state(&mut self) {
        let mut log_line = String::new();
        for (name, device) in self.state.measurements.iter() {
            if let Some(value) = device.prev_value() {
                log_line.push_str(&format!("{}:{},", name, value));
            } else {
                log_line.push_str(&format!("{}:none,", name));
            }
        }

        for (name, knob) in self.configurations.knobs.iter() {
            log_line.push_str(&format!("{}:{},", name, knob.get()));
        }
        log_line.remove(log_line.len() - 1);
        info!("instance:{},{}", self.configurations.instance_id, log_line);

        match self.state.mode {
            AptoMode::Adaptive | AptoMode::NonAdaptive => self.write_to_binary_files(),
            _ => (),
        };
    }

    fn write_to_binary_files(&mut self) {
        let mut write = |name, value: f64| {
            let writer = self.outfiles.entry(name).or_insert_with(|| {
                let newfile = OpenOptions::new()
                    .create_new(true)
                    .write(true)
                    .open(format!("{}.{}", name, self.configurations.instance_id))
                    .unwrap();
                BufWriter::new(newfile)
            });
            let _ = writer.write(&value.to_ne_bytes()).unwrap();
        };

        let relevant_measures = [
            "appLatency",
            "numCores",
            "powerConsumption",
            "err0",
            "err1",
            "perf0",
            "perf1",
            "harmonicMeanPerf",
        ];
        for name in relevant_measures {
            let _ = self
                .state
                .measurements
                .get(name)
                .and_then(|m| m.prev_value())
                .map(|v| write(name, v));
        }

        let knobs = ["utilizedCoreFrequency", "uncoreFrequency", "hyperthreading"];
        for name in knobs {
            let _ = self
                .state
                .measurements
                .get(name)
                .and_then(|m| m.prev_value())
                .map(|v| write(name, v as f64));
        }
    }
}