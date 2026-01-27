use log::{info, trace, warn};
use std::collections::BTreeMap;

use super::{ControllerContext, LogState, SchedType, XupState};
use crate::{ExpressionType, KalmanFilter, OptimizationType};
use PoleAdaptation::PoleAdaptation;
pub struct OptimizingController {
    pub tag: u64,
    pub ctx: ControllerContext,
    pub kf: KalmanFilter,
    pub xs: XupState,
    pub ls: LogState,
    pub nr_schedules: usize,
    pub pole_adaptation: PoleAdaptation,
    pub sched_type: SchedType,
    pub sched_xup: f64, 
}


impl OptimizingController {
    // TODO: All objective function details should be moved into their own struct
    //       This is for much much later
    pub fn new(
        tag: u64,
        model: Vec<Vec<f64>>,
        filtered: Vec<Vec<f64>>,
        constraint: f64,
        constrained_measure_idx: usize,
        window: usize,
        opt_type: OptimizationType,
        obj_func: &str,
        obj_measures: Vec<String>,
        initial_model_entry_idx: usize,
    ) -> OptimizingController {
        let ctx = ControllerContext::new(
            model,
            filtered,
            constraint,
            constrained_measure_idx,
            window,
            opt_type,
            obj_func,
            obj_measures,
        );

        trace!("XupModel: {:?}", ctx.xup_model);

        let initial_xup = ctx.xup_model[initial_model_entry_idx];
        let xs = XupState::new(tag, initial_xup);

        info!(
            "Initialized Controller with model_entry_idx: {}, initial_xup: {}",
            initial_model_entry_idx, initial_xup
        );

        let pole_adaptation = PoleAdaptation::new(tag);
        let sched_type = SchedType::new(tag);

        info!(
            "Using {}, {:?} scheduler (tag: {})",
            sched_type, pole_adaptation, tag
        );

        OptimizingController {
            tag,
            ctx,
            kf: KalmanFilter::new(tag),
            xs,
            ls: LogState::new(),
            nr_schedules: 0,
            pole_adaptation,
            sched_type,
            sched_xup: initial_xup,
        }
    }

    fn compute_sched_and_cost(
        &self,
        target: f64,
        upper_idx: usize,
        lower_idx: usize,
        upper: f64,
        lower: f64,
        expr_vars: &mut BTreeMap<String, f64>,
    ) -> (f64, usize) {
        let x = if upper <= lower {
            0.0
        } else {
            ((upper * lower) - (target * lower)) / ((upper * target) - (target * lower))
        };

        // TODO: Will need to speed this up.
        let interpolated: Vec<f64> = unsafe {
            (0..self.ctx.cost_model.get_unchecked(0).len())
                .map(|idx| {
                    x * self
                        .ctx
                        .cost_model
                        .get_unchecked(lower_idx)
                        .get_unchecked(idx)
                        + (1.0 - x)
                            * self
                                .ctx
                                .cost_model
                                .get_unchecked(upper_idx)
                                .get_unchecked(idx)
                })
                .collect()
        };

        // Small optimizations here
        let cost = match self.ctx.expr_type {
            ExpressionType::Value => unsafe { *interpolated.get_unchecked(0) },
            ExpressionType::Expression => {
                if !expr_vars.is_empty() {
                    for (val, name) in interpolated.iter().zip(self.ctx.obj_measures.iter()) {
                        let e = expr_vars.get_mut(name).unwrap();
                        *e = *val;
                    }
                } else {
                    for (val, name) in interpolated.iter().zip(self.ctx.obj_measures.iter()) {
                        expr_vars.insert(name.to_string(), *val);
                    }
                }
                self.ctx.opt_expr.evaluate(expr_vars).unwrap()
            }
        };

        let nr_iterations = ((self.ctx.window as f64) * x).round() as usize;

        (cost, nr_iterations)
    }

    fn compute_optimal_schedule(&self, target: f64) -> (usize, usize, usize) {
        let mut best_cost: f64 = match self.ctx.opt_type {
            OptimizationType::Maximize => f64::NEG_INFINITY,
            OptimizationType::Minimize => f64::INFINITY,
        };
        let mut expr_vars = BTreeMap::new();

        // schedule => (lower, upper, nr_lower_iterations)
        let mut schedule: (usize, usize, usize) = (0, 0, 0);

        for (i, upper) in self.ctx.xup_model.iter().enumerate() {
            if upper < &target {
                continue;
            }
            for (j, lower) in self.ctx.xup_model.iter().enumerate() {
                if lower > &target {
                    continue;
                }
                let (cost_estimate, nr_iterations) =
                    self.compute_sched_and_cost(target, i, j, *upper, *lower, &mut expr_vars);
                let is_best = match self.ctx.opt_type {
                    OptimizationType::Maximize => cost_estimate > best_cost,
                    OptimizationType::Minimize => cost_estimate < best_cost,
                };
                if is_best {
                    schedule = (j, i, nr_iterations);
                    best_cost = cost_estimate;
                }
            }
        }

        schedule
    }

    fn compute_single_best_action(&mut self, target: f64) -> (usize, usize, usize) {
        let (best_config, xup) = self
            .ctx
            .xup_model
            .iter()
            .enumerate()
            .map(|(idx, xup)| (idx, -(*xup - target).abs()))
            .max_by(|config_x, config_y| config_x.1.partial_cmp(&config_y.1).unwrap())
            .unwrap();
        self.sched_xup = xup;
        (best_config, best_config, self.ctx.window)
    }


    // pub async fn call_tool(&mut self, measured_constraint: f64) -> Result<f64, String> {
    //     let measurement_difference = (self.sched_xup * (1.0 / self.kf.x_hat)) - measured_constraint;
    //     let x_hat = self.kf.x_hat;
    //     let url = "http://127.0.0.1:8080/calculate";
    
    //     let request_body = CalculationRequest {
    //         mdiff: measurement_difference,
    //         measured: measured_constraint,
    //         workload: x_hat,
    //     };
    
    //     let client = Client::new();
    
    //     let response = client
    //         .post(url)
    //         .json(&request_body)
    //         .send()
    //         .await
    //         .map_err(|e| format!("Request error: {}", e))?;
    
    //     let json_response: Value = response
    //         .json()
    //         .await
    //         .map_err(|e| format!("JSON parsing error: {}", e))?;
    
    //     json_response
    //         .get("result")
    //         .and_then(|v| v.as_f64())
    //         .ok_or_else(|| "Missing or invalid 'result' field in response".to_string())
    // }
    

    pub fn compute_schedule(&mut self, measured_constraint: f64, multiplier : f64) -> (u64, u64, u64) {
        let measurement_difference = (self.sched_xup * (1.0 / self.kf.x_hat)) - measured_constraint;

        let workload = self
            .kf
            .estimate_base_workload(self.sched_xup, measured_constraint);

        // Adapt multiplier
        // Will always return 1 if no adaptation needs to be performed
        // self.pole_adaptation.calculate_multiplier(
        //     measurement_difference,
        //     measured_constraint,
        //     self.kf.x_hat,
        // );
        match self.sched_type {
            SchedType::ControlMultiConf => self.set_multiplier(multiplier),
            SchedType::RLMultiConf | SchedType::RLSingleConf => self.kf.a = multiplier,
        };

        let xup = self.xs.calculate_xup(
            self.ctx.constraint,
            measured_constraint,
            workload,
            *self.ctx.xup_model.last().unwrap(),
        );
        self.sched_xup = xup; // The controller wants us to get this xup

        let (id_lower, id_upper, nr_lower_iterations) = match self.sched_type {
            SchedType::RLSingleConf => self.compute_single_best_action(xup), // If using single config we might end up using a different xup than required
            _ => self.compute_optimal_schedule(xup), // Use optimizer for both RLMultiConf and ControlMultiConf
        };

        info!(
            "tag: {}, measured: {}, workload: {}, derivative: {}, xup: {}, sched_xup: {}",
            self.tag, measured_constraint, workload, self.xs.ed, xup, self.sched_xup
        );

        self.ls.log(
            self.nr_schedules,
            self.tag,
            measured_constraint,
            workload,
            &self.kf,
            &self.xs,
            measurement_difference / self.kf.x_hat,
            id_lower,
            id_upper,
            nr_lower_iterations,
            false,
        );
        self.nr_schedules += 1;

        (id_lower as u64, id_upper as u64, nr_lower_iterations as u64)
    }

    pub fn adapt_multiplier(
        &mut self,
        mdiff: f64,
        _measured_constraint: f64,
        derivative_target: f64,
    ) {
        if let (Some(last), Some(second_last)) = (self.ls.last(), self.ls.second_last()) {
            // let (e, eo, eoo) = (
            //     self.ctx.constraint - measured_constraint,
            //     self.xs.e,
            //     self.xs.eo,
            // );

            let current_workload = 1.0 / self.kf.x_hat;

            let (e, eo, eoo) = (current_workload * mdiff, last.diff, second_last.diff);

            let derivative = e - 2.0 * eo + eoo;
            let inaccuracy = derivative.abs();
            let pole = if inaccuracy > derivative_target {
                (1.0 - (derivative_target / inaccuracy)).max(0.0).min(0.95)
            } else {
                match self.sched_type {
                    SchedType::ControlMultiConf => 1.0 - self.xs.get_multiplier(),
                    SchedType::RLMultiConf | SchedType::RLSingleConf => self.kf.a,
                }
            };

            info!(
                "Multiplier Adaptatoin :: tag: {}, workload: {}, pole: {}, derivative: {}, inaccuracy: {}, e: {}, eo: {}, eoo: {}",
                self.tag, current_workload, pole, derivative, inaccuracy, e, eo, eoo
            );
            match self.sched_type {
                SchedType::ControlMultiConf => self.set_multiplier(1.0 - pole),
                SchedType::RLMultiConf | SchedType::RLSingleConf => {
                    self.kf.set_multiplier(1.0 - pole)
                }
            };
        } else {
            warn!("Multiplier cannot be adapted without a window. Ignoring adaptation call.")
        }
    }

    pub fn set_gain(&mut self, val: f64) {
        self.xs.set_gain(val);
    }

    pub fn set_multiplier(&mut self, val: f64) {
        self.xs.set_multiplier(val);
    }

    pub fn set_derivative_multiplier(&mut self, val: f64) {
        self.xs.set_derivative_gain(val);
    }

    pub fn flush_logs(&self) {
        self.ls.flush();
    }

    pub fn change_objective(
        &mut self,
        opt_type: OptimizationType,
        opt_expr_str: &str,
        obj_measures: Vec<String>,
        cost_model: Vec<Vec<f64>>,
    ) {
        self.ctx
            .change_opt_expr(opt_type, opt_expr_str, obj_measures, cost_model);
    }

    pub fn change_target(&mut self, new_value: f64) {
        self.ctx.change_target_value(new_value);
    }
}



#[cfg(test)]
mod tests {
    use super::PoleAdaptation;

    #[test]
    fn initialize_pole_adaptation() {
        todo!()
    }

    #[test]
    #[should_panic]
    fn missing_derivative_target() {
        std::env::set_var("ADAPT_INST", "0");
        std::env::set_var("ADAPT_TYPE", "inaccuracy");
        std::env::remove_var("DEV_TARGET");
        let _ = PoleAdaptation::new(0);
    }
}