use impl_helper::ImplHelper;
use wrap_context::arg_context;

use crate::utils::Location;

#[derive(Debug, Default, Clone, Copy, PartialEq, ImplHelper)]
pub struct Paginator {
    #[helper(all)]
    size: usize,

    #[helper(all)]
    start: usize,

    #[helper(all)]
    total: Option<usize>,
}

impl Paginator {
    pub fn new(size: usize, start: usize, total: Option<usize>) -> Self {
        Self { size, start, total }
    }

    pub fn page_location(&self, target: usize, total: usize) -> anyhow::Result<Location> {
        let start = (target / self.size()) * self.size();
        let finish = (arg_context!(start.checked_add(*self.size()))?)
            .min(total)
            .saturating_sub(1); // saturating_sub to use ..=finish instead of ..finish

        anyhow::Ok(Location::new(start, finish))
    }
}
