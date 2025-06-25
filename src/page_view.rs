use ratatui::{layout::Rect, widgets::Clear, Frame};
use wrap_context::arg_context;

use crate::{page_model::PageModel, render::Render, table_view::TableView};

#[derive(Debug, Clone, Default)]
pub struct PageView<'a> {
    area: Rect,
    left_table: TableView<'a>,
    rght_table: TableView<'a>,
    transparent: bool,
}

impl<'a> TryFrom<&PageModel> for PageView<'a> {
    type Error = anyhow::Error;

    fn try_from(model: &PageModel) -> anyhow::Result<Self> {
        let area = model.area().clone();

        let left_table = arg_context!(TableView::try_from(model.left_table()))?;
        let rght_table = arg_context!(TableView::try_from(model.rght_table()))?;

        let transparent = model.transparent().clone();

        anyhow::Ok(Self {
            area,
            left_table,
            rght_table,
            transparent,
        })
    }
}

impl<'a> Render for PageView<'a> {
    fn render(&mut self, frame: &mut Frame) {
        if !self.transparent {
            frame.render_widget(Clear, self.area);
        }

        self.left_table.render(frame);
        self.rght_table.render(frame);
    }
}
