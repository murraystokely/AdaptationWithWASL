use log::warn;

pub struct KalmanFilter {
    pub x_hat_minus: f64,
    pub x_hat: f64,
    pub p_minus: f64,
    pub h: f64,
    pub k: f64,
    pub p: f64,
    pub q: f64,
    pub r: f64,
    pub e: f64,
    // Constant for calculating multiplier using covariance
    pub a: f64,
    estimation_function: fn(&mut Self, f64, f64) -> f64,
}

impl KalmanFilter {
    pub fn new(tag: u64) -> KalmanFilter {
        let mut x_hat = 0.2;

        let estimation_function = if std::env::var(format!("LEARNING_BASED_{}", tag)).is_ok() {
            warn!("Using basic estimator (tag: {})", tag);
            KalmanFilter::basic_estimate_workload
        } else if std::env::var(format!("CONSTANT_WORKLOAD_{}", tag)).is_ok() {
            x_hat = std::env::var(format!("CONSTANT_WORKLOAD_{}", tag))
                .ok()
                .map(|v| v.parse().unwrap())
                .unwrap();
            warn!("Using constant workload: {} (tag: {})", x_hat, tag);
            KalmanFilter::constant_workload
        } else {
            KalmanFilter::kf_estimate_workload
        };

        KalmanFilter {
            x_hat_minus: 0.0,
            x_hat,
            p_minus: 0.0,
            h: 0.0,
            k: 0.0,
            p: 1.0,
            q: 0.00001,
            r: 0.01,
            e: 0.0,
            a: 1.0,
            estimation_function,
        }
    }

    fn kf_estimate_workload(&mut self, xup_prev: f64, workload_prev: f64) -> f64 {
        self.x_hat_minus = self.x_hat;
        self.p_minus = self.p + self.q;
        self.h = xup_prev;
        self.k = (self.p_minus * self.h) / ((self.h * self.p_minus * self.h) + self.r);
        self.e = workload_prev - (self.h * self.x_hat_minus);
        self.x_hat = self.x_hat_minus + (self.k * self.e);
        self.p = (1.0 - (self.k * self.h)) * self.p_minus;
        1.0 / self.x_hat
    }

    fn basic_estimate_workload(&mut self, xup_prev: f64, workload_prev: f64) -> f64 {
        self.x_hat_minus = self.x_hat;
        self.x_hat = (1.0 - self.a) * self.x_hat_minus + self.a * (workload_prev / xup_prev);
        1.0 / self.x_hat
    }

    pub fn estimate_base_workload(&mut self, xup_prev: f64, workload_prev: f64) -> f64 {
        (self.estimation_function)(self, xup_prev, workload_prev)
    }

    pub fn constant_workload(&mut self, _: f64, _: f64) -> f64 {
        1.0 / self.x_hat
    }

    pub fn set_multiplier(&mut self, val: f64) {
        self.a = val;
    }

    pub fn compute_multiplier(&mut self, min_xup: f64, max_xup: f64) -> f64 {
        self.q = self.a * self.q + (1.0 - self.a) * (self.k * self.e * self.e * self.k);
        (1.0 - ((max_xup / min_xup) - 3.0 * self.q).max(0.0)).max(0.0)
    }
}
