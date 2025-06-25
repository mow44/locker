use impl_helper::ImplHelper;
use ratatui::layout::{Constraint, Rect};
use std::{cell::RefCell, rc::Rc};

use wrap_context::{arg_context, liab, raw_context};

use crate::{
    column_model::ColumnModel, directional_constraint::DirectionalConstraint,
    types::CursorDirection,
};

fn amount_of_visible_columns(
    columns: &[Rc<RefCell<ColumnModel>>],
    width: u16,
) -> anyhow::Result<usize> {
    let mut columns_counter = 0;
    let mut total_width = 0;

    for column in columns.iter().rev() {
        let column = arg_context!(column.try_borrow())?;
        let constraint = column.constraint();
        match constraint {
            DirectionalConstraint::Horizontal(Constraint::Length(column_width)) => {
                total_width += column_width
            }
            _ => liab!("Wrong type: {:?}", constraint),
        }

        if total_width > width {
            break;
        } else {
            columns_counter += 1;
        }
    }

    anyhow::Ok(columns_counter)
}

/// Because Cassowary is too slow for such a simple task
fn split_area_horizontally(area: &Rect, from_left: u16) -> Option<(Rect, Rect)> {
    if area.width < from_left {
        return None;
    }

    let left = Rect::new(area.x, area.y, from_left, area.height);
    let right = Rect::new(
        area.x + from_left,
        area.y,
        area.width - from_left,
        area.height,
    );

    Some((left, right))
}

#[derive(Default, Debug, Clone, ImplHelper)]
pub struct TableModel {
    #[helper(get, get_mut /* upd is custom */)]
    area: Rect,

    #[helper(all)]
    hide_columns: Vec<Rc<RefCell<ColumnModel>>>,

    #[helper(get)]
    show_columns: Vec<Rc<RefCell<ColumnModel>>>,

    #[helper(all)]
    transparent: bool,

    #[helper(all)]
    constraint: DirectionalConstraint,
}

impl TableModel {
    pub fn area_update(&mut self, value: Rect) -> anyhow::Result<&mut Self> {
        self.area = value;
        arg_context!(self.split_area())?;
        anyhow::Ok(self)
    }

    pub fn split_area(&mut self) -> anyhow::Result<()> {
        let visible_columns_amount = arg_context!(amount_of_visible_columns(
            &self.hide_columns,
            self.area.width
        ))?;

        let columns_len = self.hide_columns.len();

        if visible_columns_amount == 1 {
            if columns_len == 1 {
                self.show_columns = self.hide_columns.clone();
            } else {
                // Without preview-column
                if *raw_context!(raw_context!(self.hide_columns.last())?.try_borrow())?.is_active()
                {
                    self.show_columns = if let Some(lc) = columns_len
                        .checked_sub(1)
                        .and_then(|index| self.hide_columns.get(index))
                    {
                        vec![lc.clone()]
                    } else {
                        vec![]
                    };
                // With preview-column
                } else {
                    self.show_columns = if let Some(pc) = columns_len
                        .checked_sub(2)
                        .and_then(|index| self.hide_columns.get(index))
                    {
                        vec![pc.clone()]
                    } else {
                        vec![]
                    };
                }
            }
        } else {
            // Only visible_columns_amount remains in the vector from the tail
            self.show_columns = self.hide_columns[columns_len - visible_columns_amount..].to_vec();
        }

        let mut splitting_area = self.area;
        let mut areas = vec![];
        for column in self.show_columns.iter() {
            let column = arg_context!(column.try_borrow())?;
            let constraint = column.constraint();
            match constraint {
                DirectionalConstraint::Horizontal(Constraint::Length(column_width)) => {
                    match split_area_horizontally(&splitting_area, column_width.clone()) {
                        Some((left, right)) => {
                            splitting_area = right;
                            areas.push(left);
                        }
                        None => {
                            break;
                        }
                    }
                }
                _ => liab!("Wrong type: {:?}", constraint),
            }
        }

        raw_context!(self.show_columns.iter_mut().zip(areas.iter()).try_for_each(
            |(column, area)| {
                let mut column = arg_context!(column.try_borrow_mut())?;
                column.area_update(*area);
                anyhow::Ok(())
            }
        ))?;

        anyhow::Ok(())
    }

    pub fn update(
        &mut self,
        cursor_direction: &CursorDirection,
        columns_drain_index: Option<usize>,
        highlight_index: Option<usize>,
        new_columns: &[Rc<RefCell<ColumnModel>>],
    ) -> anyhow::Result<()> {
        if let Some(columns_drain_index) = columns_drain_index {
            self.hide_columns.drain(columns_drain_index..);
        }

        let new_columns_len = new_columns.len();

        let new_active_column = match (
            new_columns_len
                .checked_sub(2)
                .and_then(|index| new_columns.get(index)),
            new_columns_len
                .checked_sub(1)
                .and_then(|index| new_columns.get(index)),
        ) {
            (Some(pc), Some(lc)) => {
                let pc = arg_context!(pc.try_borrow())?;
                let lc = arg_context!(lc.try_borrow())?;
                if *pc.is_active() || *lc.is_active() {
                    true
                } else {
                    false
                }
            }
            (None, Some(lc)) => {
                let lc = arg_context!(lc.try_borrow())?;
                if *lc.is_active() {
                    true
                } else {
                    false
                }
            }
            _ => false,
        };

        let old_columns_len = self.hide_columns.len();

        match cursor_direction {
            CursorDirection::Up | CursorDirection::Down => {
                if let Some(column) = old_columns_len
                    .checked_sub(1)
                    .and_then(|index| self.hide_columns.get(index))
                {
                    let mut column = arg_context!(column.try_borrow_mut())?;
                    column
                        .highlight_index_update(highlight_index)
                        .is_active_update(!new_active_column);
                }
            }
            CursorDirection::Right => {
                match (
                    old_columns_len
                        .checked_sub(2)
                        .and_then(|index| self.hide_columns.get(index)),
                    old_columns_len
                        .checked_sub(1)
                        .and_then(|index| self.hide_columns.get(index)),
                ) {
                    (Some(pc), Some(lc)) => {
                        let mut pc = arg_context!(pc.try_borrow_mut())?;
                        let mut lc = arg_context!(lc.try_borrow_mut())?;
                        pc.is_active_update(false);
                        lc.highlight_index_update(highlight_index)
                            .is_active_update(true);
                    }
                    (None, Some(lc)) => {
                        let mut lc = arg_context!(lc.try_borrow_mut())?;
                        lc.is_active_update(true);
                    }
                    _ => liab!("How did you get here?"),
                }
            }
            CursorDirection::Left => {
                if let Some(lc) = old_columns_len
                    .checked_sub(1)
                    .and_then(|index| self.hide_columns.get(index))
                {
                    let mut lc = arg_context!(lc.try_borrow_mut())?;
                    lc.is_active_update(true);
                }
            }
        }

        self.hide_columns.extend(new_columns.to_vec());

        arg_context!(self.split_area())
    }
}
