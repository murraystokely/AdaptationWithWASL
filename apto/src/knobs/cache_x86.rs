use super::utilities::*;
use super::{BorrowedValues, GenericKnob, Tunable};
use std::cell::RefCell;

pub struct CacheCOS {
    knob: RefCell<GenericKnob<u64>>,
}

impl CacheCOS {
    pub fn new(values: Vec<u64>, initial_value: u64) -> CacheCOS {
        let knob = CacheCOS {
            knob: RefCell::new(GenericKnob::new(
                "cacheCOS".to_string(),
                values,
                initial_value,
            )),
        };
        knob.set_cos();
        knob
    }

    fn set_cos(&self) {
        let current_mask = get_affinity();
        let num_cpus = unsafe { libc::sysconf(libc::_SC_NPROCESSORS_CONF) } as usize;
        let value = (self.get() << 32).to_ne_bytes();
        for cpuid in 0..num_cpus {
            if unsafe { libc::CPU_ISSET(cpuid, &current_mask) } {
                write_msr(cpuid as i64, 0xc8f, &value);
            }
        }
    }

    fn apply(&self, val: u64) {
        if self.knob.borrow().current_value == val {
            return;
        }
        self.knob.borrow_mut().current_value = val;
        self.set_cos();
    }
}

impl Tunable<u64> for CacheCOS {
    fn get(&self) -> u64 {
        self.knob.borrow().current_value
    }

    fn set(&self, val: u64) {
        self.apply(val);
    }

    fn name(&self) -> String {
        self.knob.borrow().name.to_string()
    }

    fn possible_values(&self) -> BorrowedValues<'_, u64> {
        BorrowedValues {
            borrowed_knob: self.knob.borrow(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AvailablePhysicalThreads;

    #[test]
    #[ignore]
    fn test_cache_cos() {
        let nr_cpus = unsafe { libc::sysconf(libc::_SC_NPROCESSORS_CONF) } as usize;
        let current_mask = get_affinity();
        let knob = CacheCOS::new(vec![0, 1, 2], 0);
        for id in 0..nr_cpus {
            if unsafe { libc::CPU_ISSET(id, &current_mask) } {
                assert_eq!(0x000000000, read_msr(id as i64, 0xc8f));
            }
        }

        knob.set(2);
        for id in 0..nr_cpus {
            if unsafe { libc::CPU_ISSET(id, &current_mask) } {
                assert_eq!(0x200000000, read_msr(id as i64, 0xc8f));
            }
        }

        knob.set(1);
        for id in 0..nr_cpus {
            if unsafe { libc::CPU_ISSET(id, &current_mask) } {
                assert_eq!(0x100000000, read_msr(id as i64, 0xc8f));
            }
        }

        knob.set(0);
        for id in 0..nr_cpus {
            if unsafe { libc::CPU_ISSET(id, &current_mask) } {
                assert_eq!(0x000000000, read_msr(id as i64, 0xc8f));
            }
        }

        let _ = AvailablePhysicalThreads::new(vec![3], 3);
        knob.set(3);
        let current_mask = get_affinity();
        for id in 0..nr_cpus {
            if unsafe { libc::CPU_ISSET(id, &current_mask) } {
                assert_eq!(0x300000000, read_msr(id as i64, 0xc8f));
            } else if core_online(id) {
                assert_eq!(0x000000000, read_msr(id as i64, 0xc8f));
            }
        }
    }
}
