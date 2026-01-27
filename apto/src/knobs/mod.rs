use std::cell::{Ref, RefCell};

mod cache_x86;
mod homogenous;
mod utilities;

pub use homogenous::{
    AvailablePhysicalCores, AvailablePhysicalThreads, CoreFrequency, Hyperthreading,
    UncoreFrequency,
};

pub use cache_x86::CacheCOS;

pub struct BorrowedValues<'a, T>
where
    T: Copy,
{
    borrowed_knob: Ref<'a, GenericKnob<T>>,
}

impl<T> std::ops::Deref for BorrowedValues<'_, T>
where
    T: Copy,
{
    type Target = [T];

    fn deref(&self) -> &'_ Self::Target {
        &self.borrowed_knob.permitted_values
    }
}

pub trait Tunable<T>
where
    T: Copy,
{
    fn get(&self) -> T;
    fn set(&self, val: T);
    fn name(&self) -> String;
    fn possible_values(&'_ self) -> BorrowedValues<'_, T>;
}

struct GenericKnob<T: Copy> {
    name: String,
    permitted_values: Vec<T>,
    current_value: T,
}

impl<T: Copy> GenericKnob<T> {
    fn new(name: String, permitted_values: Vec<T>, current_value: T) -> GenericKnob<T> {
        GenericKnob {
            name,
            permitted_values,
            current_value,
        }
    }
}

pub struct ApplicationKnob<T: Copy> {
    knob: RefCell<GenericKnob<T>>,
    application_func: Option<Box<dyn Fn(Option<T>, T)>>,
}

impl<T> ApplicationKnob<T>
where
    T: Copy + Eq,
{
    pub fn new(
        name: String,
        values: Vec<T>,
        initial_value: T,
        application_func: Option<Box<dyn Fn(Option<T>, T)>>,
    ) -> ApplicationKnob<T> {
        let knob = RefCell::new(GenericKnob::new(name, values, initial_value));
        let app_knob = ApplicationKnob {
            knob,
            application_func,
        };
        if let Some(func) = app_knob.application_func.as_ref() {
            func(None, initial_value);
        }
        app_knob
    }

    pub fn possible_values(&self) -> BorrowedValues<'_, T> {
        BorrowedValues {
            borrowed_knob: self.knob.borrow(),
        }
    }

    pub fn apply(&self, val: T) {
        if let Some(func) = self.application_func.as_ref() {
            func(Some(self.get()), val);
        }
        self.knob.borrow_mut().current_value = val;
    }
}

impl<T> Tunable<T> for ApplicationKnob<T>
where
    T: Copy + Eq,
{
    fn get(&self) -> T {
        self.knob.borrow().current_value
    }

    fn set(&self, val: T) {
        self.apply(val);
    }

    fn name(&self) -> String {
        self.knob.borrow().name.clone()
    }

    fn possible_values(&self) -> BorrowedValues<'_, T> {
        BorrowedValues {
            borrowed_knob: self.knob.borrow(),
        }
    }
}

pub struct ConstantKnob<T: Copy> {
    knob: RefCell<GenericKnob<T>>,
}

impl<T> ConstantKnob<T>
where
    T: Copy,
{
    pub fn new(name: String, value: T) -> ConstantKnob<T> {
        let knob = RefCell::new(GenericKnob::new(name, vec![value], value));
        ConstantKnob { knob }
    }
}

impl<T> Tunable<T> for ConstantKnob<T>
where
    T: Copy + Eq + std::fmt::Display,
{
    fn get(&self) -> T {
        self.knob.borrow().current_value
    }

    fn set(&self, val: T) {
        if val != self.get() {
            panic!(
                "Tried to change value of Constant Knob {}: {} -> {}",
                self.name(),
                self.get(),
                val
            );
        }
    }

    fn name(&self) -> String {
        self.knob.borrow().name.clone()
    }

    fn possible_values(&self) -> BorrowedValues<'_, T> {
        BorrowedValues {
            borrowed_knob: self.knob.borrow(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ApplicationKnob, ConstantKnob, Tunable};

    #[test]
    fn test_app_knob() {
        let knob: ApplicationKnob<u64> =
            ApplicationKnob::new("dummy".to_string(), vec![1, 2, 3], 1, None);
        assert_eq!(*knob.possible_values(), vec![1, 2, 3]);
        assert_eq!(knob.get(), 1);
        knob.set(3);
        assert_eq!(knob.get(), 3);
        knob.set(100);
        assert_eq!(knob.get(), 100);
    }

    #[test]
    fn test_tunable_names() {
        let knob: ApplicationKnob<u64> =
            ApplicationKnob::new("dummy".to_string(), vec![1, 2, 3], 1, None);
        assert_eq!(knob.name(), "dummy");
    }

    #[test]
    #[should_panic]
    fn test_constant_knob_invalid_set() {
        let knob: ConstantKnob<u64> = ConstantKnob::new("dummy".to_string(), 10);
        knob.set(11);
    }

    #[test]
    fn test_constant_knob() {
        let knob: ConstantKnob<u64> = ConstantKnob::new("dummy".to_string(), 10);
        assert_eq!(knob.name(), "dummy");
        assert_eq!(knob.get(), 10);
        assert_eq!(*knob.possible_values(), vec![10u64]);
    }
}
