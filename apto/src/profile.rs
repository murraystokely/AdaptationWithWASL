use crate::knobs::Tunable;
use std::collections::HashMap;
use std::fmt::Display;
use std::path::Path;
use std::rc::Rc;
use std::str::FromStr;

fn read_table<P, T>(path: P) -> (Vec<String>, Vec<Vec<T>>)
where
    P: AsRef<Path> + Display,
    T: FromStr,
{
    let file_content =
        std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("Could not read {}", &path));
    let mut lines = file_content.trim().lines();
    let header: Vec<String> = lines
        .next()
        .unwrap()
        .trim()
        .split(',')
        .map(String::from)
        .collect();
    let remainder: Vec<Vec<T>> = lines
        .map(|line| {
            line.trim()
                .split(',')
                .map(|s| {
                    s.parse()
                        .unwrap_or_else(|_| panic!("Could not parse {}.", path))
                })
                .collect()
        })
        .collect();

    (header, remainder)
}

#[derive(Clone)]
pub struct MeasureTable {
    pub names: Vec<String>,
    pub profile: Vec<Vec<f64>>,
}

impl MeasureTable {
    pub fn new<P>(path: P) -> MeasureTable
    where
        P: AsRef<Path> + Display,
    {
        let (names, profile) = read_table(path);
        MeasureTable { names, profile }
    }

    pub fn constraint_idx(&self, constraint: &str) -> usize {
        self.names
            .iter()
            .position(|name| name == constraint)
            .expect("Constraint measure not in measure table.")
    }
}

#[derive(Clone)]
pub struct KnobTable {
    pub names: Vec<String>,
    pub configurations: Vec<HashMap<String, u64>>,
}

impl KnobTable {
    pub fn new<P>(path: P) -> KnobTable
    where
        P: AsRef<Path> + Display,
    {
        let (names, values) = read_table(path);
        let configurations = values
            .into_iter()
            .map(|config| names.iter().cloned().zip(config.into_iter()).collect())
            .collect();

        KnobTable {
            names,
            configurations,
        }
    }
}

#[derive(Clone)]
pub struct ActiveModel {
    pub configs: Vec<(Vec<f64>, HashMap<String, u64>)>,
}

impl ActiveModel {
    pub fn new(measure_table: &MeasureTable, knob_table: &KnobTable) -> ActiveModel {
        let configs = measure_table
            .profile
            .iter()
            .zip(knob_table.configurations.iter())
            .map(|(mt_entry, kt_entry)| (mt_entry.clone(), kt_entry.clone()))
            .collect();
        ActiveModel { configs }
    }

    pub fn find_id(&self, knobs: &HashMap<String, Rc<dyn Tunable<u64>>>) -> Option<usize> {
        
        for (idx, (_, hmap)) in self.configs.iter().enumerate() {
            println!("Checking hmap: {:?}", hmap.keys().collect::<Vec<_>>());
            println!("Knobs available: {:?}", knobs.keys().collect::<Vec<_>>());
            let mut complete_match = true;
            for (k, &v) in hmap.iter() {
                if k == "id" {
                    continue;
                }
                if v != knobs
                    .get(k)
                    .unwrap_or_else(|| panic!("Could not find {} in knobs", k))
                    .get()
                {
                    complete_match = false;
                    break;
                }
            }
            if complete_match {
                return Some(idx);
            }
        }

        None
    }

    pub fn restrict_model(&mut self, knobs: &HashMap<String, Rc<dyn Tunable<u64>>>) -> usize {
        let original_length = self.configs.len();
        let configs = std::mem::take(&mut self.configs);
        self.configs = configs
            .into_iter()
            .filter_map(|(measures, settings)| {
                for (name, value) in settings.iter().filter(|(name, _)| *name != "id") {
                    if let Some(tunable) = knobs.get(name) {
                        if !tunable.possible_values().contains(value) {
                            return None;
                        }
                    } else {
                        println!("Missing key in knobs: '{}'", name);
                        return None;  // Skip this config if the key doesn't exist
                    }
                    
                }
                Some((measures, settings))
            })
            .collect();
        original_length - self.configs.len()
    }
    

    pub fn measure_values(&self) -> Vec<Vec<f64>> {
        self.configs.iter().map(|(m, _)| m.clone()).collect()
    }

    pub fn get_knob_settings(&self, idx: usize) -> &HashMap<String, u64> {
        &self.configs[idx].1
    }

    pub fn sort_by_constraint(&mut self, idx: usize) {
        self.configs
            .sort_by(|e0, e1| e0.0[idx].partial_cmp(&e1.0[idx]).unwrap())
    }

    pub fn cost_model(&self, indices: &[usize]) -> Vec<Vec<f64>> {
        self.configs
            .iter()
            .map(|(m, _)| m)
            .map(|line| indices.iter().map(|&idx| line[idx]).collect())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::{ActiveModel, KnobTable, MeasureTable};
    use lazy_static::lazy_static;
    use regex::Regex;
    use std::collections::HashMap;
    use std::io::Write;

    lazy_static! {
        static ref NAME_REGEX: Regex = Regex::new("[[:alpha:]]+[a-zA-Z0-9_]*").unwrap();
    }

    static MEASURE_TABLE_STRING: &str = r#"id,currentConfiguration,energy,energyDelta,iteration,latency,operations,performance,powerConsumption,quality,runningTime,systemEnergy,time,windowSize
0,-1.0,66037923.38,660274.235,99.5,0.21323804974556,50000.0,4.68959456904254,3096418.46653472,1.0,21.3261238873005,66876406.32,1554836505.73334,20.0
1,-1.0,261643351.675,2616127.165,99.5,0.845102413892746,200000.0,1.18328853824208,3095633.28892824,1.0,84.5128715789318,395299728.425,1554836611.82094,20.0
2,-1.0,17062789.245,170605.0,99.5,0.0550970566272736,12500.0,18.1497898656349,3096444.90002664,0.25,5.50908312678337,676698295.14,1554836702.72642,20.0
3,-1.0,65982635.43,659723.43,99.5,0.213206874132156,50000.0,4.69028029265206,3094287.80232982,0.25,21.3219397556782,760037632.135,1554836729.65205,20.0
"#;
    static KNOB_TABLE_STRING: &str = r#"id,step,threshold
0,1,50000
1,1,200000
2,4,50000
3,4,200000"#;

    #[test]
    fn read_measure_table() {
        let mut file =
            std::fs::File::create("/tmp/measuretable").expect("Could not create test measuretable");
        let _ = file
            .write_all(b"id,quality,performance,energy\n0,1,2,3    \n4,5,6,7    \n")
            .expect("Could not write measure table.");
        let table = MeasureTable::new("/tmp/measuretable");
        assert_eq!(table.names, vec!["id", "quality", "performance", "energy"]);
        assert_eq!(
            table.profile,
            vec![vec![0.0, 1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0, 7.0]]
        );
        std::fs::remove_file("/tmp/measuretable").expect("Could not clean test measuretable.");
    }

    #[test]
    fn read_knob_table() {
        let mut file =
            std::fs::File::create("/tmp/knobtable").expect("Could not create test knobtable");
        let _ = file
            .write_all(b"id,me,qp,subme\n   0,1,2,3\n\n")
            .expect("Could not write measure table.");

        let mut correct = HashMap::new();
        for (name, value) in [("id", 0), ("me", 1), ("qp", 2), ("subme", 3)] {
            correct.insert(String::from(name), value);
        }

        let table = KnobTable::new("/tmp/knobtable");
        assert_eq!(table.names, vec!["id", "me", "qp", "subme"]);
        assert_eq!(table.configurations[0].len(), correct.len());
        for (k, v) in table.configurations[0].iter() {
            assert_eq!(correct.get(k).unwrap(), v);
        }

        std::fs::remove_file("/tmp/knobtable").expect("Could not clean test knobtable.");
    }

    #[test]
    fn regex_text() {
        let regex = regex::Regex::new("[[:alpha:]]+[a-zA-Z0-9_]*").unwrap();
        let matches = regex
            .find_iter("performance / powerConsumption")
            .map(|m| m.as_str())
            .collect::<Vec<&str>>();
        assert_eq!(matches, vec!["performance", "powerConsumption"]);
        let matches = regex
            .find_iter("iteration * time")
            .map(|m| m.as_str())
            .collect::<Vec<&str>>();
        assert_eq!(matches, vec!["iteration", "time"]);
    }

    #[test]
    fn get_active_model_measures() {
        let _ = std::fs::File::create("/tmp/amt")
            .expect("Could not create test file for get_active_measure_table")
            .write_all(MEASURE_TABLE_STRING.as_bytes());
        let _ = std::fs::File::create("/tmp/kamt")
            .expect("Could not create test file for get_active_measure_table")
            .write_all(KNOB_TABLE_STRING.as_bytes());
        let measure_table = MeasureTable::new("/tmp/amt");
        let knob_table = KnobTable::new("/tmp/kamt");

        let mut active_model = ActiveModel::new(&measure_table, &knob_table);
        active_model.sort_by_constraint(measure_table.constraint_idx("quality"));

        assert_eq!(active_model.configs[0].0[0] as u64, 2);
        assert_eq!(active_model.configs[1].0[0] as u64, 3);
        assert_eq!(active_model.configs[2].0[0] as u64, 0);
        assert_eq!(active_model.configs[3].0[0] as u64, 1);
    }

    #[test]
    fn active_model_constraint_cost_models() {
        let _ = std::fs::File::create("/tmp/active_model_mt")
            .expect("Could not create test file for active_model")
            .write_all(MEASURE_TABLE_STRING.as_bytes());
        let _ = std::fs::File::create("/tmp/active_model_kt")
            .expect("Could not create test file for active_model")
            .write_all(KNOB_TABLE_STRING.as_bytes());

        let measure_table = MeasureTable::new("/tmp/active_model_mt");
        let knob_table = KnobTable::new("/tmp/active_model_kt");
        let mut active_model = ActiveModel::new(&measure_table, &knob_table);

        let constraint_idx = measure_table.constraint_idx("performance");

        let mut sorted_model: Vec<Vec<f64>> = MEASURE_TABLE_STRING
            .trim()
            .lines()
            .skip(1)
            .map(|line| {
                line.trim()
                    .split(',')
                    .map(|v| v.parse::<f64>().unwrap())
                    .collect()
            })
            .collect();
        sorted_model.sort_by(|e0, e1| e0[constraint_idx].partial_cmp(&e1[constraint_idx]).unwrap());

        let correct_order: Vec<f64> = sorted_model
            .iter()
            .map(|line| line[constraint_idx])
            .collect();

        active_model.sort_by_constraint(constraint_idx);
        let constraint_values: Vec<f64> = active_model
            .measure_values()
            .iter()
            .map(|line| line[constraint_idx])
            .collect();
        assert_eq!(correct_order, constraint_values);

        let obj_func = "performance / energy";
        let obj_measures: Vec<String> = NAME_REGEX
            .find_iter(obj_func)
            .map(|f| String::from(f.as_str()))
            .collect();
        let obj_measure_indices: Vec<usize> = obj_measures
            .iter()
            .map(|needle| {
                measure_table
                    .names
                    .iter()
                    .position(|haystack| needle == haystack)
                    .unwrap_or_else(|| {
                        panic!(
                            "Measure ({}) not found in measure table header",
                            needle.as_str()
                        )
                    })
            })
            .collect();

        let correct_cost_model: Vec<Vec<f64>> = sorted_model
            .iter()
            .map(|line| obj_measure_indices.iter().map(|&idx| line[idx]).collect())
            .collect();

        assert_eq!(
            correct_cost_model,
            active_model.cost_model(&obj_measure_indices)
        );
    }
}