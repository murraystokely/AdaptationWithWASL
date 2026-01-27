use lazy_static::lazy_static;
use regex::Regex;

mod configurations;
mod goal;
mod knobs;
mod measures;
mod optimize;
mod profile;
mod system_measures;
use goal::Perturbation;

pub use configurations::Configurations as AptoConfig;
pub use goal::Goal;
pub use knobs::{
    ApplicationKnob, AvailablePhysicalCores, AvailablePhysicalThreads, CacheCOS, ConstantKnob,
    CoreFrequency, Hyperthreading, Tunable, UncoreFrequency,
};
pub use optimize::Apto;
pub use OptimizingController::OptimizationType;

lazy_static! {
    static ref NAME_REGEX: Regex = Regex::new("[[:alpha:]]+[a-zA-Z0-9_]*").unwrap();
}

#[cfg(test)]
mod tests {
    use super::knobs::ApplicationKnob;
    use super::AptoConfig;
    use super::{Apto, Goal, OptimizationType, Tunable};
    use std::io::Write;
    use std::rc::Rc;

    fn write_incrementer_profile() {
        let knob_table_string = r#"id,step,threshold
0,1,50000
1,1,200000
2,4,50000
3,4,200000"#;
        let measure_table_string = r#"id,currentConfiguration,energy,energyDelta,iteration,latency,operations,performance,powerConsumption,quality,runningTime,systemEnergy,time,windowSize
0,-1.0,66037923.38,660274.235,99.5,0.21323804974556,50000.0,4.68959456904254,3096418.46653472,1.0,21.3261238873005,66876406.32,1554836505.73334,20.0
1,-1.0,261643351.675,2616127.165,99.5,0.845102413892746,200000.0,1.18328853824208,3095633.28892824,1.0,84.5128715789318,395299728.425,1554836611.82094,20.0
2,-1.0,17062789.245,170605.0,99.5,0.0550970566272736,12500.0,18.1497898656349,3096444.90002664,0.25,5.50908312678337,676698295.14,1554836702.72642,20.0
3,-1.0,65982635.43,659723.43,99.5,0.213206874132156,50000.0,4.69028029265206,3094287.80232982,0.25,21.3219397556782,760037632.135,1554836729.65205,20.0
"#;

        std::fs::File::create("/tmp/kt_incrementer")
            .expect("Could not create file for incrementer knob table.")
            .write_all(knob_table_string.as_bytes())
            .expect("Could not write knob table for incrementer.");
        std::fs::File::create("/tmp/mt_incrementer")
            .expect("Could not create file for incrementer measure table.")
            .write_all(measure_table_string.as_bytes())
            .expect("Could not write measure table for incrementer.")
    }

    fn moving_averages(values: &[f64], window: usize) -> Vec<f64> {
        let mut averages = Vec::new();

        let mut total = 0.0;
        let mut left = 0;

        for right in 0..values.len() {
            let nr_values = right - left + 1;
            if nr_values > window {
                total -= values[left];
                left += 1;
            }
            total += values[right];
            averages.push(total / nr_values as f64);
        }

        averages
    }

    fn calculate_mape(values: &[f64], target: f64) -> f64 {
        let mut total = 0.0;
        for value in values {
            total += ((value - target).abs()) / target;
        }
        100.0 * total / values.len() as f64
    }

    #[ignore]
    #[test]
    fn incrementer() {
        write_incrementer_profile();

        let threshold: Rc<ApplicationKnob<u64>> = Rc::new(ApplicationKnob::new(
            "threshold".to_string(),
            vec![50000, 200000],
            200000,
            None,
        ));
        let step: Rc<ApplicationKnob<u64>> = Rc::new(ApplicationKnob::new(
            "step".to_string(),
            vec![1, 4],
            1,
            None,
        ));
        let knobs: Vec<Rc<dyn Tunable<u64>>> = vec![threshold.clone(), step.clone()];

        let target = 0.5;
        let goal = Goal::new(
            "quality".to_string(),
            target,
            OptimizationType::Maximize,
            "1.0 / (operations * operations)".to_string(),
        );

        let config = AptoConfig::new(
            0,
            "/tmp/mt_incrementer",
            "/tmp/kt_incrementer",
            knobs,
            goal,
            20,
        );
        let mut apto = Apto::new(config);

        let mut x = 0;
        let mut iteration = 0;
        let mut qualities = Vec::with_capacity(500);
        apto.optimize(
            None,
            None,
            Box::new(|apto: &mut Apto| {
                let mut operations = 0.0;
                while x < threshold.get() {
                    x += step.get();
                    operations += 1.0;
                }
                x = 0;
                apto.measure("operations", operations);

                let quality = 1.0 / step.get() as f64;
                apto.measure("quality", quality);

                qualities.push(quality);

                iteration += 1;
                iteration != 500
            }),
        );

        let window_averages = moving_averages(&qualities, 20);
        let mape = calculate_mape(&window_averages, target);

        assert!(mape < 15.0);

        std::fs::remove_file("/tmp/kt_incrementer")
            .expect("Could not delete knob table for incrementer");
        std::fs::remove_file("/tmp/mt_incrementer")
            .expect("Could not delete measure table for incrementer");
    }
}
