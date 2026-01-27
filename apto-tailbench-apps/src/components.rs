use crate::apps::Application;
use crate::arch_utils::*;
use apto::*;
use std::sync::mpsc::Receiver;
use std::thread::JoinHandle;

pub fn launch_sys_module(
    app: Application,
    goal: f64,
    receiver: Receiver<f64>,
    inst_id: usize,
) -> JoinHandle<()> {
    std::thread::spawn(move || {
        let core_freq = generate_core_freq();
        let uncore = generate_uncore_freq();

        let goal = Goal::new(
            "latency".to_string(),
            goal,
            OptimizationType::Minimize,
            "powerConsumption".to_string(),
        );

        let profile = app.sys_only_profiles();
        let config = AptoConfig::new(
            inst_id,
            &profile.0,
            &profile.1,
            vec![core_freq, uncore],
            goal,
            app.window,
        );
        let mut apto = Apto::new(config);

        let mut iteration = 0;
        let body = |apto: &mut Apto| {
            iteration += 1;

            if let Ok(value) = receiver.recv() {
                apto.measure("latency", value);
                true
            } else {
                false
            }
        };

        apto.optimize(None, None, Box::new(body));
    })
}

pub fn launch_sys_scheduler() -> JoinHandle<()> {
    todo!()
}