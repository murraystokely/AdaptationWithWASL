use crate::profile::{KnobTable, MeasureTable};
use crate::Goal;
use crate::Tunable;
use std::collections::HashMap;
use std::fmt::Display;
use std::path::Path;
use std::rc::Rc;

pub struct Configurations<T> {
    pub(crate) instance_id: usize,
    pub(crate) measure_table: MeasureTable,
    pub(crate) knob_table: KnobTable,
    pub(crate) knobs: HashMap<String, Rc<dyn Tunable<T>>>,
    pub(crate) goal: Goal,
    pub(crate) window_size: u64,
}

impl<T: Copy> Configurations<T> {
    pub fn new<P>(
        instance_id: usize,
        mt_path: P,
        kt_path: P,
        knobs: Vec<Rc<dyn Tunable<T>>>,
        goal: Goal,
        window_size: u64,
    ) -> Configurations<T>
    where
        P: AsRef<Path> + Display,
    {
        let measure_table = MeasureTable::new(mt_path);
        let knob_table = KnobTable::new(kt_path);
        let knobs = knobs.into_iter().map(|k| (k.name(), k)).collect();

        Configurations {
            instance_id,
            measure_table,
            knob_table,
            knobs,
            goal,
            window_size,
        }
    }
}
