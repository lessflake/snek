use crate::parse::Encounter;

pub trait Filter {
    fn filter(&self, log: &Encounter) -> bool;
}

pub struct Length {}
impl Filter for Length {
    fn filter(&self, log: &Encounter) -> bool {
        // only upload logs that got past first phase (1st "phase" in this
        // `log.phases` is overall, second is first phase)
        // or if it's over 5 seconds long
        log.phases.len() > 2 || log.phases[0].duration() > 5000
    }
}

pub struct Unsuccessful {}
impl Filter for Unsuccessful {
    fn filter(&self, log: &Encounter) -> bool {
        !log.success
    }
}
