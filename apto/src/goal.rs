use serde::{Deserialize, Serialize};
use std::convert::From;
use std::{io::BufReader, path::Path};
use OptimizingController::OptimizationType;

#[derive(Serialize, Deserialize)]
#[serde(remote = "OptimizationType")]
enum OptimizationTypeDef {
    Minimize,
    Maximize,
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct Goal {
    pub(crate) constraint: String,
    pub(crate) target: f64,

    #[serde(with = "OptimizationTypeDef")]
    pub(crate) opt_type: OptimizationType,

    pub(crate) opt_func: String,
}

impl Goal {
    pub fn new(
        constraint: String,
        target: f64,
        opt_type: OptimizationType,
        opt_func: String,
    ) -> Goal {
        Goal {
            constraint,
            target,
            opt_type,
            opt_func,
        }
    }
}

impl std::fmt::Display for Goal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(
            f,
            "{}({}) such that {} == {}",
            self.opt_type, self.opt_func, self.constraint, self.target
        )
    }
}

impl<T> From<T> for Goal
where
    T: AsRef<Path>,
{
    fn from(path: T) -> Goal {
        let reader = BufReader::new(std::fs::File::open(path).unwrap());
        serde_yaml::from_reader(reader).unwrap()
    }
}

#[derive(Debug, PartialEq)]
pub enum Perturbation {
    NoChange,
    ChangeObjective(OptimizationType, String),
    ChangeConstraintValue(f64),
    ChangeEntireGoal(Goal), // Add option for changing knob space
}

impl std::fmt::Display for Perturbation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match &self {
            Perturbation::NoChange => write!(f, "NONE"),
            Perturbation::ChangeObjective(opt_type, opt_func) => {
                return write!(f, "{}({})", opt_type, opt_func)
            }
            Perturbation::ChangeConstraintValue(value) => write!(f, "constraint value: {}", value),
            Perturbation::ChangeEntireGoal(goal) => write!(f, "new goal: {}", goal),
        }
    }
}

impl std::ops::Sub for &Goal {
    type Output = Perturbation;

    fn sub(self, rhs: Self) -> Self::Output {
        if (self.target == rhs.target && self.constraint == rhs.constraint)
            && (self.opt_type == rhs.opt_type && self.opt_func == rhs.opt_func)
        {
            Perturbation::NoChange
        } else if (self.target == rhs.target && self.constraint == rhs.constraint)
            && (self.opt_type != rhs.opt_type || self.opt_func != rhs.opt_func)
        {
            Perturbation::ChangeObjective(self.opt_type, self.opt_func.clone())
        } else if (self.target != rhs.target && self.constraint == rhs.constraint)
            && (self.opt_type == rhs.opt_type && self.opt_func == rhs.opt_func)
        {
            Perturbation::ChangeConstraintValue(self.target)
        } else {
            Perturbation::ChangeEntireGoal(self.clone())
        }
    }
}

#[cfg(test)]
mod tests {
    use OptimizingController::OptimizationType;

    use super::{Goal, Perturbation};

    #[test]
    fn parse_goal_from_yaml() {
        let goal_str = r#"---
constraint: performance
target: 0.01
opt_type: Minimize
opt_func: powerConsumption
"#;
        let parsed_goal = serde_yaml::from_str(goal_str).unwrap();
        assert_eq!(
            Goal::new(
                "performance".to_string(),
                0.01,
                OptimizationType::Minimize,
                "powerConsumption".to_string()
            ),
            parsed_goal
        );

        let goal_str = r#"---
constraint: powerConsumption
target: 100
opt_type: Maximize
opt_func: performance
"#;
        let parsed_goal = serde_yaml::from_str(goal_str).unwrap();
        assert_eq!(
            Goal::new(
                "powerConsumption".to_string(),
                100.0,
                OptimizationType::Maximize,
                "performance".to_string()
            ),
            parsed_goal
        );
    }

    #[test]
    #[should_panic]
    fn parse_incorrect_goal_from_yaml() {
        let goal_str = r#"---
constraint: performance
opt_type: minimize
opt_func: powerConsumption
"#;
        let _: Goal = serde_yaml::from_str(goal_str).unwrap();
    }

    #[test]
    fn perturbation_no_change() {
        let old_goal = Goal::new(
            "latency".to_string(),
            30.0,
            OptimizationType::Maximize,
            "quality".to_string(),
        );
        assert_eq!(&old_goal - &old_goal, Perturbation::NoChange);
    }

    #[test]
    fn perturbation_change_objective() {
        let old_goal = Goal::new(
            "latency".to_string(),
            30.0,
            OptimizationType::Maximize,
            "quality".to_string(),
        );

        let new_goal = Goal::new(
            "latency".to_string(),
            30.0,
            OptimizationType::Minimize,
            "quality".to_string(),
        );
        assert_eq!(
            &new_goal - &old_goal,
            Perturbation::ChangeObjective(OptimizationType::Minimize, "quality".to_string())
        );
        let new_goal = Goal::new(
            "latency".to_string(),
            30.0,
            OptimizationType::Minimize,
            "somethingElse".to_string(),
        );
        assert_eq!(
            &new_goal - &old_goal,
            Perturbation::ChangeObjective(OptimizationType::Minimize, "somethingElse".to_string())
        );
    }

    #[test]
    fn perturbation_change_constraint_value() {
        let old_goal = Goal::new(
            "latency".to_string(),
            30.0,
            OptimizationType::Maximize,
            "quality".to_string(),
        );

        let new_goal = Goal::new(
            "latency".to_string(),
            60.0,
            OptimizationType::Maximize,
            "quality".to_string(),
        );
        assert_eq!(
            &new_goal - &old_goal,
            Perturbation::ChangeConstraintValue(60.0)
        );
    }

    #[test]
    fn perturbation_entire_goal() {
        let old_goal = Goal::new(
            "latency".to_string(),
            30.0,
            OptimizationType::Maximize,
            "quality".to_string(),
        );

        let new_goal = Goal::new(
            "performance".to_string(),
            30.0,
            OptimizationType::Maximize,
            "quality".to_string(),
        );
        assert_eq!(
            &new_goal - &old_goal,
            Perturbation::ChangeEntireGoal(new_goal)
        );
        let new_goal = Goal::new(
            "latency".to_string(),
            60.0,
            OptimizationType::Maximize,
            "somethingElse".to_string(),
        );
        assert_eq!(
            &new_goal - &old_goal,
            Perturbation::ChangeEntireGoal(new_goal)
        );
    }
}
