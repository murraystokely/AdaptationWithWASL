use anyhow::{anyhow, Result};
use std::env;
use OptimizingController::simulate;
use OptimizingController::OptimizationType;

fn main() -> Result<()> {
    let mut args = env::args();
    args.next();
    let (profile_path, history_path) = match (args.next(), args.next()) {
        (Some(profile_path), Some(history_path)) => (profile_path, history_path),
        _ => return Err(anyhow!("Profile path and/or history path not found!")),
    };
    let (constraint_name, constraint_target) = match (args.next(), args.next()) {
        (Some(name), Some(goal)) => (name, goal.parse::<f64>()?),
        _ => {
            return Err(anyhow!(
                "Constraint name and/or goal not found or malformed!",
            ))
        }
    };

    let window = match args.next() {
        Some(w) => w.parse::<usize>()?,
        None => return Err(anyhow!("Missing window size")),
    };

    let opt_type = match args.next() {
        Some(opt_type) => match &opt_type[..] {
            "min" => OptimizationType::Minimize,
            "max" => OptimizationType::Maximize,
            _ => return Err(anyhow!("Incorrect optimization type")),
        },
        None => return Err(anyhow!("Missing optimization type")),
    };

    let objective_function = args.collect::<Vec<String>>().join(" ");
    if objective_function.is_empty() {
        return Err(anyhow!("Missing objective function"));
    }

    env_logger::init();

    simulate(
        profile_path.as_str(),
        history_path.as_str(),
        constraint_name.as_str(),
        constraint_target,
        opt_type,
        objective_function.as_str(),
        window,
    );

    Ok(())
}
