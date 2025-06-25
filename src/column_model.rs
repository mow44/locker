use impl_helper::ImplHelper;
use ratatui::layout::Rect;
use std::rc::Rc;

use crate::{directional_constraint::DirectionalConstraint, types::Entry};

#[derive(Default, Debug, Clone, PartialEq, Eq, ImplHelper)]
pub struct ColumnModel {
    #[helper(all)]
    area: Rect,

    #[helper(all)]
    highlight_index: Option<usize>,

    #[helper(all)]
    is_active: bool,

    #[helper(all)]
    entries: Vec<Rc<Entry>>,

    #[helper(all)]
    selected_entries: Vec<Rc<Entry>>,

    #[helper(all)]
    transparent: bool,

    #[helper(all)]
    constraint: DirectionalConstraint,
}
