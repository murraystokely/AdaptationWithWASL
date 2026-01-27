use log::warn;

pub struct XupState {
    // xups
    pub u: f64,
    uo: f64,
    uoo: f64,
    pub ud: f64,
    up: f64,
    xup_func: fn(&mut Self, f64, f64, f64, f64) -> f64,
    // errors
    pub e: f64,
    pub eo: f64,
    pub eoo: f64,
    pub ed: f64,
    // dominant pole
    pub p1: f64,
    // constants
    // second pole
    p2: f64,
    // zero
    z1: f64,
    // gain
    mu: f64,
    // Gains
    k: f64,
    kd: f64,
    kp: f64,
}

impl XupState {
    pub fn new(tag: u64, start: f64) -> XupState {
        let xup_func = if std::env::var(format!("LEARNING_BASED_{}", tag)).is_ok() {
            warn!("Using learning based xup calculation (tag: {})", tag);
            XupState::learning_based_calculate_xup
        } else {
            XupState::control_based_calculate_xup
        };

        let kp = std::env::var(format!("KP_{}", tag))
            .ok()
            .map(|v| {
                warn!("Using KP: {} (tag: {})", v, tag);
                v.parse().unwrap()
            })
            .unwrap_or_default();

        XupState {
            u: start,
            uo: start,
            uoo: start,
            ud: 0.0,
            up: 0.0,
            xup_func,
            e: 0.0,
            eo: 0.0,
            eoo: 0.0,
            ed: 0.0,
            p1: 0.0,
            p2: 0.0,
            z1: 0.0,
            mu: 1.0,
            k: 1.0,
            kd: 0.0,
            kp,
        }
    }

    pub fn set_gain(&mut self, val: f64) {
        self.mu = val;
    }

    pub fn get_multiplier(&self) -> f64 {
        self.k
    }

    pub fn set_multiplier(&mut self, val: f64) {
        self.k = val;
    }

    pub fn set_derivative_gain(&mut self, val: f64) {
        self.kd = val;
    }

    fn control_based_calculate_xup(
        &mut self,
        target: f64,
        measured: f64,
        w: f64,
        max_xup: f64,
    ) -> f64 {
        let a = -(-(self.p1 * self.z1) - (self.p2 * self.z1) + (self.mu * self.p1 * self.p2)
            - (self.mu * self.p2)
            + self.p2
            - (self.mu * self.p1)
            + self.p1
            + self.mu);
        let b = -(-(self.mu * self.p1 * self.p2 * self.z1)
            + (self.p1 * self.p2 * self.z1)
            + (self.mu * self.p2 * self.z1)
            + (self.mu * self.p1 * self.z1)
            - (self.mu * self.z1)
            - (self.p1 * self.p2));
        let c = (((self.mu - (self.mu * self.p1)) * self.p2) + (self.mu * self.p1) - self.mu) * w;
        let d = ((((self.mu * self.p1) - self.mu) * self.p2) - (self.mu * self.p1) + self.mu)
            * w
            * self.z1;
        let f = 1.0 / (self.z1 - 1.0);
        // Save old values
        self.uoo = self.uo;
        self.uo = self.u;
        self.eoo = self.eo;
        self.eo = self.e;
        // compute error
        self.e = (target - measured) * self.k;
        // compute error difference
        self.ed = w * (self.e - 2.0 * self.eo + self.eoo);
        // calculate xup
        self.ud = -(self.kd * self.ed);
        self.up = -1.0 * w * self.kp * (self.e - self.eo);
        self.u =
            f * ((a * self.uo) + (b * self.uoo) + (c * self.e) + (d * self.eo) + self.up + self.ud);
        // xup less than 1 has no effect; greater than the maximum is not achievable
        self.u = self.u.max(1.0).min(max_xup);
        self.u
    }

    fn learning_based_calculate_xup(
        &mut self,
        target: f64,
        measured: f64,
        w: f64,
        max_xup: f64,
    ) -> f64 {
        // Save old values
        self.uoo = self.uo;
        self.uo = self.u;
        self.eoo = self.eo;
        self.eo = self.e;
        // compute error
        self.e = target - measured;
        // compute error difference
        self.ed = w * (self.e - 2.0 * self.eo + self.eoo);
        // calculate xup
        self.ud = -(self.kd * self.ed);

        self.u = target * w;
        // xup less than 1 has no effect; greater than the maximum is not achievable
        self.u = self.u.max(1.0).min(max_xup);
        self.u
    }

    pub fn calculate_xup(&mut self, target: f64, measured: f64, w: f64, max_xup: f64) -> f64 {
        (self.xup_func)(self, target, measured, w, max_xup)
    }
}
