use std::collections::HashMap;
use std::sync::mpsc::{channel, Sender};
use std::thread::JoinHandle;

use crate::arch_utils;

pub enum ConfigRequest {
    ChangeAffinity(u32, libc::cpu_set_t),
    ChangeCOS(u32, u64),
    Terminate,
}

pub fn init_configurer() -> (JoinHandle<()>, Sender<ConfigRequest>) {
    let (sender, receiver) = channel();

    let mut pid_cos: HashMap<u32, u64> = HashMap::new();
    let mut pid_affinity = HashMap::new();

    let handle = std::thread::spawn(move || {
        while let Ok(req) = receiver.recv() {
            match req {
                ConfigRequest::ChangeAffinity(pid, mask) => {
                    arch_utils::set_thread_affinity(pid, mask);
                    if pid_cos.contains_key(&pid) {
                        arch_utils::apply_cos(mask, pid_cos[&pid]);
                    }
                    pid_affinity.insert(pid, mask);
                }
                ConfigRequest::ChangeCOS(pid, cos) => {
                    if pid_affinity.contains_key(&pid) {
                        arch_utils::apply_cos(pid_affinity[&pid], cos);
                    }
                    pid_cos.insert(pid, cos);
                }
                ConfigRequest::Terminate => {
                    break;
                }
            }
        }
    });
    (handle, sender)
}
