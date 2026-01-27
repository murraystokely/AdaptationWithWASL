use crate::optimize::AptoMode;

pub(crate) struct Measurement {
    values: Vec<f64>,
    window_values: Vec<f64>,
    nr_values: u64,
    total_average: f64,
    agg_func: Option<Box<dyn Fn(&[f64]) -> f64>>,
}

impl Measurement {
    pub fn new(window_size: u64, agg_func: Option<Box<dyn Fn(&[f64]) -> f64>>) -> Measurement {
        let mut window_values = Vec::new();
        window_values.reserve(window_size as usize);
        Measurement {
            values: Vec::new(),
            window_values,
            nr_values: 0,
            total_average: 0.0,
            agg_func,
        }
    }

    pub fn reset_window(&mut self) {
        self.window_values.clear();
    }

    pub fn reset_complete(&mut self) {
        self.values.clear();
        self.window_values.clear();
        self.nr_values = 0;
        self.total_average = 0.0;
    }

    pub fn aggregate(&self) -> f64 {
        match self.agg_func.as_ref() {
            None => {
                self.window_values.iter().fold(0.0, |acc, x| acc + x)
                    / (self.window_values.len() as f64)
            }
            Some(func) => func(&self.window_values),
        }
    }

    pub fn prev_value(&self) -> Option<f64> {
        self.window_values.last().copied()
    }

    pub fn register_value(&mut self, value: f64, apto_mode: &AptoMode) {
        if let AptoMode::Profile(_) = apto_mode {
            self.values.push(value);
        }

        self.window_values.push(value);
        self.nr_values += 1;

        let nr_vals = self.nr_values as f64;
        self.total_average =
            ((nr_vals - 1.0) / nr_vals) * self.total_average + (1.0 / nr_vals) * value;
    }

    pub fn total_average(&self) -> f64 {
        self.total_average
    }
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;
    use rand::Rng;

    use super::*;

    #[test]
    fn total_average() {
        let mut rng = rand::thread_rng();
        let mut m = Measurement::new(40, None);
        let mut values = Vec::with_capacity(5000);
        for _ in 0..5000 {
            let new_val = rng.gen_range(0.0..10000.0);
            values.push(new_val);
            m.register_value(new_val, &AptoMode::Profile(10));
        }
        let expected = values.iter().fold(0.0, |acc, v| acc + v) / (values.len() as f64);
        assert!(expected - m.aggregate() <= f64::EPSILON);
    }

    #[test]
    fn window_average() {
        let mut rng = rand::thread_rng();
        let mut m = Measurement::new(40, None);
        let mut values = Vec::new();
        for idx in 0..100 {
            let new_val = rng.gen_range(0.0..10000.0);
            values.push(new_val);
            m.register_value(new_val, &AptoMode::Adaptive);
            if idx % 40 == 0 {
                let expected = values.iter().fold(0.0, |acc, v| acc + v) / (values.len() as f64);
                assert!(expected - m.aggregate() <= f64::EPSILON);
                values.clear();
                m.reset_window();
            }
        }
    }

    #[test]
    fn measures_aggregation_percentile() {
        let mut m = Measurement::new(5, Some(make_percentile_function(0.4)));
        for number in [15, 20, 35, 40, 50] {
            m.register_value(number as f64, &AptoMode::Adaptive);
        }
        assert!((m.aggregate() - 29.0).abs() < f64::EPSILON);

        let mut m = Measurement::new(5, Some(make_percentile_function(0.75)));
        for number in [1, 2, 3, 4] {
            m.register_value(number as f64, &AptoMode::Adaptive);
        }
        assert!((m.aggregate() - 3.25).abs() < f64::EPSILON);
    }

    fn make_percentile_function(ptile: f64) -> Box<dyn Fn(&[f64]) -> f64> {
        Box::new(move |values: &[f64]| {
            let index = ((values.len() as f64) - 1.0) * ptile;
            let values: Vec<f64> = values
                .iter()
                .cloned()
                .sorted_by(|a, b| a.partial_cmp(b).unwrap())
                .collect();
            values[index.floor() as usize]
                + index.fract() * (values[index.ceil() as usize] - values[index.floor() as usize])
        })
    }
}
