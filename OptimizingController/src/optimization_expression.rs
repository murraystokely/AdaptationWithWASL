use fasteval::Compiler;
use fasteval::Evaler;
use std::cell::RefCell;
use std::collections::BTreeMap;

pub enum ExpressionType {
    Value,
    Expression,
}

pub struct ObjectiveFunction {
    slab: fasteval::Slab,
    expr: RefCell<fasteval::Instruction>,
}

impl ObjectiveFunction {
    pub fn new(obj: &str) -> ObjectiveFunction {
        let parser = fasteval::Parser::new();
        let mut slab = fasteval::Slab::new();
        let expr = parser
            .parse(obj, &mut slab.ps)
            .unwrap()
            .from(&slab.ps)
            .compile(&slab.ps, &mut slab.cs);
        ObjectiveFunction {
            slab,
            expr: RefCell::new(expr),
        }
    }

    pub fn evaluate(&self, value_map: &mut BTreeMap<String, f64>) -> Result<f64, anyhow::Error> {
        let expr = self.expr.take();
        let value = fasteval::eval_compiled!(expr, &self.slab, value_map);
        *self.expr.borrow_mut() = expr;
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::ObjectiveFunction;
    #[test]
    fn test_expr() {
        let expr = ObjectiveFunction::new("performance / powerConsumption");
        let mut map = std::collections::BTreeMap::new();
        map.insert("performance".to_string(), 100f64);
        map.insert("powerConsumption".to_string(), 10f64);
        assert!(expr.evaluate(&mut map).unwrap() - 10.0 < 0.0001);

        map.insert("performance".to_string(), 200f64);
        map.insert("powerConsumption".to_string(), 10f64);
        assert!(expr.evaluate(&mut map).unwrap() - 20.0 < 0.0001);
    }
}
