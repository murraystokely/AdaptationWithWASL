use energy_monitor::EnergyMonitor;
use energymon::EnergyMon as EM;
use std::time::Instant;

pub struct Energymon {
    em: EM,
    start_energy: u64,
    end_energy: u64,
    start_time: Instant,
    end_time: Instant,
}

impl Energymon {
    pub fn new() -> Energymon {
        let em = EM::new().expect("Could not initialize Energy Monitor.");
        Energymon {
            em,
            start_energy: 0,
            end_energy: 0,
            start_time: Instant::now(),
            end_time: Instant::now(),
        }
    }

    pub fn start(&mut self) {
        self.reset();

        self.start_energy = self.em.read_uj().expect("Could not read energy.");
        self.start_time = Instant::now();
    }

    pub fn stop(&mut self) {
        self.end_energy = self.em.read_uj().expect("Could not read energy.");
        self.end_time = Instant::now();
    }

    pub fn reset(&mut self) {
        self.start_energy = 0;
        self.end_energy = 0;
        self.start_time = Instant::now();
        self.end_time = Instant::now();
    }

    pub fn energy_delta(&self) -> Result<f64, &str> {
        match (self.end_energy - self.start_energy) as f64 {
            energy_delta if (energy_delta - 0.0).abs() < 0.0001 => {
                Err("Zero energy consumed. Cannot calculate power consumption")
            }
            energy_delta => Ok(energy_delta),
        }
    }

    pub fn power_consumption(&self) -> Result<f64, &str> {
        match (self.energy_delta(), self.duration()) {
            (Ok(energy_delta), Ok(elapsed)) => Ok(energy_delta / (elapsed)),
            (Err(_), Err(_)) => Err("Zero time and energy spent. Cannot calculate power consumption."),
            (Err(_), _) => Err("Zero energy spent during Energymon start and stop. Cannot calculate power consumption."),
            (_, Err(_)) => Err("Zero time spent during Energymon start and stop. Cannot calculate power consumption.")
        }
    }

    pub fn energy(&self) -> f64 {
        self.end_energy as f64
    }

    pub fn current_energy(&self) -> f64 {
        self.em.read_uj().expect("Could not read energy") as f64
    }

    pub fn duration(&self) -> Result<f64, &str> {
        match self.end_time.duration_since(self.start_time).as_secs_f64() {
            elapsed_time if (elapsed_time - 0.0).abs() < 0.000000001 => {
                Err("Zero time spent during Energymon start and stop.")
            }
            elapsed_time => Ok(elapsed_time),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Energymon;
    use std::time::Duration;

    #[ignore]
    #[test]
    fn energy() {
        let mut em = Energymon::new();
        em.start();
        std::thread::sleep(Duration::from_secs(1));
        em.stop();
        let energy_delta = em.energy_delta().unwrap();
        assert!(energy_delta > 0.0);
    }
}
