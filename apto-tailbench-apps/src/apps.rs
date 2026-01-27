use crate::arch_utils::*;
use crate::{Average, MessageQueue};
use apto::*;
use std::cell::RefCell;
use std::process::{Child, Command};
use std::rc::Rc;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, AtomicI8, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Barrier, Mutex};
use std::thread::JoinHandle;

#[derive(Debug, Clone)]
pub struct Application {
    pub name: String,
    pub launcher: fn() -> Child,
    pub profiles: (String, String), // (MT, KT)
    pub pipe_name: String,
    pub window: u64,
}

impl Application {
    fn new(name: String) -> Application {
        let (launcher, window): (fn() -> Child, u64) = match name.as_str() {
            "dnn" => (launch_dnn, 200),
            "masstree" => (launch_masstree, 1000),
            "moses" => (launch_moses, 200),
            "silo" => (launch_silo, 1000),
            "xapian" => (launch_xapian, 200),
            "sphinx" => (launch_sphinx, 2),
            "nginx" => (launch_nginx, 1000),
            _ => panic!("incorrect application"),
        };

        let profiles = (
            format!("profiles/{}.mt", name),
            format!("profiles/{}.kt", name),
        );

        let pipe_name = format!("/{}", name);

        Application {
            name,
            launcher,
            profiles,
            pipe_name,
            window,
        }
    }

    fn make_queue(&self) -> MessageQueue {
        MessageQueue::new(self.pipe_name.as_str(), std::mem::size_of::<u64>() as i64)
            .unwrap_or_else(|_| {
                let message = std::ffi::CString::new("opening").unwrap();
                unsafe { libc::perror(message.as_ptr()) };
                panic!("Error opening queue");
            })
    }

    pub fn monolithic(&self, goal: f64, warmup: u64, exec_time: u64, inst_id: usize) {
        let mq = self.make_queue();

        let pid = Rc::new(RefCell::new(None));

        let active_cores = if inst_id % 2 == 0 {
            get_active_cores()
        } else {
            get_active_cores().into_iter().rev().collect()
        };

        let num_cores = generate_num_cores(pid.clone(), active_cores.clone(), inst_id);

        let core_freq = generate_core_freq();

        let uncore_freq = generate_uncore_freq();

        let hyperthreading =
            generate_hyperthreading(pid.clone(), num_cores.clone(), active_cores, inst_id);

        let goal = Goal::new(
            "appLatency".to_string(),
            goal,
            OptimizationType::Minimize,
            "numCores".to_string(),
        );
        let config = AptoConfig::new(
            inst_id,
            &self.profiles.0,
            &self.profiles.1,
            vec![num_cores.clone(), core_freq, uncore_freq, hyperthreading],
            goal,
            self.window,
        );
        let mut apto = Apto::new(config);

        let chld: Rc<RefCell<Option<Child>>> = Rc::new(RefCell::new(None));

        let init_chld = chld.clone();
        let init_pid = pid.clone();
        let init_mq = mq.clone();
        let stream_init = Box::new(move || {
            *init_chld.borrow_mut() = Some((self.launcher)());
            let mut pids = vec![init_chld.borrow().as_ref().map(|e| e.id()).unwrap()];
            if self.name == "nginx" {
                let nginx_pids = get_nginx_pids();
                pids = nginx_pids;
            }
            *init_pid.borrow_mut() = Some(pids);

            let mut buffer: [u8; std::mem::size_of::<u64>()] = [0; std::mem::size_of::<u64>()];
            while init_mq.read_message(&mut buffer).is_err() {}

            let start_time = std::time::Instant::now();
            while start_time.elapsed().as_secs() < warmup {
                let mut buffer: [u8; std::mem::size_of::<u64>()] = [0; std::mem::size_of::<u64>()];
                let _ = init_mq.read_message(&mut buffer);
            }
        });

        let deinit_mq = mq.clone();
        let stream_deinit = Box::new(move || {
            let mut chld_instance = chld.borrow_mut().take().unwrap();
            let _ = chld_instance.kill();
            let _ = chld_instance.wait();

            *pid.borrow_mut() = None;
            if self.name == "nginx" {
                stop_nginx();
            }

            println!(
                "Killed old {}. Cleared {} messages from queue.",
                &self.name,
                deinit_mq.clear_queue()
            );
        });

        let mut iteration = 0;
        let start_time = std::time::Instant::now(); // Need to account for wramup time in the execution time as well
        let main_loop = Box::new(|apto: &mut Apto| {
            if start_time.elapsed().as_secs() > exec_time {
                return false;
            }
            let mut buffer: [u8; std::mem::size_of::<u64>()] = [0; std::mem::size_of::<u64>()];

            iteration += 1;

            if mq.read_message(&mut buffer).is_ok() {
                apto.measure("appLatency", u64::from_ne_bytes(buffer) as f64);
                apto.measure("numCores", num_cores.get() as f64);
                true
            } else {
                println!("Exiting early because of end of input for instance 0");
                false
            }
        });

        apto.optimize(Some(stream_init), Some(stream_deinit), main_loop);
    }

    pub fn app_component(
        &self,
        goal: f64,
        warmup: u64,
        exec_time: u64,
        sender: Sender<f64>,
        inst_id: usize,
    ) -> JoinHandle<()> {
        let app = self.clone();
        std::thread::spawn(move || {
            let mq = app.make_queue();

            let pid = Rc::new(RefCell::new(None));

            let active_cores = if inst_id % 2 == 0 {
                get_active_cores()
            } else {
                get_active_cores().into_iter().rev().collect()
            };

            let num_cores = generate_num_cores(pid.clone(), active_cores.clone(), inst_id);

            let hyperthreading =
                generate_hyperthreading(pid.clone(), num_cores.clone(), active_cores, inst_id);

            let goal = Goal::new(
                "latency".to_string(),
                goal,
                OptimizationType::Minimize,
                "numCores".to_string(),
            );
            let profiles = app.app_only_profiles();

            let config = AptoConfig::new(
                inst_id,
                &profiles.0,
                &profiles.1,
                vec![num_cores.clone(), hyperthreading],
                goal,
                app.window,
            );
            let mut apto = Apto::new(config);

            let chld: Rc<RefCell<Option<Child>>> = Rc::new(RefCell::new(None));

            let is_nginx = app.name == "nginx";
            let init_chld = chld.clone();
            let init_pid = pid.clone();
            let init_mq = mq.clone();
            let stream_init = Box::new(move || {
                *init_chld.borrow_mut() = Some((app.launcher)());
                let mut pids = vec![init_chld.borrow().as_ref().map(|e| e.id()).unwrap()];
                if is_nginx {
                    let nginx_pids = get_nginx_pids();
                    pids = nginx_pids;
                }
                *init_pid.borrow_mut() = Some(pids);

                let mut buffer: [u8; std::mem::size_of::<u64>()] = [0; std::mem::size_of::<u64>()];
                while init_mq.read_message(&mut buffer).is_err() {}

                let start_time = std::time::Instant::now();
                while start_time.elapsed().as_secs() < warmup {
                    let mut buffer: [u8; std::mem::size_of::<u64>()] =
                        [0; std::mem::size_of::<u64>()];
                    let _ = init_mq.read_message(&mut buffer);
                }
            });

            let deinit_mq = mq.clone();
            let stream_deinit = Box::new(move || {
                let mut chld_instance = chld.borrow_mut().take().unwrap();
                let _ = chld_instance.kill();
                let _ = chld_instance.wait();

                *pid.borrow_mut() = None;
                if app.name == "nginx" {
                    stop_nginx();
                }

                deinit_mq.clear_queue();
                println!("Killed old xapian. Cleared Queue");
            });

            let mut iteration = 0;
            let start_time = std::time::Instant::now();
            let main_loop = Box::new(|apto: &mut Apto| {
                if start_time.elapsed().as_secs() > exec_time {
                    return false;
                }

                let mut buffer: [u8; std::mem::size_of::<u64>()] = [0; std::mem::size_of::<u64>()];

                iteration += 1;

                if mq.read_message(&mut buffer).is_ok() {
                    let latency = u64::from_ne_bytes(buffer) as f64;
                    apto.measure("latency", latency);
                    apto.measure("numCores", num_cores.get() as f64);
                    let _ = sender.send(latency);
                    true
                } else {
                    false
                }
            });

            let _ = sender.send(0.0);
            apto.optimize(Some(stream_init), Some(stream_deinit), main_loop);
        })
    }
    
    
    pub fn tenant(
        &self,
        goal: f64,
        warmup: u64,
        average: Arc<Mutex<Average>>,
        (should_continue, warmup_counter, barrier): (Arc<AtomicBool>, Arc<AtomicI8>, Arc<Barrier>),
        inst_id: usize,
    ) -> JoinHandle<()> {
        // TODO: Decide after testing if we want to add a config requester
        //       I don't think we will need it here because we're only modifying
        //       cores and hyperthreading in disjoint sets
        let app = self.clone();
        std::thread::spawn(move || {
            let mq = app.make_queue();

            let pid = Rc::new(RefCell::new(None));

            let active_cores = if inst_id % 2 == 0 {
                get_active_cores()
            } else {
                get_active_cores().into_iter().rev().collect()
            };

            let num_cores = generate_num_cores(pid.clone(), active_cores.clone(), inst_id);

            let hyperthreading =
                generate_hyperthreading(pid.clone(), num_cores.clone(), active_cores, inst_id);

            let goal = Goal::new(
                "appLatency".to_string(),
                goal,
                OptimizationType::Minimize,
                "numCores".to_string(),
            );

            let profiles = app.app_only_profiles();
            let config = AptoConfig::new(
                inst_id,
                &profiles.0,
                &profiles.1,
                vec![num_cores.clone(), hyperthreading],
                goal,
                app.window,
            );
            let mut apto = Apto::new(config);

            let chld: Rc<RefCell<Option<Child>>> = Rc::new(RefCell::new(None));

            let is_nginx = app.name == "nginx";
            let init_chld = chld.clone();
            let init_pid = pid.clone();
            let init_mq = mq.clone();
            let stream_init = Box::new(move || {
                *init_chld.borrow_mut() = Some((app.launcher)());
                let mut pids = vec![init_chld.borrow().as_ref().map(|e| e.id()).unwrap()];
                if is_nginx {
                    let nginx_pids = get_nginx_pids();
                    pids = nginx_pids;
                }
                *init_pid.borrow_mut() = Some(pids);

                let mut buffer: [u8; std::mem::size_of::<u64>()] = [0; std::mem::size_of::<u64>()];
                while init_mq.read_message(&mut buffer).is_err() {}

                let start_time = std::time::Instant::now();
                while start_time.elapsed().as_secs() < warmup {
                    let mut buffer: [u8; std::mem::size_of::<u64>()] =
                        [0; std::mem::size_of::<u64>()];
                    let _ = init_mq.read_message(&mut buffer);
                }

                warmup_counter.fetch_add(1, Ordering::Relaxed);

                while warmup_counter.load(Ordering::Relaxed) < 2 {
                    let mut buffer: [u8; std::mem::size_of::<u64>()] =
                        [0; std::mem::size_of::<u64>()];
                    let _ = init_mq.read_message(&mut buffer);
                }

                barrier.wait();
            });

            let deinit_mq = mq.clone();
            let stream_deinit = Box::new(move || {
                let mut chld_instance = chld.borrow_mut().take().unwrap();
                let _ = chld_instance.kill();
                let _ = chld_instance.wait();

                *pid.borrow_mut() = None;
                if app.name == "nginx" {
                    stop_nginx();
                }

                deinit_mq.clear_queue();
                println!("Killed old {}. Cleared Queue", app.name);
            });

            let mut iterations = 0;
            let main_loop = Box::new(|apto: &mut Apto| {
                let mut buffer: [u8; std::mem::size_of::<u64>()] = [0; std::mem::size_of::<u64>()];

                iterations += 1;
                if mq.read_message(&mut buffer).is_ok() {
                    let latency = u64::from_ne_bytes(buffer) as f64;
                    apto.measure("appLatency", latency);
                    apto.measure("numCores", num_cores.get() as f64);

                    average.lock().unwrap().update(latency);

                    should_continue.load(Ordering::Relaxed)
                } else {
                    eprintln!(
                        "====================Exiting {} after {} because of timeout====================",
                        inst_id, iterations
                    );
                    should_continue.store(false, Ordering::Relaxed);
                    false
                }
            });

            apto.optimize(Some(stream_init), Some(stream_deinit), main_loop);
        })
    }

    pub fn app_only_profiles(&self) -> (String, String) {
        (
            format!("{}-data/app_mt_filtered", self.name),
            format!("{}-data/app_kt_filtered", self.name),
        )
    }

    pub fn sys_only_profiles(&self) -> (String, String) {
        (
            format!("{}-data/sys_mt_filtered", self.name),
            format!("{}-data/sys_kt_filtered", self.name),
        )
    }
}

impl FromStr for Application {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Application::new(s.to_string()))
    }
}

pub fn launch_xapian() -> Child {
    Command::new("/home/vedant/wasl/tailbench/xapian/xapian_integrated")
        .envs(
            [
                (
                    "LD_LIBRARY_PATH",
                    "/home/vedant/wasl/tailbench/xapian/xapian-core-1.2.13/install/lib/",
                ),
                ("TBENCH_QPS", "500"),
                ("TBENCH_MAXREQS", "50000000"),
                ("TBENCH_WARMUPREQS", "0"),
                ("TBENCH_MINSLEEPNS", "100000"),
                (
                    "TBENCH_TERMS_FILE",
                    "/home/vedant/wasl/tailbench/tailbench.inputs/xapian/terms.in",
                ),
                ("QUEUE_NAME", "/xapian"),
            ]
            .into_iter(),
        )
        .args([
            "-n",
            "16",
            "-d",
            "/home/vedant/wasl/tailbench/tailbench.inputs/xapian/wiki",
            "-r",
            "1000000000",
        ])
        .stdout(std::process::Stdio::null())
        .current_dir("/home/vedant/wasl/tailbench/xapian")
        .spawn()
        .unwrap()
}

pub fn launch_masstree() -> Child {
    Command::new("/home/vedant/wasl/tailbench/masstree/mttest_integrated")
        .envs(
            [
                ("TBENCH_QPS", "5000"),
                ("TBENCH_MAXREQS", "50000000"),
                ("TBENCH_WARMUPREQS", "0"),
                ("TBENCH_MINSLEEPNS", "10000"),
                ("QUEUE_NAME", "/masstree"),
            ]
            .into_iter(),
        )
        .args(["-j", "8", "mycsba", "masstree"])
        .stdout(std::process::Stdio::null())
        .current_dir("/home/vedant/wasl/tailbench/masstree")
        .spawn()
        .unwrap()
}

pub fn launch_moses() -> Child {
    Command::new("/home/vedant/wasl/tailbench/moses/bin/moses_integrated")
        .envs([
            ("TBENCH_QPS", "550"),
            ("TBENCH_MAXREQS", "7500000"),
            ("TBENCH_WARMUPREQS", "0"),
            ("TBENCH_MINSLEEPNS", "10000"),
            ("QUEUE_NAME", "/moses"),
        ])
        .args([
            "-config",
            "./moses.ini",
            "-input-file",
            "/home/vedant/wasl/tailbench/tailbench.inputs/moses/testTerms",
            "-threads",
            "16",
            "-num-tasks",
            "100000",
            "-verbose",
            "0",
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .current_dir("/home/vedant/wasl/tailbench/moses")
        .spawn()
        .unwrap()
}

pub fn launch_dnn() -> Child {
    Command::new("/home/vedant/wasl/tailbench/img-dnn/img-dnn_integrated")
        .envs([
            ("TBENCH_WARMUPREQS", "0"),
            ("TBENCH_MAXREQS", "100000000"),
            ("TBENCH_QPS", "725"),
            ("TBENCH_MINSLEEPNS", "10000"),
            (
                "TBENCH_MNIST_DIR",
                "/home/vedant/wasl/tailbench/tailbench.inputs/img-dnn/mnist",
            ),
            ("QUEUE_NAME", "/dnn"),
        ])
        .args([
            "-r",
            "16",
            "-f",
            "/home/vedant/wasl/tailbench/tailbench.inputs/img-dnn/models/model.xml",
            "-n",
            "100000000",
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap()
}

pub fn launch_sphinx() -> Child {
    Command::new("/home/vedant/wasl/tailbench/sphinx/decoder_integrated")
        .envs([
            ("QUEUE_NAME", "/sphinx"),
            (
                "LD_LIBRARY_PATH",
                "/home/vedant/wasl/tailbench/sphinx/sphinx-install/lib",
            ),
            ("TBENCH_QPS", "1.0"),
            ("TBENCH_MAXREQS", "100000000000000000000000000000"),
            ("TBENCH_WARNUPREQS", "0"),
            ("TBENCH_MINSLEEPNS", "10000"),
            ("TBENCH_AN4_CORPUS", "/home/vedant/wasl/tailbench/tailbench.inputs/sphinx"),
            ("TBENCH_AUDIO_SAMPLES", "audio_samples"),
        ])
        .args(["-t", "16"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .current_dir("/home/vedant/wasl/tailbench/sphinx")
        .spawn()
        .unwrap()
}

pub fn launch_silo() -> Child {
    Command::new("/home/cc/wasl/wasl-tailbench/silo/out-perf.masstree/benchmarks/dbtest_integrated")
        .envs([
            ("QUEUE_NAME", "/silo"),
            ("TBENCH_QPS", "4000"),
            ("TBENCH_MAXREQS", "100000000000000000000000000000"),
            ("TBENCH_WARNUPREQS", "0"),
            ("TBENCH_MINSLEEPNS", "10000"),
        ])
        .args([
            "--verbose",
            "--bench",
            "tpcc",
            "--num-threads",
            "16",
            "--scale-factor",
            "10",
            "--retry-aborted-transactions",
            "--ops-per-worker",
            "10000000",
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .current_dir("/home/cc/wasl/wasl-tailbench/silo")
        .spawn()
        .unwrap()
}

pub fn launch_nginx() -> Child {
    stop_nginx();
    let _ = Command::new("systemctl")
        .args(["start", "nginx"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output();

    std::thread::sleep(std::time::Duration::from_secs(5));

    Command::new("/home/wrk/wrk")
        .args([
            "-t4",
            "-c400",
            "-d5h",
            "http://127.0.0.1:8090",
            "-s",
            "/home/wrk/scripts/random.lua",
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap()
}

pub fn stop_nginx() {
    let _ = Command::new("systemctl")
        .args(["stop", "nginx"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output();
}

pub fn get_nginx_pids() -> Vec<u32> {
    let command_output = Command::new("ps").args(["aux"]).output().unwrap();
    let output = String::from_utf8_lossy(&command_output.stdout);
    output
        .lines()
        .filter(|p| p.contains("nginx:"))
        .map(|line| {
            line.split_ascii_whitespace()
                .nth(1)
                .unwrap()
                .parse()
                .unwrap()
        })
        .collect()
}
