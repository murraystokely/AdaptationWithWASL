pub mod controller;
mod kalman_filter;
mod optimization_expression;
mod optimization_type;

pub(crate) use kalman_filter::KalmanFilter;
pub(crate) use optimization_expression::ExpressionType;
pub(crate) use optimization_expression::ObjectiveFunction;

pub use controller::OptimizingController as Controller;
pub use optimization_type::OptimizationType;

use controller::Log;

use std::time::Instant;

fn transpose<T: Copy>(table: &[Vec<T>]) -> Vec<Vec<T>> {
    (0..table[0].len())
        .map(|col| (0..table.len()).map(|row| table[row][col]).collect())
        .collect()
}

fn read_logs(path: &str) -> Vec<Log> {
    let content = std::fs::read_to_string(path).unwrap();
    content.lines().map(|line| line.parse().unwrap()).collect()
}

fn read_profile(path: &str, constraint_name: &str) -> (Vec<String>, Vec<Vec<f64>>) {
    let file_content = std::fs::read_to_string(path).unwrap();
    let content = file_content.trim();
    let table_string: Vec<Vec<&str>> = content
        .lines()
        .map(|line| line.split(',').collect::<Vec<&str>>())
        .collect();
    let mut table_string_transposed = transpose(&table_string);
    table_string_transposed.sort_by_key(|e| e[0]);
    let table_string = transpose(&table_string_transposed);
    let header: Vec<String> = table_string[0]
        .iter()
        .map(|name| String::from(*name))
        .collect();
    let mut profile: Vec<Vec<f64>> = table_string[1..]
        .iter()
        .map(|row| row.iter().map(|e| e.parse().unwrap()).collect())
        .collect();

    let constraint_idx = header.binary_search(&constraint_name.to_string()).unwrap();
    profile.sort_by(|e0, e1| e0[constraint_idx].partial_cmp(&e1[constraint_idx]).unwrap());

    (header, profile)
}

fn filter_model(
    obj_func: &str,
    headers: &[String],
    model: &[Vec<f64>],
) -> (Vec<String>, Vec<Vec<f64>>) {
    // This is pretty bad!
    let mut measures = obj_func
        .trim()
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace() || *c == '_')
        .collect::<String>()
        .split(' ')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect::<Vec<String>>();
    measures.sort();

    let indices: Vec<usize> = measures
        .iter()
        .map(|m| headers.binary_search(m).unwrap())
        .collect();
    let filtered_model: Vec<Vec<f64>> = model
        .iter()
        .map(|config| indices.iter().map(|idx| config[*idx]).collect())
        .collect();

    (measures, filtered_model)
}

fn replay(controller: &mut Controller, logs: &[Log]) {
    for log in logs {
        let now = Instant::now();
        let _ = controller.compute_schedule(log.measured_constraint, 1.0);
        let time_elapsed = now.elapsed().as_micros();
        println!("Time elapsed: {}", time_elapsed);
    }
}

pub fn simulate(
    profile_path: &str,
    history_path: &str,
    constraint_name: &str,
    constraint_target: f64,
    opt_type: OptimizationType,
    objective_function: &str,
    window: usize,
) {
    let (headers, measuretable) = read_profile(profile_path, &constraint_name);
    let (obj_measures, filtered_model) = filter_model(objective_function, &headers, &measuretable);
    // measuretable should be sorted according to header names
    let constrained_measure_idx = headers.binary_search(&constraint_name.to_string()).unwrap();
    let original_logs = read_logs(history_path);

    let mut controller = Controller::new(
        0,
        measuretable,
        filtered_model,
        constraint_target,
        constrained_measure_idx,
        window,
        opt_type,
        objective_function,
        obj_measures,
        0,
    );

    replay(&mut controller, &original_logs);
    println!("Printing Logs");
    controller.flush_logs();
}

#[cfg(test)]
mod tests {
    use super::transpose;
    #[test]
    fn test_transpose() {
        let original = vec![
            vec!["a", "b", "c"],
            vec!["d", "e", "f"],
            vec!["g", "h", "i"],
        ];
        assert_eq!(
            transpose(&original),
            vec![
                vec!["a", "d", "g"],
                vec!["b", "e", "h"],
                vec!["c", "f", "i"]
            ]
        );
        let original = vec![
            vec![&1, &2, &3],
            vec![&4, &5, &6],
            vec![&7, &8, &9],
            vec![&10, &11, &12],
        ];
        assert_eq!(
            transpose(&original),
            vec![
                vec![&1, &4, &7, &10],
                vec![&2, &5, &8, &11],
                vec![&3, &6, &9, &12]
            ]
        );
    }

    use super::read_profile;
    use std::io::Write;
    #[test]
    fn test_read_profile() {
        let measuretable_string = "i,a,c,d\n1,2,3,4\n5,6,7,8\n";
        let mut file = std::fs::File::create("/tmp/test_read_profile").unwrap();
        let _ = file.write_all(measuretable_string.as_bytes());
        let (headers, profile) = read_profile("/tmp/test_read_profile", "i");
        assert_eq!(
            headers,
            vec![
                "a".to_string(),
                "c".to_string(),
                "d".to_string(),
                "i".to_string()
            ]
        );
        assert_eq!(
            profile,
            vec![vec![2.0, 3.0, 4.0, 1.0], vec![6.0, 7.0, 8.0, 5.0]]
        );
        let _ = std::fs::remove_file("/tmp/test_read_profile");
    }
}
