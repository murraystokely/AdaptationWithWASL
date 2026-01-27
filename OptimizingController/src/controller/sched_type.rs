use std::fmt::Display;

#[derive(Debug)]
pub enum SchedType {
    RLMultiConf,
    RLSingleConf,
    ControlMultiConf,
}

impl SchedType {
    pub fn new(tag: u64) -> SchedType {
        if std::env::var(format!("LEARNING_BASED_{}", tag)).is_err() {
            return SchedType::ControlMultiConf;
        }

        match std::env::var(format!("CONF_TYPE_{}", tag))
            .as_ref()
            .map(|s| s.as_str())
        {
            Ok("multi") => SchedType::RLMultiConf,
            Ok("single") => SchedType::RLSingleConf,
            Ok(_) => panic!("Incorrect sched type"),
            Err(_) => panic!("LEARNING_BASED specified without CONF_TYPE"),
        }
    }
}

impl Display for SchedType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            SchedType::RLMultiConf => write!(f, "RLMutliConf"),
            SchedType::RLSingleConf => write!(f, "RLSingleConf"),
            SchedType::ControlMultiConf => write!(f, "ControlMultiConf"),
        }
    }
}
