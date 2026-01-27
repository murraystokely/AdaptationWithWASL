use apto::*;
use apto_tailbench::apps::Application;
use apto_tailbench::arch_utils::{generate_core_freq, generate_uncore_freq};
use apto_tailbench::components;
use apto_tailbench::Average;
use clap::{Parser, Subcommand};
use env_logger::{Builder, Target};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicI8, Ordering};
use std::sync::{mpsc, Arc, Barrier, Mutex, MutexGuard};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about=None)]
struct Options {
    #[clap(subcommand)]
    command: Variants,
    #[clap(default_value_t = 15, long = "warmup-time")]
    warmup_time: u64,
    #[clap(default_value_t = 120, long = "exec-time")]
    exec_time: u64,
}

#[derive(Subcommand, Debug)]
enum Variants {
    Monolithic {
        app: Application,
        goal: f64,
    },
    Multimodule {
        app: Application,
        goal: f64,
    },
    MultiApp {
        app0: Application,
        goal0: f64,
        app1: Application,
        goal1: f64,
    },
    NonAdaptiveMulti {
        app0: Application,
        app1: Application,
    },
    Profile {
        app: Application,
    },
}

fn main() {
    let options = Options::parse();

    let mut builder = Builder::from_default_env();
    builder.target(Target::Stdout);
    builder.init();

    match options.command {
        Variants::Monolithic { app, goal } => {
            run_monolithic(app, goal, options.warmup_time, options.exec_time)
        }
        Variants::Multimodule { app, goal } => {
            run_multimodule(app, goal, options.warmup_time, options.exec_time)
        }
        Variants::MultiApp {
            app0,
            goal0,
            app1,
            goal1,
        } => run_multi_application(
            app0,
            goal0,
            app1,
            goal1,
            options.warmup_time,
            options.exec_time,
        ),
        Variants::NonAdaptiveMulti { app0, app1 } => {
            run_non_adaptive(app0, app1, options.warmup_time, options.exec_time)
        }
        Variants::Profile { app } => run_monolithic(app, 0.0, options.warmup_time, options.exec_time),
    }
}

fn run_monolithic(app: Application, goal: f64, warmup: u64, exec_time: u64) {
    app.monolithic(goal, warmup, exec_time, 0);
}

fn run_multimodule(app: Application, goal: f64, warmup: u64, exec_time: u64) {
    let mut handles = vec![];

    let (sender, receiver) = mpsc::channel();

    handles.push(components::launch_sys_module(
        app.clone(),
        goal,
        receiver,
        1,
    ));
    handles.push(app.app_component(goal, warmup, exec_time, sender, 0));

    for h in handles {
        let _ = h.join();
    }
}

fn run_multi_application(
    app0: Application,
    goal0: f64,
    app1: Application,
    goal1: f64,
    warmup: u64,
    exec_time: u64,
) {
    let sys_constraint = 1.0;
    let sys_window = 5;

    let scheduler_profile = (
        format!(
            "profiles/multi/multi-{}-{}-generic.mt",
            app0.name, app1.name
        ),
        format!(
            "profiles/multi/multi-{}-{}-generic.kt",
            app0.name, app1.name
        ),
    );

    let core_freq = generate_core_freq();
    let uncore_freq = generate_uncore_freq();

    let goal = Goal::new(
        "harmonicMeanPerf".to_string(),
        sys_constraint,
        OptimizationType::Minimize,
        "powerConsumption".to_string(),
    );
    let config = AptoConfig::new(
        2,
        &scheduler_profile.0,
        &scheduler_profile.1,
        vec![uncore_freq, core_freq],
        goal,
        sys_window,
    );
    let mut apto = Apto::new(config);

    let (first_average, second_average) = (
        Arc::new(Mutex::new(Average::new())),
        Arc::new(Mutex::new(Average::new())),
    );
    let should_continue = Arc::new(AtomicBool::new(true));
    let barrier = Arc::new(Barrier::new(3));

    let handles = Rc::new(RefCell::new(Vec::new()));

    let (app_should_continue, app_barrier) = (should_continue.clone(), barrier.clone());
    let init_first_average = first_average.clone();
    let init_second_average = second_average.clone();
    let init_handles = handles.clone();
    let stream_init = Box::new(move || {
        let warmup_counter = Arc::new(AtomicI8::new(0));
        init_handles.borrow_mut().push(app0.tenant(
            goal0,
            warmup,
            init_first_average.clone(),
            (
                app_should_continue.clone(),
                warmup_counter.clone(),
                app_barrier.clone(),
            ),
            0,
        ));
        init_handles.borrow_mut().push(app1.tenant(
            goal1,
            warmup,
            init_second_average.clone(),
            (
                app_should_continue.clone(),
                warmup_counter,
                app_barrier.clone(),
            ),
            1,
        ));

        let _ = barrier.wait();

        init_first_average.lock().unwrap().reset();
        init_second_average.lock().unwrap().reset();

        println!("Starting main loop");
    });

    let deinit_should_continue = should_continue.clone();
    let stream_deinit = Box::new(move || {
        deinit_should_continue.store(false, Ordering::Relaxed);
        while let Some(handle) = handles.borrow_mut().pop() {
            let _ = handle.join();
        }
    });

    let mut iteration = 0;
    let start_time = std::time::Instant::now();
    let sleep_time = std::time::Duration::from_millis(50);
    let body = |apto: &mut Apto| {
        std::thread::sleep(sleep_time);

        iteration += 1;
        if start_time.elapsed().as_secs() > exec_time || !should_continue.load(Ordering::Relaxed) {
            return false;
        }

        let (e0, p0) = first_average
            .lock()
            .map(|inner| compute_perf_and_err(inner, iteration, sys_window, goal0))
            .unwrap();

        apto.measure("err0", e0);
        apto.measure("perf0", p0);

        let (e1, p1) = second_average
            .lock()
            .map(|inner| compute_perf_and_err(inner, iteration, sys_window, goal1))
            .unwrap();

        apto.measure("err1", e1);
        apto.measure("perf1", p1);

        let hm_perf = 2.0 / ((1.0 / p0) + (1.0 / p1));
        apto.measure("harmonicMeanPerf", hm_perf); // Controller should only see the last values

        let hm_err = 2.0 / ((1.0 / e0.abs()) + (1.0 / e1.abs()));
        apto.measure("harmonicMeanError", hm_err);

        let gm_perf = ((p0.ln() + p1.ln()) / 2.0).exp();
        apto.measure("geometricMeanPerf", gm_perf);

        let gm_err = ((e0.abs().ln() + e1.abs().ln()) / 2.0).exp();
        apto.measure("geometricMeanError", gm_err); // Controller should only see the last values

        true
    };

    apto.optimize(Some(stream_init), Some(stream_deinit), Box::new(body));
}

fn compute_perf_and_err(
    mut average: MutexGuard<Average>,
    iteration: u64,
    window_size: u64,
    goal: f64,
) -> (f64, f64) {
    let avg = average.average;
    if iteration % window_size == 0 {
        average.reset();
    }
    let err = (goal - avg) / goal;
    let perf_ratio = goal / avg;
    (err, perf_ratio)
}

fn run_non_adaptive(app0: Application, app1: Application, warmup: u64, exec_time: u64) {
    let mut handles = vec![];
    handles.push(std::thread::spawn(move || {
        app0.monolithic(0.0, warmup, exec_time, 0)
    }));
    handles.push(std::thread::spawn(move || {
        app1.monolithic(0.0, warmup, exec_time, 2)
    }));

    for handle in handles {
        let _ = handle.join();
    }
}