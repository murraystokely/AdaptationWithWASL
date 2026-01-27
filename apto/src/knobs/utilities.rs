use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};

pub fn get_affinity() -> libc::cpu_set_t {
    unsafe {
        let mut mask: libc::cpu_set_t = std::mem::zeroed();
        libc::CPU_ZERO(&mut mask);
        libc::sched_getaffinity(0, std::mem::size_of::<libc::cpu_set_t>(), &mut mask);
        mask
    }
}

pub fn set_thread_affinity(cpu_mask: libc::cpu_set_t) {
    // let children = std::fs::read_dir("/proc/self/task")
    //     .unwrap_or_else(|e| panic!("Could not read children tasks for process: {:?}", e));
    // for chld in children {
    //     let chld_pid: i32 = chld
    //         .unwrap_or_else(|e| panic!("Could not read directory entry: {:?}", e))
    //         .file_name()
    //         .as_os_str()
    //         .to_str()
    //         .unwrap()
    //         .parse()
    //         .unwrap_or_else(|e| panic!("Could not prase chld pid to i32: {:?}", e));

    //     unsafe {
    //         libc::sched_setaffinity(chld_pid, std::mem::size_of::<libc::cpu_set_t>(), &cpu_mask)
    //     };
    // }
}

pub fn set_frequency<T>(target_freq: u64, cores: T)
where
    T: Iterator<Item = usize>,
{
    // Convert from MHz to Hz
    // let target_freq = target_freq.to_string();
    // for core_num in cores {
    //     let filename = format!(
    //         "/sys/devices/system/cpu/cpu{}/cpufreq/scaling_max_freq",
    //         core_num
    //     );
    //     let mut file = OpenOptions::new()
    //         .write(true)
    //         .open(filename)
    //         .unwrap_or_else(|e| panic!("Could not open frequency file for {}: {:?}", core_num, e));
    //     file.write_all(target_freq.as_bytes()).unwrap_or_else(|e| {
    //         panic!("Could not write to frequency file of {}: {:?}", core_num, e)
    //     });
    // }
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

pub fn core_online(id: usize) -> bool {
    if id == 0 {
        return true;
    }

    std::fs::read_to_string(format!("/sys/devices/system/cpu/cpu{}/online", id))
        .unwrap()
        .trim()
        == "1"
}

pub fn get_package_default_cores() -> Vec<usize> {
    let mut socket_cores: HashMap<usize, usize> = HashMap::new();
    let reader = BufReader::new(std::fs::File::open("/proc/cpuinfo").unwrap());
    let mut prev_core = 0;
    for line in reader.lines().filter_map(|r| r.ok()) {
        if line.starts_with("processor") {
            prev_core = line.split(':').nth(1).unwrap().trim().parse().unwrap();
        }
        if line.starts_with("physical id") {
            let socket = line.split(':').nth(1).unwrap().trim().parse().unwrap();
            socket_cores.entry(socket).or_insert(prev_core);
        }
    }
    socket_cores.values().cloned().collect()
}

pub fn write_msr(processor: i64, reg: u64, value: &[u8]) {
    // let filename = format!("/dev/cpu/{}/msr", processor);
    // let mut file = OpenOptions::new()
    //     .write(true)
    //     .open(filename)
    //     .unwrap_or_else(|e| panic!("Could not open MSR file for {}: {:?}", processor, e));
    // let _ = file.seek(SeekFrom::Start(reg));
    // let _ = file.write_all(value).unwrap();
}

pub fn read_msr(processor: i64, reg: u64)  {
    // let mut register_value: [u8; 8] = [0; 8];
    // let filename = format!("/dev/cpu/{}/msr", processor);
    // let mut file = OpenOptions::new()
    //     .read(true)
    //     .open(filename)
    //     .unwrap_or_else(|e| {
    //         panic!(
    //             "Could not open MSR file to read uncore frequency for testing: {:?}",
    //             e
    //         )
    //     });
    // let _ = file.seek(SeekFrom::Start(reg));
    // let _ = file.read_exact(&mut register_value);

    // u64::from_ne_bytes(register_value)
}
