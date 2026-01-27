use apto::*;
use apto_tailbench::apps::launch_silo;
use apto_tailbench::{arch_utils, MessageQueue};
use env_logger::{Builder, Target};
use std::cell::RefCell;
use std::process::Child;
use std::rc::Rc;

fn main() {
    let tables = std::env::args().skip(1).collect::<Vec<String>>();

    let mut builder = Builder::from_default_env();
    builder.target(Target::Stdout);
    builder.init();

    let mq = MessageQueue::new("/silo", std::mem::size_of::<u64>() as i64).unwrap_or_else(|_| {
        let message = std::ffi::CString::new("opening").unwrap();
        unsafe { libc::perror(message.as_ptr()) };
        panic!("Error opening queue");
    });

    let pid = Rc::new(RefCell::new(None));

    let active_physical_cores = arch_utils::get_active_cores();

    let knob_pid = pid.clone();
    let num_cores = Rc::new(ApplicationKnob::new(
        "utilizedPhysicalCores".to_string(),
        vec![2, 4, 6, 8],
        8,
        None,
    ));
    let core_freq = Rc::new(CoreFrequency::new(vec![1200, 2000, 2800], 2800));
    let uncore_freq = Rc::new(UncoreFrequency::new(vec![12, 16, 20, 24, 28], 28));

    let knob_pid = pid.clone();
    let knob_num_cores = num_cores.clone();
    let active_physical_cores = arch_utils::get_active_cores();
    let core_pairs: Vec<(usize, usize)> = active_physical_cores
        .iter()
        .zip(
            active_physical_cores
                .iter()
                .skip(active_physical_cores.len() / 2),
        )
        .map(|(&t0, &t1)| (t0, t1))
        .collect();
    let hyperthreading = Rc::new(ApplicationKnob::new(
        "hyperthreading".to_string(),
        vec![0, 1],
        1,
        None,
    ));

    let goal = Goal::new(
        "appLatency".to_string(),
        329000.00,
        OptimizationType::Minimize,
        "numCores".to_string(),
    );
    let config = AptoConfig::new(
        0,
        &tables[0],
        &tables[1],
        vec![num_cores.clone(), core_freq, uncore_freq, hyperthreading],
        goal,
        1000,
    );
    let mut apto = Apto::new(config);

    let chld: Rc<RefCell<Option<Child>>> = Rc::new(RefCell::new(None));

    let init_chld = chld.clone();
    let init_pid = pid.clone();
    let init_mq = mq.clone();
    let stream_init = Box::new(move || {
        *init_chld.borrow_mut() = Some(launch_silo());
        *init_pid.borrow_mut() = init_chld.borrow().as_ref().map(|e| e.id());
        for _ in 0..10000 {
            let mut buffer: [u8; std::mem::size_of::<u64>()] = [0; std::mem::size_of::<u64>()];
            if init_mq.read_message(&mut buffer).is_err() {
                println!("Could not read message in stream initialization");
            }
        }
    });

    let deinit_mq = mq.clone();
    let stream_deinit = Box::new(move || {
        let mut chld_instance = chld.borrow_mut().take().unwrap();
        let _ = chld_instance.kill();
        let _ = chld_instance.wait();
        *pid.borrow_mut() = None;
        println!(
            "Killed old silo. Cleared {} messages from queue.",
            deinit_mq.clear_queue()
        );
    });

    let mut iteration = 0;
    let main_loop = Box::new(|apto: &mut Apto| {
        let mut buffer: [u8; std::mem::size_of::<u64>()] = [0; std::mem::size_of::<u64>()];

        iteration += 1;
        if iteration == 100000 {
            return false;
        }

        if mq.read_message(&mut buffer).is_ok() {
            apto.measure("appLatency", u64::from_ne_bytes(buffer) as f64);
            apto.measure("numCores", num_cores.get() as f64);
            true
        } else {
            false
        }
    });

    apto.optimize(Some(stream_init), Some(stream_deinit), main_loop);
}
