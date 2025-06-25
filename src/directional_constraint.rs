use ratatui::layout::Constraint;

#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub enum DirectionalConstraint {
    #[default]
    Undefined,
    Vertical(Constraint),
    Horizontal(Constraint),
}
