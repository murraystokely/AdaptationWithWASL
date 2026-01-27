use crate::OptimizationType;
use crate::{ExpressionType, ObjectiveFunction};

pub struct ControllerContext {
    pub constraint: f64,
    pub constrained_measure_idx: usize,
    pub opt_type: OptimizationType,
    pub opt_expr: ObjectiveFunction,
    pub expr_type: ExpressionType,
    pub model: Vec<Vec<f64>>,
    pub cost_model: Vec<Vec<f64>>,
    pub xup_model: Vec<f64>,
    pub window: usize,
    pub obj_measures: Vec<String>,
}

impl ControllerContext {
    pub fn new(
        model: Vec<Vec<f64>>,
        cost_model: Vec<Vec<f64>>,
        constraint: f64,
        constrained_measure_idx: usize,
        period: usize,
        opt_type: OptimizationType,
        opt_expr_str: &str,
        obj_measures: Vec<String>,
    ) -> ControllerContext {
        // Get the Xup model
        let base_value = model[0][constrained_measure_idx];
        let mut xup_model: Vec<f64> = model
            .iter()
            .map(|model_entry| model_entry[constrained_measure_idx] / base_value)
            .collect();
        xup_model[0] = 1.0;

        let expr_type = match obj_measures.len() {
            0 | 1 => ExpressionType::Value,
            _ => ExpressionType::Expression,
        };

        ControllerContext {
            constraint,
            constrained_measure_idx,
            opt_type,
            opt_expr: ObjectiveFunction::new(opt_expr_str),
            expr_type,
            model,
            cost_model,
            xup_model,
            window: period,
            obj_measures,
        }
    }

    pub fn change_opt_expr(
        &mut self,
        opt_type: OptimizationType,
        opt_expr_str: &str,
        obj_measures: Vec<String>,
        cost_model: Vec<Vec<f64>>,
    ) {
        let expr_type = match obj_measures.len() {
            0 | 1 => ExpressionType::Value,
            _ => ExpressionType::Expression,
        };
        self.obj_measures = obj_measures;
        self.expr_type = expr_type;
        self.opt_type = opt_type;
        self.opt_expr = ObjectiveFunction::new(opt_expr_str);
        self.cost_model = cost_model;
    }

    pub fn change_target_value(&mut self, val: f64) {
        self.constraint = val;
    }
}
