#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptimizationType {
    Minimize,
    Maximize,
}

impl std::fmt::Display for OptimizationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match &self {
            OptimizationType::Maximize => write!(f, "maximize"),
            OptimizationType::Minimize => write!(f, "minimize"),
        }
    }
}
