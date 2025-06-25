use itertools::Itertools;
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::Clear,
};

use crate::{render::Render, textline_model::TextLineModel};

#[derive(Debug, Default)]
pub struct TextLineView<'a> {
    area: Rect,
    line: Line<'a>,
    transparent: bool,
}

impl<'a> From<&TextLineModel> for TextLineView<'a> {
    fn from(model: &TextLineModel) -> Self {
        let area = model.area().clone();

        let line = Line::from(
            model
                .show_spans()
                .iter()
                .map(|(content, style)| Span::styled(content.clone(), style.clone()))
                .collect_vec(),
        )
        .style(*model.style())
        .alignment(*model.alignment());

        let transparent = model.transparent().clone();

        Self {
            area,
            line,
            transparent,
        }
    }
}

impl<'a> Render for TextLineView<'a> {
    fn render(&mut self, frame: &mut ratatui::Frame) {
        if !self.transparent {
            frame.render_widget(Clear, self.area);
        }

        frame.render_widget(&self.line, self.area);
    }
}
