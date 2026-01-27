use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Seek, SeekFrom, Write};
use std::ops::Range;
use std::rc::Rc;

use apto::{ApplicationKnob, CoreFrequency, Tunable, UncoreFrequency};

pub fn set_thread_affinity(pid: u32, cpu_mask: libc::cpu_set_t) {
    let dirname = format!("/proc/{}/task", pid);
    let children = std::fs::read_dir(dirname)
        .unwrap_or_else(|e| panic!("Could not read children tasks for process: {:?}", e));
    for chld in children {
        let chld_pid: i32 = chld
            .unwrap_or_else(|e| panic!("Could not read directory entry: {:?}", e))
            .file_name()
            .as_os_str()
            .to_str()
            .unwrap()
            .parse()
            .unwrap_or_else(|e| panic!("Could not prase chld pid to i32: {:?}", e));

        unsafe {
            let mut current_mask: libc::cpu_set_t = std::mem::zeroed::<libc::cpu_set_t>();
            libc::sched_getaffinity(
                chld_pid,
                std::mem::size_of::<libc::cpu_set_t>(),
                &mut current_mask,
            );
            if current_mask == cpu_mask {
                continue;
            }
            libc::sched_setaffinity(chld_pid, std::mem::size_of::<libc::cpu_set_t>(), &cpu_mask)
        };
    }
}

pub fn set_frequency(target_freq: u64, core_range: Range<usize>) {
    // Convert from MHz to Hz
    let target_freq = target_freq.to_string();
    for core_num in core_range {
        let filename = format!(
            "/sys/devices/system/cpu/cpu{}/cpufreq/scaling_max_freq",
            core_num
        );
        let mut file = OpenOptions::new()
            .write(true)
            .open(filename)
            .unwrap_or_else(|e| panic!("Could not open frequency file for {}: {:?}", core_num, e));
        file.write_all(target_freq.as_bytes()).unwrap_or_else(|e| {
            panic!("Could not write to frequency file of {}: {:?}", core_num, e)
        });
    }
}

pub fn get_num_physical_cpus() -> usize {
    let file = OpenOptions::new().read(true).open("/proc/cpuinfo").unwrap();
    let reader = BufReader::new(file);
    let mut map = HashMap::new();
    let mut physid: u32 = 0;
    let mut cores: usize = 0;
    let mut chgcount = 0;
    for line in reader.lines().filter_map(|result| result.ok()) {
        let mut it = line.split(':');
        let (key, value) = match (it.next(), it.next()) {
            (Some(key), Some(value)) => (key.trim(), value.trim()),
            _ => continue,
        };
        if key == "physical id" {
            match value.parse() {
                Ok(val) => physid = val,
                Err(_) => break,
            };
            chgcount += 1;
        }
        if key == "cpu cores" {
            match value.parse() {
                Ok(val) => cores = val,
                Err(_) => break,
            };
            chgcount += 1;
        }
        if chgcount == 2 {
            map.insert(physid, cores);
            chgcount = 0;
        }
    }
    let count = map.into_iter().fold(0, |acc, (_, cores)| acc + cores);

    if count == 0 {
        1
    } else {
        count
    }
}

pub fn get_active_cores() -> Vec<usize> {
    let nr_cores = unsafe { libc::sysconf(libc::_SC_NPROCESSORS_CONF) } as usize;
    let mut active_cores = vec![0];
    for id in 1..nr_cores {
        let online =
            std::fs::read_to_string(format!("/sys/devices/system/cpu/cpu{}/online", id)).unwrap();
        if online.trim() == "1" {
            active_cores.push(id);
        }
    }
    active_cores
}

pub fn write_msr(processor: i64, reg: u64, value: &[u8]) {
    let filename = format!("/dev/cpu/{}/msr", processor);
    let mut file = OpenOptions::new()
        .write(true)
        .open(filename)
        .unwrap_or_else(|e| panic!("Could not open MSR file for {}: {:?}", processor, e));
    let _ = file.seek(SeekFrom::Start(reg));
    let _ = file.write_all(value).unwrap();
}

pub fn make_mask<'a, I>(nr_cores: u64, valid_cores: I) -> libc::cpu_set_t
where
    I: Iterator<Item = &'a usize>,
{
    let mut cpu_mask = unsafe { std::mem::zeroed::<libc::cpu_set_t>() };
    for core_id in valid_cores.take(nr_cores as usize) {
        let fname = format!(
            "/sys/devices/system/cpu/cpu{}/topology/thread_siblings_list",
            core_id
        );
        let core_threads: Vec<usize> = std::fs::read_to_string(fname)
            .unwrap_or_else(|e| panic!("Could not read siblings list for {}: {:?}", core_id, e))
            .trim()
            .split(',')
            .map(|e| e.parse::<usize>().unwrap())
            .collect();
        for id in core_threads {
            unsafe { libc::CPU_SET(id, &mut cpu_mask) };
        }
    }

    cpu_mask
}

#[allow(dead_code)]
pub fn apply_affinity<'a, T>(_prev: Option<u64>, _new: u64, _pids: &Option<Vec<u32>>, _cores: T)
where
    T: Iterator<Item = &'a usize>,
{
    // if pid.is_none() {
    //     return;
    // }

    // // TODO: Test and replace the following cpu_mask computation with make_mask
    // let cpu_mask: libc::cpu_set_t = unsafe {
    //     let mut cpu_mask = std::mem::zeroed();
    //     for core_id in cores.take(new as usize) {
    //         // Read siblings and add them to the mask
    //         let filename = format!(
    //             "/sys/devices/system/cpu/cpu{}/topology/thread_siblings_list",
    //             core_id
    //         );
    //         let core_threads: Vec<usize> = std::fs::read_to_string(filename)
    //             .unwrap_or_else(|e| panic!("Could not read siblings list for {}: {:?}", core_id, e))
    //             .trim()
    //             .split(',')
    //             .map(|e| e.parse::<usize>().unwrap())
    //             .collect();
    //         for id in core_threads {
    //             libc::CPU_SET(id, &mut cpu_mask);
    //         }
    //     }
    //     cpu_mask
    // };

    // set_thread_affinity(pid.unwrap(), cpu_mask);
}

pub fn apply_cos(mask: libc::cpu_set_t, cos: u64) {
    let num_cpus = unsafe { libc::sysconf(libc::_SC_NPROCESSORS_CONF) } as usize;
    let value = (cos << 32).to_ne_bytes();
    for cpuid in 0..num_cpus {
        if unsafe { libc::CPU_ISSET(cpuid, &mask) } {
            write_msr(cpuid as i64, 0xc8f, &value);
        }
    }
}

pub fn toggle_hyperthreading<'a, I>(pids: &Option<Vec<u32>>, num_cores: u64, ht: u64, core_pairs: I)
where
    I: Iterator<Item = &'a (usize, usize)>,
{
    if pids.is_none() {
        return;
    }

    let mut mask = unsafe { std::mem::zeroed::<libc::cpu_set_t>() };

    let mut threads = Vec::new();
    for &(mut t0, mut t1) in core_pairs.take(num_cores as usize) {
        if t1 < t0 {
            // t0 should always be the smallest sibling
            std::mem::swap(&mut t0, &mut t1)
        }

        unsafe { libc::CPU_SET(t0, &mut mask) };
        threads.push(t0);
        if ht == 1 {
            unsafe { libc::CPU_SET(t1, &mut mask) };
            threads.push(t1);
        }
    }

    for pid in pids.as_ref().unwrap() {
        set_thread_affinity(*pid, mask);
    }
}

pub fn generate_num_cores(
    pids: Rc<RefCell<Option<Vec<u32>>>>,
    cores: Vec<usize>,
    id: usize,
) -> Rc<ApplicationKnob<u64>> {
    let allowed_values = match std::env::var(format!("CORES_{}", id)) {
        Ok(values) => values.split(',').map(|i| i.parse().unwrap()).collect(),
        Err(_) => vec![2, 4, 6, 8],
    };
    let init_value = match std::env::var(format!("INIT_CORES_{}", id)) {
        Ok(value) => value.parse().unwrap(),
        Err(_) => 8,
    };

    Rc::new(ApplicationKnob::new(
        "utilizedPhysicalCores".to_string(),
        allowed_values,
        init_value,
        Some(Box::new(move |prev, new| {
            apply_affinity(prev, new, &pids.borrow(), cores.iter())
        })),
    ))
}

pub fn generate_core_freq() -> Rc<CoreFrequency> {
    let allowed_values = match std::env::var("FREQS") {
        Ok(values) => values.split(',').map(|i| i.parse().unwrap()).collect(),
        Err(_) => vec![1200, 2000, 2800],
    };
    let init_value = match std::env::var("INIT_FREQ") {
        Ok(value) => value.parse().unwrap(),
        Err(_) => 2800,
    };

    Rc::new(CoreFrequency::new(allowed_values, init_value))
}

pub fn generate_uncore_freq() -> Rc<UncoreFrequency> {
    let allowed_values = match std::env::var("UNCORE") {
        Ok(values) => values.split(',').map(|i| i.parse().unwrap()).collect(),
        Err(_) => vec![12, 16, 20, 24, 28],
    };
    let init_value = match std::env::var("INIT_UNCORE") {
        Ok(value) => value.parse().unwrap(),
        Err(_) => 28,
    };

    Rc::new(UncoreFrequency::new(allowed_values, init_value))
}

pub fn generate_hyperthreading(
    pids: Rc<RefCell<Option<Vec<u32>>>>,
    num_cores: Rc<ApplicationKnob<u64>>,
    cores: Vec<usize>,
    id: usize,
) -> Rc<ApplicationKnob<u64>> {
    let allowed_values = match std::env::var(format!("HYPERTHREADING_{}", id)) {
        Ok(values) => values.split(',').map(|i| i.parse().unwrap()).collect(),
        Err(_) => vec![0, 1],
    };
    let init_value = match std::env::var(format!("INIT_HYPERTHREADING_{}", id)) {
        Ok(value) => value.parse().unwrap(),
        Err(_) => 1,
    };

    let core_pairs: Vec<(usize, usize)> = cores
        .iter()
        .zip(cores.iter().skip(cores.len() / 2))
        .map(|(&t0, &t1)| (t0, t1))
        .collect();

    Rc::new(ApplicationKnob::new(
        "hyperthreading".to_string(),
        allowed_values,
        init_value,
        Some(Box::new(move |_prev, new| {
            toggle_hyperthreading(&pids.borrow(), num_cores.get(), new, core_pairs.iter());
        })),
    ))
}
