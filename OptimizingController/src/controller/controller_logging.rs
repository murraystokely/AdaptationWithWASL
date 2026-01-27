use core::fmt;
use log::debug;

use super::XupState;
use crate::KalmanFilter;

pub struct Log {
    nr_schedule: usize,
    pub(crate) tag: u64,
    pub(crate) measured_constraint: f64,
    pub(crate) workload: f64,
    kf_x_hat_minus: f64,
    pub(crate) kf_x_hat: f64,
    kf_p_minus: f64,
    kf_h: f64,
    pub(crate) kf_k: f64,
    kf_p: f64,
    xs_p1: f64,
    pub(crate) xs_u: f64,
    xs_e: f64,
    pub(crate) diff: f64,
    id_lower: usize,
    id_upper: usize,
    nr_lower_iterations: usize,
    oscillating: bool,
}

// Replace this with serde
impl std::str::FromStr for Log {
    type Err = Box<dyn std::error::Error>;

    fn from_str(log_line: &str) -> Result<Self, Self::Err> {
        let mut e = log_line.split(',');
        Ok(Log {
            nr_schedule: e.next().unwrap().parse()?,
            tag: e.next().unwrap().parse()?,
            measured_constraint: e.next().unwrap().parse()?,
            workload: e.next().unwrap().parse()?,
            kf_x_hat_minus: e.next().unwrap().parse()?,
            kf_x_hat: e.next().unwrap().parse()?,
            kf_p_minus: e.next().unwrap().parse()?,
            kf_h: e.next().unwrap().parse()?,
            kf_k: e.next().unwrap().parse()?,
            kf_p: e.next().unwrap().parse()?,
            xs_p1: e.next().unwrap().parse()?,
            xs_u: e.next().unwrap().parse()?,
            xs_e: e.next().unwrap().parse()?,
            diff: e.next().unwrap().parse()?,
            id_lower: e.next().unwrap().parse()?,
            id_upper: e.next().unwrap().parse()?,
            nr_lower_iterations: e.next().unwrap().parse()?,
            oscillating: e.next().unwrap().parse()?,
        })
    }
}

// Replace these with Serde
impl fmt::Display for Log {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
            self.nr_schedule,
            self.tag,
            self.measured_constraint,
            self.workload,
            self.kf_x_hat_minus,
            self.kf_x_hat,
            self.kf_p_minus,
            self.kf_h,
            self.kf_k,
            self.kf_p,
            self.xs_p1,
            self.xs_u,
            self.xs_e,
            self.diff,
            self.id_lower,
            self.id_upper,
            self.nr_lower_iterations,
            self.oscillating
        )
    }
}

pub struct LogState {
    // For now, we just log to stdout
    // Later on, I'll add a file
    logs: Vec<Log>,
}

impl LogState {
    pub fn new() -> LogState {
        let logs: Vec<Log> = Vec::with_capacity(100);
        LogState { logs }
    }

    // TODO: Make and use a Schedule object instead of the schedule tuples
    pub fn log(
        &mut self,
        nr_schedule: usize,
        tag: u64,
        measured_constraint: f64,
        workload: f64,
        kf: &KalmanFilter,
        xs: &XupState,
        diff: f64,
        id_lower: usize,
        id_upper: usize,
        nr_lower_iterations: usize,
        oscillating: bool,
    ) {
        let new_log = Log {
            nr_schedule,
            tag,
            measured_constraint,
            workload,
            kf_x_hat_minus: kf.x_hat_minus,
            kf_x_hat: kf.x_hat,
            kf_p_minus: kf.p_minus,
            kf_h: kf.h,
            kf_k: kf.k,
            kf_p: kf.p,
            xs_p1: xs.p1,
            xs_u: xs.u,
            xs_e: xs.e,
            diff,
            id_lower,
            id_upper,
            nr_lower_iterations,
            oscillating,
        };

        debug!("{}", new_log);

        self.logs.push(new_log);
    }

    pub fn flush(&self) {
        for log in self.logs.iter() {
            println!("{}", log);
        }
    }

    pub fn second_last(&self) -> Option<&Log> {
        self.logs.get(self.logs.len() - 2)
    }

    pub fn last(&self) -> Option<&Log> {
        self.logs.last()
    }
}

impl fmt::Display for LogState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut output = String::from(
            "ID,Tag,Constraint,Workload \
             ,kf.x_hat_minus,kf.x_hat,kf.p_minus,kf.h,kf.k,kf.p \
             ,xs.pole,xs.u,xs.e \
             ,sched.idLower,sched.idUpper,sched.nLowerIterations,sched.oscillating\n",
        );

        for log in self.logs.iter() {
            output += &log.to_string()
        }

        write!(f, "{}", output)
    }
}
