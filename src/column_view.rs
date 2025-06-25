use itertools::Itertools;
use ratatui::{
    layout::Rect,
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Clear, List, ListItem, ListState},
    Frame,
};

use crate::{column_model::ColumnModel, render::Render, utils::clip_string};

#[derive(Debug, Clone, Default)]
pub struct ColumnView<'a> {
    area: Rect,
    list: List<'a>,
    state: ListState,
    transparent: bool,
}

impl<'a> From<&ColumnModel> for ColumnView<'a> {
    fn from(model: &ColumnModel) -> Self {
        let area = model.area().clone();

        let list = List::new(
            model
                .entries()
                .iter()
                .map(|entry| {
                    let style;

                    if model.selected_entries().contains(entry) {
                        style = Style::new().on_red();
                    } else {
                        style = Style::default();
                    }

                    ListItem::new(Line::from(vec![Span::from(clip_string(
                        entry.name().clone(),
                        "…",
                        area.width.into(),
                    ))]))
                    .style(style)
                })
                .collect_vec(),
        )
        .highlight_style(if *model.is_active() {
            Style::default().bg(Color::Rgb(214, 94, 14))
        } else {
            Style::default().bg(Color::Rgb(80, 73, 69))
        });

        let state = ListState::default().with_selected(model.highlight_index().clone());

        let transparent = model.transparent().clone();

        Self {
            area,
            list,
            state,
            transparent,
        }
    }
}

impl<'a> Render for ColumnView<'a> {
    fn render(&mut self, frame: &mut Frame) {
        if !self.transparent {
            frame.render_widget(Clear, self.area);
        }

        frame.render_stateful_widget(&self.list, self.area, &mut self.state);
    }
}
