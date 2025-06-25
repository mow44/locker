use impl_helper::ImplHelper;
use ratatui::layout::{Constraint, Layout, Rect};

use wrap_context::{arg_context, liab, raw_context};

use crate::{directional_constraint::DirectionalConstraint, table_model::TableModel};

#[derive(Default, Debug, Clone, ImplHelper)]
pub struct PageModel {
    #[helper(get, get_mut /* set and upd are custom */)]
    area: Rect,

    #[helper(all)]
    left_table: TableModel,

    #[helper(all)]
    rght_table: TableModel,

    #[helper(all)]
    transparent: bool,

    #[helper(all)]
    constraint: DirectionalConstraint,
}

impl PageModel {
    pub fn split_area(&mut self) -> anyhow::Result<()> {
        let [left_table_area, rght_table_area] = Layout::horizontal({
            let constraints = [self.left_table.constraint(), self.rght_table.constraint()];
            match constraints {
                [DirectionalConstraint::Horizontal(left_table_constraint), DirectionalConstraint::Horizontal(rght_table_constraint)] => [left_table_constraint, rght_table_constraint],
                _ => liab!("Wrong type: {:?}", constraints),
            }
        })
        .areas(self.area);

        arg_context!(self.left_table.area_update(left_table_area))?;
        arg_context!(self.rght_table.area_update(rght_table_area))?;

        anyhow::Ok(())
    }

    #[must_use = "method moves the value of self and returns the modified value"]
    pub fn area_set(mut self, value: Rect) -> anyhow::Result<Self> {
        self.area = value;
        arg_context!(self.split_area())?;
        anyhow::Ok(self)
    }

    pub fn area_update(&mut self, value: Rect) -> anyhow::Result<&mut Self> {
        self.area = value;
        arg_context!(self.split_area())?;
        anyhow::Ok(self)
    }

    pub fn set_left_table_width(&mut self, value: u16) -> anyhow::Result<()> {
        self.left_table_mut()
            .constraint_update(DirectionalConstraint::Horizontal(Constraint::Min(value)));

        raw_context!(self
            .left_table_mut()
            .hide_columns_mut()
            .iter_mut()
            .try_for_each(|column| {
                let mut column = arg_context!(column.try_borrow_mut())?;

                column.area_update(Rect::default());
                column.constraint_update(DirectionalConstraint::Horizontal(Constraint::Length(
                    value,
                )));

                anyhow::Ok(())
            }))?;

        arg_context!(self.split_area())?;

        anyhow::Ok(())
    }

    pub fn set_rght_table_width(&mut self, value: u16) -> anyhow::Result<()> {
        self.rght_table_mut()
            .constraint_update(DirectionalConstraint::Horizontal(Constraint::Max(value)));

        raw_context!(self
            .rght_table_mut()
            .hide_columns_mut()
            .iter_mut()
            .try_for_each(|column| {
                let mut column = arg_context!(column.try_borrow_mut())?;

                column.area_update(Rect::default());
                column.constraint_update(DirectionalConstraint::Horizontal(Constraint::Length(
                    value,
                )));

                anyhow::Ok(())
            }))?;

        arg_context!(self.split_area())?;

        anyhow::Ok(())
    }
}
