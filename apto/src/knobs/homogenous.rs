use super::utilities::*;
use super::BorrowedValues;
use super::GenericKnob;
use super::Tunable;
use std::cell::RefCell;

pub struct AvailablePhysicalThreads {
    knob: RefCell<GenericKnob<u64>>,
    valid_cores: Vec<usize>,
}

impl AvailablePhysicalThreads {
    pub fn new(values: Vec<u64>, initial_value: u64) -> AvailablePhysicalThreads {
        let nr_cores = unsafe { libc::sysconf(libc::_SC_NPROCESSORS_CONF) } as usize;
        let valid_cores = (0..nr_cores).filter(|id| core_online(*id)).collect();

        let knob = AvailablePhysicalThreads {
            knob: RefCell::new(GenericKnob::new(
                "utilizedPhysicalThreads".to_string(),
                values,
                initial_value,
            )),
            valid_cores,
        };
        knob.set_affinity();
        knob
    }

    fn apply(&self, val: u64) {
        if self.knob.borrow().current_value == val {
            return;
        }
        self.knob.borrow_mut().current_value = val;
        self.set_affinity();
    }

    fn set_affinity(&self) {
        // Make target CPU mask
        let cpu_mask: libc::cpu_set_t = unsafe {
            let mut cpu_mask = std::mem::zeroed();
            for i in self.valid_cores.iter().take(self.get() as usize).copied() {
                libc::CPU_SET(i, &mut cpu_mask);
            }
            cpu_mask
        };

        set_thread_affinity(cpu_mask);
    }
}

impl Tunable<u64> for AvailablePhysicalThreads {
    fn get(&self) -> u64 {
        self.knob.borrow().current_value
    }

    fn set(&self, val: u64) {
        self.apply(val);
    }

    fn name(&self) -> String {
        self.knob.borrow().name.clone()
    }

    fn possible_values(&self) -> BorrowedValues<'_, u64> {
        BorrowedValues {
            borrowed_knob: self.knob.borrow(),
        }
    }
}

pub struct AvailablePhysicalCores {
    knob: RefCell<GenericKnob<u64>>,
    valid_cores: Vec<usize>,
}

impl AvailablePhysicalCores {
    pub fn new(values: Vec<u64>, initial_value: u64) -> AvailablePhysicalCores {
        let nr_cores = get_num_physical_cpus();
        let valid_cores = (0..nr_cores).filter(|id| core_online(*id)).collect();

        let knob = AvailablePhysicalCores {
            knob: RefCell::new(GenericKnob::new(
                "utilizedPhysicalCores".to_string(),
                values,
                initial_value,
            )),
            valid_cores,
        };
        knob.set_affinity();
        knob
    }

    fn apply(&self, val: u64) {
        if self.knob.borrow().current_value == val {
            return;
        }
        self.knob.borrow_mut().current_value = val;
        self.set_affinity();
    }

    fn set_affinity(&self) {
        // Make target CPU mask
        let cpu_mask: libc::cpu_set_t = unsafe {
            let mut cpu_mask = std::mem::zeroed();
            for core_id in self.valid_cores.iter().take(self.get() as usize) {
                // Read siblings and add them to the mask
                let filename = format!(
                    "/sys/devices/system/cpu/cpu{}/topology/thread_siblings_list",
                    core_id
                );
                let core_threads: Vec<usize> = std::fs::read_to_string(filename)
                    .unwrap_or_else(|e| {
                        panic!("Could not read siblings list for {}: {:?}", core_id, e)
                    })
                    .trim()
                    .split(',')
                    .map(|e| e.parse::<usize>().unwrap())
                    .collect();
                for id in core_threads {
                    libc::CPU_SET(id, &mut cpu_mask);
                }
            }
            cpu_mask
        };

        set_thread_affinity(cpu_mask);
    }
}

impl Tunable<u64> for AvailablePhysicalCores {
    fn get(&self) -> u64 {
        self.knob.borrow().current_value
    }

    fn set(&self, val: u64) {
        self.apply(val);
    }

    fn name(&self) -> String {
        self.knob.borrow().name.clone()
    }

    fn possible_values(&self) -> BorrowedValues<'_, u64> {
        BorrowedValues {
            borrowed_knob: self.knob.borrow(),
        }
    }
}

pub struct CoreFrequency {
    knob: RefCell<GenericKnob<u64>>,
    online_cpus: Vec<usize>,
}

impl CoreFrequency {
    pub fn new(values: Vec<u64>, initial_value: u64) -> CoreFrequency {
        let nr_cpus = unsafe { libc::sysconf(libc::_SC_NPROCESSORS_CONF) } as usize;
        let online_cpus = (0..nr_cpus).filter(|c| core_online(*c)).collect();

        let knob = CoreFrequency {
            knob: RefCell::new(GenericKnob::new(
                "utilizedCoreFrequency".to_string(),
                values,
                initial_value,
            )),
            online_cpus,
        };
        knob.apply_frequency();
        knob
    }

    fn apply(&self, val: u64) {
        if self.knob.borrow().current_value == val {
            return;
        }
        self.knob.borrow_mut().current_value = val;
        self.apply_frequency();
    }

    fn apply_frequency(&self) {
        set_frequency(self.get() * 1000, self.online_cpus.iter().copied());
    }
}

impl Tunable<u64> for CoreFrequency {
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

pub struct UncoreFrequency {
    knob: RefCell<GenericKnob<u64>>,
    socket_cores: Vec<usize>,
}

impl UncoreFrequency {
    pub fn new(values: Vec<u64>, initial_value: u64) -> UncoreFrequency {
        let active_socket_cores = get_package_default_cores()
            .into_iter()
            .filter(|&i| core_online(i))
            .collect();

        let knob = UncoreFrequency {
            knob: RefCell::new(GenericKnob::new(
                "uncoreFrequency".to_string(),
                values,
                initial_value,
            )),
            socket_cores: active_socket_cores,
        };
        knob.apply_uncore_frequency();
        knob
    }

    fn apply_uncore_frequency(&self) {
        let target_freq = self.knob.borrow().current_value;
        let register_value = ((target_freq << 8) + target_freq).to_ne_bytes();
        for core_num in self.socket_cores.iter() {
            write_msr(*core_num as i64, 0x620, &register_value);
        }
    }

    fn apply(&self, val: u64) {
        if self.knob.borrow().current_value == val {
            return;
        }
        self.knob.borrow_mut().current_value = val;
        self.apply_uncore_frequency();
    }
}

impl Tunable<u64> for UncoreFrequency {
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

pub struct Hyperthreading {
    knob: RefCell<GenericKnob<u64>>,
    nr_physical_cpus: usize,
    first_ht_id: usize,
}

impl Hyperthreading {
    pub fn new(values: Vec<u64>, initial_value: u64) -> Hyperthreading {
        let knob = Hyperthreading {
            knob: RefCell::new(GenericKnob::new(
                "hyperthreading".to_string(),
                values,
                initial_value,
            )),
            nr_physical_cpus: get_num_physical_cpus(),
            first_ht_id: (unsafe { libc::sysconf(libc::_SC_NPROCESSORS_CONF) } as usize) / 2,
        };
        knob.apply_hyperthreading_mask();
        knob
    }

    fn apply_hyperthreading_mask(&self) {
        let mask_adjustment_function = if self.get() == 0 {
            |mask: &mut libc::cpu_set_t, cpu: usize| unsafe { libc::CPU_CLR(cpu, mask) }
        } else {
            |mask: &mut libc::cpu_set_t, cpu: usize| unsafe { libc::CPU_SET(cpu, mask) }
        };

        let current_mask = get_affinity();
        let mut new_mask = current_mask;
        for cpuid in 0..self.nr_physical_cpus {
            if unsafe { libc::CPU_ISSET(cpuid, &current_mask) } {
                mask_adjustment_function(&mut new_mask, cpuid + self.first_ht_id);
            }
        }

        set_thread_affinity(new_mask);
    }

    fn apply(&self, val: u64) {
        self.knob.borrow_mut().current_value = val;
        self.apply_hyperthreading_mask()
    }
}

impl Tunable<u64> for Hyperthreading {
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
    use std::time::Duration;

    #[test]
    #[ignore]
    fn test_available_threads() {
        let online_cores = get_active_physical_threads();
        let knob = AvailablePhysicalThreads::new(vec![1, 2, 3, 4], 3);
        assert_eq!(
            get_affinity(),
            make_mask(online_cores.iter().take(3).copied())
        );
        knob.set(2);
        assert_eq!(
            get_affinity(),
            make_mask(online_cores.iter().take(2).copied())
        );
        knob.set(4);
        assert_eq!(
            get_affinity(),
            make_mask(online_cores.iter().take(4).copied())
        );
    }

    #[test]
    #[ignore]
    fn test_core_frequency() {
        let get_freq = |id| {
            let fname = format!("/sys/devices/system/cpu/cpu{}/cpufreq/scaling_max_freq", id);
            std::fs::read_to_string(fname)
                .unwrap_or_else(|e| panic!("Could not fetch core frequency: {:?}", e))
                .trim()
                .parse::<u64>()
                .unwrap()
        };

        let cores = get_active_physical_threads();

        let knob = CoreFrequency::new(vec![1200, 1500, 1600], 1200);
        std::thread::sleep(Duration::from_millis(10));
        for i in cores.iter() {
            assert_eq!(get_freq(i), 1200 * 1000);
        }

        knob.set(1500);
        std::thread::sleep(Duration::from_millis(10));
        for i in cores.iter() {
            assert_eq!(get_freq(i), 1500 * 1000);
        }

        knob.set(1600);
        std::thread::sleep(Duration::from_millis(10));
        for i in cores.iter() {
            assert_eq!(get_freq(i), 1600 * 1000);
        }

        knob.set(4600);
    }

    fn make_mask<T>(tids: T) -> libc::cpu_set_t
    where
        T: IntoIterator<Item = usize>,
    {
        unsafe {
            let mut mask: libc::cpu_set_t = std::mem::zeroed();
            libc::CPU_ZERO(&mut mask);
            for id in tids {
                libc::CPU_SET(id, &mut mask);
            }
            mask
        }
    }

    #[test]
    #[ignore]
    fn test_available_cores() {
        let online_cores = get_active_physical_threads();
        let (physical, hyper) = online_cores.split_at(online_cores.len() / 2);

        let knob = AvailablePhysicalCores::new(vec![1, 2, 3], 1);
        assert_eq!(
            get_affinity(),
            make_mask(physical.iter().take(1).chain(hyper.iter().take(1)).copied())
        );

        knob.set(2);
        assert_eq!(
            get_affinity(),
            make_mask(physical.iter().take(2).chain(hyper.iter().take(2)).copied())
        );

        knob.set(3);
        assert_eq!(
            get_affinity(),
            make_mask(physical.iter().take(3).chain(hyper.iter().take(3)).copied())
        );
    }

    #[test]
    #[ignore]
    fn test_uncore_frequency() {
        let physical_threads: Vec<i64> = get_active_physical_threads()
            .into_iter()
            .map(|x| x as i64)
            .collect();
        let knob = UncoreFrequency::new(vec![16, 20, 24], 24);
        let compute_register_value = |val| (val << 8) + val;

        for id in physical_threads.iter().copied() {
            assert_eq!(compute_register_value(knob.get()), read_msr(id, 0x620));
        }

        knob.set(16);
        for id in physical_threads.iter().copied() {
            assert_eq!(compute_register_value(knob.get()), read_msr(id, 0x620));
        }

        knob.set(20);
        for id in physical_threads.iter().copied() {
            assert_eq!(compute_register_value(knob.get()), read_msr(id, 0x620));
        }

        knob.set(24);
        for id in physical_threads.iter().copied() {
            assert_eq!(compute_register_value(knob.get()), read_msr(id, 0x620));
        }
    }

    #[test]
    #[ignore]
    fn test_hyperthreading() {
        let online_cores = get_active_physical_threads();
        let (physical, _) = online_cores.split_at(online_cores.len() / 2);
        let last_physical_core =
            (unsafe { libc::sysconf(libc::_SC_NPROCESSORS_CONF) } as usize) / 2;

        let nr_cores = AvailablePhysicalCores::new(vec![1, 3, 6], 6);
        nr_cores.set(6);

        assert_eq!(
            get_affinity(),
            make_mask(
                physical
                    .iter()
                    .take(6)
                    .copied()
                    .chain(physical.iter().take(6).map(|p| p + last_physical_core))
            )
        );

        let ht = Hyperthreading::new(vec![0, 1], 1);
        assert_eq!(
            get_affinity(),
            make_mask(
                physical
                    .iter()
                    .take(6)
                    .copied()
                    .chain(physical.iter().take(6).map(|p| p + last_physical_core))
            )
        );

        ht.set(0);
        assert_eq!(get_affinity(), make_mask(physical.iter().take(6).copied()));

        nr_cores.set(1);
        ht.set(0);
        assert_eq!(get_affinity(), make_mask(physical.iter().take(1).copied()));

        ht.set(1);
        assert_eq!(
            get_affinity(),
            make_mask(
                physical
                    .iter()
                    .take(1)
                    .copied()
                    .chain(physical.iter().take(1).map(|p| p + last_physical_core))
            )
        );

        nr_cores.set(3);
        ht.set(1);
        assert_eq!(
            get_affinity(),
            make_mask(
                physical
                    .iter()
                    .take(3)
                    .copied()
                    .chain(physical.iter().take(3).map(|p| p + last_physical_core))
            )
        );
        ht.set(0);
        assert_eq!(get_affinity(), make_mask(physical.iter().take(3).copied()));

        nr_cores.set(6);
        assert_eq!(
            get_affinity(),
            make_mask(
                physical
                    .iter()
                    .take(6)
                    .copied()
                    .chain(physical.iter().take(6).map(|p| p + last_physical_core))
            )
        );
        ht.set(0);
        assert_eq!(get_affinity(), make_mask(physical.iter().take(6).copied()));
    }

    fn get_active_physical_threads() -> Vec<usize> {
        let nr_cpus = unsafe { libc::sysconf(libc::_SC_NPROCESSORS_CONF) } as usize;
        let mut threads = std::collections::HashSet::new();
        for id in 0..nr_cpus {
            if !core_online(id) {
                continue;
            }
            let filename = format!(
                "/sys/devices/system/cpu/cpu{}/topology/thread_siblings_list",
                id
            );
            threads.extend(
                std::fs::read_to_string(filename)
                    .unwrap()
                    .trim()
                    .split(',')
                    .map(|c| c.parse::<usize>().unwrap()),
            )
        }
        let mut threads: Vec<usize> = threads.into_iter().collect();
        threads.sort();
        threads
    }
}
