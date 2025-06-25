use ratatui::{layout::Rect, widgets::Clear, Frame};
use wrap_context::arg_context;

use crate::{column_view::ColumnView, render::Render, table_model::TableModel};

#[derive(Debug, Clone, Default)]
pub struct TableView<'a> {
    area: Rect,
    columns: Vec<ColumnView<'a>>,
    transparent: bool,
}

impl<'a> TryFrom<&TableModel> for TableView<'a> {
    type Error = anyhow::Error;

    fn try_from(model: &TableModel) -> anyhow::Result<Self> {
        let area = model.area().clone();

        let columns = model
            .show_columns()
            .iter()
            .map(|column| {
                let column = arg_context!(column.try_borrow())?;
                anyhow::Ok(ColumnView::from(&*column))
            })
            .collect::<anyhow::Result<Vec<ColumnView>>>()?;

        let transparent = model.transparent().clone();

        anyhow::Ok(Self {
            area,
            columns,
            transparent,
        })
    }
}

impl<'a> Render for TableView<'a> {
    fn render(&mut self, frame: &mut Frame) {
        if !self.transparent {
            frame.render_widget(Clear, self.area);
        }

        for column in self.columns.iter_mut() {
            column.render(frame);
        }
    }
}
