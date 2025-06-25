use impl_helper::ImplHelper;

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
}
