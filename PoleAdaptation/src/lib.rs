use log::info;
use std::collections::VecDeque;

#[derive(Debug)]
enum AdaptationType {
    NoAdaptation,
    Linear,
    Ewma(f64, f64),
    Modeled(VecDeque<f64>),
}

#[derive(Debug)]
pub struct PoleAdaptation {
    pub target: f64,
    methodology: AdaptationType,
    prev: f64,
    eo: f64,
    eoo: f64,
    tag: u64,
}

impl PoleAdaptation {
    pub fn new(tag: u64) -> PoleAdaptation {
        println!("tag: {}", tag);
        // Pole Adaptation is constructed using environment variables
        if let Ok(instances) = std::env::var("ADAPT_INST") {
            if instances
                .split(',')
                .filter(|i| !i.is_empty())
                .any(|i| i.parse::<u64>().unwrap() == tag)
            {
                let target: f64 = std::env::var("DEV_TARGET")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or_else(|| panic!("Require a valid DEV_TARGET"));
                let methodology = match std::env::var("ADAPT_TYPE").unwrap().as_str() {
                    "linear" => Ok(AdaptationType::Linear),
                    "ewma" => {
                        let alpha = std::env::var("ALPHA")
                            .ok()
                            .and_then(|v| v.parse().ok())
                            .unwrap_or_else(|| panic!("Could not parse alpha"));
                        Ok(AdaptationType::Ewma(alpha, f64::INFINITY))
                    }
                    "modeled" => {
                        let model_path = std::env::var(format!("MODEL_PATH_{}", tag))
                            .unwrap_or_else(|_| panic!("Require path for forecasted values"));
                        let model: VecDeque<f64> = std::fs::read_to_string(model_path)
                            .unwrap()
                            .lines()
                            .map(|l| l.trim().parse().unwrap())
                            .collect();
                        Ok(AdaptationType::Modeled(model))
                    }
                    incorrect => Err(format!("Incorrect adaptation type: {}", incorrect)),
                }
                .unwrap();
                return PoleAdaptation {
                    target,
                    methodology,
                    prev: 1.0,
                    eo: 0.0,
                    eoo: 0.0,
                    tag,
                };
            }
        }
        PoleAdaptation::default()
    }

    pub fn calculate_multiplier(&mut self, mdiff: f64, _measured: f64, workload: f64) -> f64 {
        let e = mdiff / workload;

        if self.eo == 0.0 || self.eoo == 0.0 {
            self.eoo = self.eo;
            self.eo = e;
            if let AdaptationType::Modeled(forecast) = &mut self.methodology {
                forecast.pop_front().unwrap();
            }
            return self.prev;
        }

        let mut d = e - 2.0 * self.eo + self.eoo;
        d = match &mut self.methodology {
            AdaptationType::NoAdaptation => return self.prev,
            AdaptationType::Linear => d,
            AdaptationType::Ewma(alpha, average) => {
                let d = if *average == f64::INFINITY {
                    d
                } else {
                    (*alpha) * (*average) + (1.0 - (*alpha)) * d
                };

                *average = d;

                d
            }
            AdaptationType::Modeled(forecast) => forecast.pop_front().unwrap(),
        };

        self.eoo = self.eo;
        self.eo = e;

        let inacc = d.abs();

        let pole = if inacc > self.target {
            (1.0 - (self.target / inacc)).max(0.0).min(0.95)
        } else {
            1.0 - self.prev
        };

        self.prev = 1.0 - pole;

        info!(
                "Multiplier Adaptation :: tag: {}, workload: {}, pole: {}, derivative: {}, inaccuracy: {}, e: {}, eo: {}, eoo: {}",
                self.tag, 1.0 / workload, pole, d, inacc, e, self.eo, self.eoo
            );

        1.0 - pole
    }
}

impl Default for PoleAdaptation {
    fn default() -> PoleAdaptation {
        PoleAdaptation {
            target: f64::default(),
            methodology: AdaptationType::NoAdaptation,
            prev: 1.0,
            eo: 0.0,
            eoo: 0.0,
            tag: 0,
        }
    }
}
