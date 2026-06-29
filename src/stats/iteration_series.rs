/// Per-generation run statistics. Length is `iterations_run + 1`: index 0 is
/// generation 0, `alive[0]` is the initial live-cell count, and
/// `births[0] == deaths[0] == 0`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IterationSeries {
    pub alive: Vec<u64>,
    pub births: Vec<u64>,
    pub deaths: Vec<u64>,
}

impl IterationSeries {
    pub fn len(&self) -> usize {
        self.alive.len()
    }

    pub fn is_empty(&self) -> bool {
        self.alive.is_empty()
    }
}
