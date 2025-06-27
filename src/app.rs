use itertools::Itertools;
use ratatui::{
    backend::Backend,
    layout::{Alignment, Constraint, Flex, Layout, Rect, Size},
    style::{Color, Style, Stylize},
    widgets::Clear,
    Frame,
};
use serde_json::value::RawValue;
use std::{cell::RefCell, path::PathBuf, rc::Rc};
use wrap_context::{arg_context, liab, raw_context};

use crate::{
    column_model::*, directional_constraint::*, event::*, handler::*, node::*, page_model::*,
    page_view::*, paginator::*, preferences::*, render::*, table_model::*, textline_model::*,
    textline_view::*, tui::*, types::*, utils::*,
};

#[derive(Debug)]
pub struct App<'a> {
    running: bool,
    root: Node<'a>,
    path: Path,
    terminal_size: Size,
    preferences: Preferences,

    page: ViewModel<PageView<'a>, PageModel>,
    bottom_textline: ViewModel<TextLineView<'a>, TextLineModel>,
    status_textline: ViewModel<TextLineView<'a>, TextLineModel>,
    flpath_textline: ViewModel<TextLineView<'a>, TextLineModel>,
}

pub fn nodes_in_path_to_columns(
    root: &Node,
    path: &[Step],
    node_index_offset: usize,
    selected_entries: &Vec<Rc<Entry>>,
    column_width: &u16,
) -> anyhow::Result<Vec<Rc<RefCell<ColumnModel>>>> {
    if path.is_empty() {
        liab!("Path could not be empty");
    }

    let mut current_node = root;
    let mut nodes_in_path = vec![];

    if node_index_offset == 0 {
        nodes_in_path.push(current_node);
    }

    for (i, step) in path.iter().enumerate() {
        let shifted_step = arg_context!(step.checked_sub(*current_node.paginator().start()))?;

        if let Some(next_node) = current_node.children().get(shifted_step) {
            current_node = next_node;

            // i + 1 because 'nodes_in_path.push(root)' is called above
            if i + 1 >= node_index_offset {
                nodes_in_path.push(current_node);
            }
        } else {
            liab!(
                "In the path: {:?}, the node at step: {:?} (index: {:?}) does not exist",
                path,
                step,
                i
            );
        }
    }

    let mut columns = vec![];

    for (i, node) in nodes_in_path.iter().enumerate() {
        // Because there are no children, there's no need to create column ('entries' will be an empty vec![])
        if !node.children().is_empty() {
            columns.push({
                let highlight_index = if let Some(step) = path.get(node_index_offset + i) {
                    step.checked_sub(*node.paginator().start())
                } else {
                    None
                };
                let is_active = node_index_offset + i == path.len() - 1;
                let entries = node
                    .children()
                    .iter()
                    .map(|child| child.entry().clone())
                    .collect_vec();
                let constraint =
                    DirectionalConstraint::Horizontal(Constraint::Length(column_width.clone()));

                Rc::new(RefCell::new(
                    ColumnModel::default()
                        .highlight_index_set(highlight_index)
                        .is_active_set(is_active)
                        .entries_set(entries)
                        .selected_entries_set(selected_entries.clone())
                        .constraint_set(constraint),
                ))
            });
        }
    }

    anyhow::Ok(columns)
}

impl<'a> App<'a> {
    pub fn new(
        terminal_size: Size,
        file: &PathBuf,
        raw_value: &'a RawValue,
        path: Box<[Step]>,
    ) -> anyhow::Result<Self> {
        let mut preferences = Preferences::default();
        arg_context!(preferences.apply_term_width(terminal_size.width))?;

        let terminal_area = Rect::new(0, 0, terminal_size.width, terminal_size.height);

        let page_constraint = DirectionalConstraint::Vertical(Constraint::Min(0));
        let bottom_textline_constraint = DirectionalConstraint::Vertical(Constraint::Length(1));
        let status_textline_constraint = DirectionalConstraint::Horizontal(Constraint::Min(1));
        let flpath_textline_constraint = DirectionalConstraint::Horizontal(Constraint::Max(
            u16::try_from(file.display().to_string().chars().count()).unwrap_or(u16::MAX), // FIXME
        ));

        let [page_area, bottom_textline_area] = Layout::vertical(
            match [&page_constraint, &bottom_textline_constraint] {
                [
                    DirectionalConstraint::Vertical(page_constraint),
                    DirectionalConstraint::Vertical(bottom_textline_constraint),
                ] => [
                    page_constraint,
                    bottom_textline_constraint,
                ],
                _ => liab!("Wrong type!"),
            },
        )
        .areas(terminal_area);

        let [status_textline_area, flpath_textline_area] =
        Layout::horizontal(match [&status_textline_constraint, &flpath_textline_constraint] {
            [
                DirectionalConstraint::Horizontal(status_textline_constraint),
                DirectionalConstraint::Horizontal(flpath_textline_constraint),
            ] => [
                status_textline_constraint,
                flpath_textline_constraint,
            ],
            _ => liab!("Wrong type!"),
        })
        .flex(Flex::SpaceBetween)
        .areas(bottom_textline_area);

        let paginator = Paginator::new(page_area.height.into(), 0, None);
        let mut root = Node::new(Some(raw_value), Rc::new(Entry::default()), paginator);
        let path = arg_context!(validate_path(&mut root, &path))?;
        if path.is_empty() {
            liab!("Provided file does not contain any data to show");
        }

        let page = raw_context!(ViewModel::default().try_model_set(raw_context!(
            PageModel::default()
                .left_table_set(
                    TableModel::default()
                        .hide_columns_set(arg_context!(nodes_in_path_to_columns(
                            &root,
                            &path,
                            0,
                            &Vec::<Rc<Entry>>::default(),
                            preferences.left_table_column_width()
                        ))?)
                        .constraint_set(DirectionalConstraint::Horizontal(Constraint::Min(
                            preferences.left_table_column_width().clone()
                        ))),
                )
                .rght_table_set(
                    TableModel::default()
                        .hide_columns_set(vec![Rc::new(RefCell::new(
                            ColumnModel::default()
                                .constraint_set(DirectionalConstraint::Horizontal(
                                    Constraint::Length(
                                        preferences.rght_table_column_width().clone()
                                    )
                                ))
                                .clone(),
                        ))])
                        .constraint_set(DirectionalConstraint::Horizontal(Constraint::Max(0))),
                )
                .constraint_set(page_constraint)
                .area_set(page_area)
        )?))?;

        let bottom_textline = ViewModel::default().model_set(
            TextLineModel::default()
                .area_set(bottom_textline_area)
                .hide_spans_set(vec![(format!(" "), Style::default())])
                .style_set(Style::default().bg(Color::Rgb(80, 73, 69)))
                .alignment_set(Alignment::Center)
                .constraint_set(bottom_textline_constraint),
        );

        let status_textline = ViewModel::default().model_set(
            TextLineModel::default()
                .area_set(status_textline_area)
                .hide_spans_set(vec![(
                    format!(
                        "[{}:{}]: {}",
                        arg_context!(path.len().checked_sub(1))?,
                        arg_context!(path.last())?,
                        arg_context!(node_by_path(&root, &path))?.entry().name()
                    ),
                    Style::default(),
                )])
                .style_set(Style::default().white())
                .alignment_set(Alignment::Left)
                .transparent_set(true)
                .constraint_set(status_textline_constraint),
        );

        let flpath_textline = ViewModel::default().model_set(
            TextLineModel::default()
                .area_set(flpath_textline_area)
                .hide_spans_set(vec![(file.display().to_string(), Style::default())])
                .style_set(Style::default().white())
                .alignment_set(Alignment::Right)
                .transparent_set(true)
                .constraint_set(flpath_textline_constraint),
        );

        let app = Self {
            running: true,
            root,
            path,
            terminal_size,
            preferences,
            page,
            bottom_textline,
            status_textline,
            flpath_textline,
        };

        anyhow::Ok(app)
    }

    fn update_data(
        &mut self,
        cursor_direction: &CursorDirection,
    ) -> anyhow::Result<(Path, Option<usize>, usize, Option<usize>)> {
        let mut new_path = self.path.clone();
        let columns_drain_index;
        let node_index_offset;
        let highlight_index;

        match cursor_direction {
            CursorDirection::Down => {
                match arg_context!(path_change_location(
                    &self.root,
                    &self.path,
                    &cursor_direction
                ))? {
                    PathChangeLocation::Down(path_index) => {
                        arg_context!(kill_children_at_index(
                            &mut self.root,
                            &self.path,
                            path_index + 1
                        ))?;
                        new_path = arg_context!(update_path(
                            &mut self.root,
                            &new_path,
                            UpdatePath::Down(path_index)
                        ))?;
                        columns_drain_index = Some(path_index + 1);
                        node_index_offset = path_index + 1;
                        highlight_index = Some(
                            arg_context!(new_path.get(path_index))?
                                - arg_context!(node_by_path(&self.root, &new_path[0..path_index]))?
                                    .paginator()
                                    .start(),
                        );
                    }
                    PathChangeLocation::NextPage(path_index) => {
                        arg_context!(kill_children_at_index(
                            &mut self.root,
                            &self.path,
                            path_index
                        ))?;
                        new_path = arg_context!(update_path(
                            &mut self.root,
                            &new_path,
                            UpdatePath::Down(path_index)
                        ))?;
                        columns_drain_index = Some(path_index);
                        node_index_offset = path_index;
                        highlight_index = None;
                    }
                    PathChangeLocation::Prohibited => {
                        arg_context!(kill_children_at_index(&mut self.root, &self.path, 0))?;
                        new_path = arg_context!(set_path_steps_to_min(&mut self.root, &new_path))?;
                        columns_drain_index = Some(0);
                        node_index_offset = 0;
                        highlight_index = None;
                    }
                    _ => {
                        liab!("Unexpected case!");
                    }
                }
            }
            CursorDirection::Up => {
                match arg_context!(path_change_location(
                    &self.root,
                    &self.path,
                    &cursor_direction
                ))? {
                    PathChangeLocation::Up(path_index) => {
                        arg_context!(kill_children_at_index(
                            &mut self.root,
                            &self.path,
                            path_index + 1
                        ))?;
                        new_path = arg_context!(update_path(
                            &mut self.root,
                            &new_path,
                            UpdatePath::Up(path_index)
                        ))?;
                        columns_drain_index = Some(path_index + 1);
                        node_index_offset = path_index + 1;
                        highlight_index = Some(
                            arg_context!(new_path.get(path_index))?
                                - arg_context!(node_by_path(&self.root, &new_path[0..path_index]))?
                                    .paginator()
                                    .start(),
                        );
                    }
                    PathChangeLocation::PrevPage(path_index) => {
                        arg_context!(kill_children_at_index(
                            &mut self.root,
                            &self.path,
                            path_index
                        ))?;
                        new_path = arg_context!(update_path(
                            &mut self.root,
                            &new_path,
                            UpdatePath::Up(path_index)
                        ))?;
                        columns_drain_index = Some(path_index);
                        node_index_offset = path_index;
                        highlight_index = None;
                    }
                    PathChangeLocation::Prohibited => {
                        arg_context!(kill_children_at_index(&mut self.root, &self.path, 0))?;
                        new_path = arg_context!(set_path_steps_to_max(&mut self.root, &new_path))?;
                        columns_drain_index = Some(0);
                        node_index_offset = 0;
                        highlight_index = None;
                    }
                    _ => {
                        liab!("Unexpected case!");
                    }
                }
            }
            CursorDirection::Right => {
                new_path = arg_context!(update_path(&mut self.root, &new_path, UpdatePath::Right))?;
                columns_drain_index = None;
                node_index_offset = new_path.len();
                highlight_index = Some(0);
            }
            CursorDirection::Left => {
                new_path = arg_context!(update_path(&mut self.root, &new_path, UpdatePath::Left))?;
                columns_drain_index = Some(self.path.len() - 1);
                node_index_offset = &self.path.len() - 1;
                highlight_index = None;
            }
        }

        anyhow::Ok((
            new_path,
            columns_drain_index,
            node_index_offset,
            highlight_index,
        ))
    }

    pub async fn run<B: Backend>(&mut self, tui: &mut Tui<B>) -> anyhow::Result<()> {
        let mut draw = true;

        while self.running {
            if draw {
                arg_context!(tui.draw(self))?;
                draw = false;
            }

            let event = arg_context!(tui.events.next().await)?;

            match event {
                Event::Tick => self.tick(),
                Event::Key(key_event) => {
                    arg_context!(handle_key_events(key_event, self))?;
                    draw = true;
                }
                Event::Resize(width, height) => {
                    arg_context!(self.set_terminal_size(Size::new(width, height)))?;
                    draw = true;
                }
                Event::Mouse(mouse_event) => {
                    todo!("{:#?}", mouse_event);
                }
            }
        }

        anyhow::Ok(())
    }

    pub fn tick(&self) {}

    pub fn quit(&mut self) {
        self.running = false;
    }

    pub fn inc_left_table_column_width(&mut self) -> anyhow::Result<()> {
        let is_left_table_visible = self.page.model().left_table().area().width != 0;
        let is_rght_table_visible = self.page.model().rght_table().area().width != 0;

        let left_width = self.preferences.left_table_column_width().clone();
        let rght_width = self.preferences.rght_table_column_width().clone();
        let term_width = self.terminal_size.width;

        let mut current_width = 0;
        if is_left_table_visible {
            current_width += left_width;
        }
        if is_rght_table_visible {
            current_width += rght_width;
        }

        if current_width < term_width {
            if let Some(new_width) = self.preferences.left_table_column_width_inc() {
                if is_left_table_visible {
                    raw_context!(self.page.try_with_model_mut(|model| {
                        arg_context!(model.set_left_table_width(new_width))?;

                        anyhow::Ok(())
                    }))?;
                }
            }
        } else if current_width == term_width {
            if is_left_table_visible && is_rght_table_visible {
                if let Some(new_rght_width) = self.preferences.rght_table_column_width_dec() {
                    if let Some(new_left_width) = self.preferences.left_table_column_width_inc() {
                        raw_context!(self.page.try_with_model_mut(|model| {
                            arg_context!(model.set_left_table_width(new_left_width))?;
                            arg_context!(model.set_rght_table_width(new_rght_width))?;

                            anyhow::Ok(())
                        }))?;
                    }
                }
            }
        }

        anyhow::Ok(())
    }

    pub fn dec_left_table_column_width(&mut self) -> anyhow::Result<()> {
        if let Some(new_width) = self.preferences.left_table_column_width_dec() {
            let is_left_table_visible = self.page.model().left_table().area().width != 0;

            if is_left_table_visible {
                raw_context!(self.page.try_with_model_mut(|model| {
                    arg_context!(model.set_left_table_width(new_width))?;

                    anyhow::Ok(())
                }))?;
            }
        }

        anyhow::Ok(())
    }

    pub fn inc_rght_table_column_width(&mut self) -> anyhow::Result<()> {
        let is_left_table_visible = self.page.model().left_table().area().width != 0;
        let is_rght_table_visible = self.page.model().rght_table().area().width != 0;

        let left_width = self.preferences.left_table_column_width().clone();
        let rght_width = self.preferences.rght_table_column_width().clone();
        let term_width = self.terminal_size.width;

        let mut current_width = 0;
        if is_left_table_visible {
            current_width += left_width;
        }
        if is_rght_table_visible {
            current_width += rght_width;
        }

        if current_width < term_width {
            if let Some(new_width) = self.preferences.rght_table_column_width_inc() {
                let is_rght_table_visible = self.page.model().rght_table().area().width != 0;

                if is_rght_table_visible {
                    raw_context!(self.page.try_with_model_mut(|model| {
                        arg_context!(model.set_rght_table_width(new_width))?;

                        anyhow::Ok(())
                    }))?;
                }
            }
        } else if current_width == term_width {
            if is_left_table_visible && is_rght_table_visible {
                if let Some(new_left_width) = self.preferences.left_table_column_width_dec() {
                    if let Some(new_rght_width) = self.preferences.rght_table_column_width_inc() {
                        raw_context!(self.page.try_with_model_mut(|model| {
                            arg_context!(model.set_left_table_width(new_left_width))?;
                            arg_context!(model.set_rght_table_width(new_rght_width))?;

                            anyhow::Ok(())
                        }))?;
                    }
                }
            }
        }

        anyhow::Ok(())
    }

    pub fn dec_rght_table_column_width(&mut self) -> anyhow::Result<()> {
        if let Some(new_width) = self.preferences.rght_table_column_width_dec() {
            let is_rght_table_visible = self.page.model().rght_table().area().width != 0;

            if is_rght_table_visible {
                raw_context!(self.page.try_with_model_mut(|model| {
                    arg_context!(model.set_rght_table_width(new_width))?;

                    anyhow::Ok(())
                }))?;
            }
        }

        anyhow::Ok(())
    }

    pub fn set_terminal_size(&mut self, terminal_size: Size) -> anyhow::Result<()> {
        self.terminal_size = terminal_size;

        let [new_left_width, new_rght_width] =
            arg_context!(self.preferences.apply_term_width(self.terminal_size.width))?;

        let terminal_area = Rect::new(0, 0, self.terminal_size.width, self.terminal_size.height);

        let [page_area, bottom_textline_area] = Layout::vertical(
            {
                let constraints = [self.page.model().constraint(), self.bottom_textline.model().constraint()];
                match constraints {
                    [
                        DirectionalConstraint::Vertical(page_constraint),
                        DirectionalConstraint::Vertical(bottom_textline_constraint),
                    ] => [
                        page_constraint,
                        bottom_textline_constraint,
                    ],
                    _ => liab!("Wrong type: {:?}", constraints),
                }
            }
        )
        .areas(terminal_area);

        let [status_textline_area, flpath_textline_area] = Layout::horizontal(
            {
                let constraints = [self.status_textline.model().constraint(), self.flpath_textline.model().constraint()];
                match constraints {
                    [
                        DirectionalConstraint::Horizontal(status_textline_constraint),
                        DirectionalConstraint::Horizontal(flpath_textline_constraint),
                    ] => [
                        status_textline_constraint,
                        flpath_textline_constraint,
                    ],
                    _ => liab!("Wrong type: {:?}", constraints),
                }
            }
        )
        .flex(Flex::SpaceBetween)
        .areas(bottom_textline_area);

        self.root
            .paginator_mut()
            .size_update(page_area.height.into());
        self.root.kill_children(); // FIXME
        arg_context!(validate_path(&mut self.root, &self.path))?;

        let selected_entries = {
            let column = arg_context!(self.page.model().rght_table().hide_columns().get(0))?;
            arg_context!(column.try_borrow())?.entries().clone()
        };

        let new_columns = arg_context!(nodes_in_path_to_columns(
            &self.root,
            &self.path,
            0,
            &selected_entries,
            self.preferences.left_table_column_width()
        ))?;

        raw_context!(self.page.try_with_model_mut(|model| {
            if let Some(new_width) = new_left_width {
                arg_context!(model.set_left_table_width(new_width))?;
            }

            if let Some(new_width) = new_rght_width {
                arg_context!(model.set_rght_table_width(new_width))?;
            }

            arg_context!(model.left_table_mut().update(
                &CursorDirection::Down,
                Some(0),
                None::<usize>,
                &new_columns,
            ))?;

            arg_context!(model.area_update(page_area))?;

            anyhow::Ok(())
        }))?;

        raw_context!(self.bottom_textline.with_model_mut(|model| {
            model.area_update(bottom_textline_area);
            anyhow::Ok(())
        }))?;

        raw_context!(self.status_textline.with_model_mut(|model| {
            model.area_update(status_textline_area);
            anyhow::Ok(())
        }))?;

        raw_context!(self.flpath_textline.with_model_mut(|model| {
            model.area_update(flpath_textline_area);
            anyhow::Ok(())
        }))?;

        anyhow::Ok(())
    }

    pub fn cursor_move(&mut self, cursor_direction: CursorDirection) -> anyhow::Result<()> {
        let (new_path, columns_drain_index, node_index_offset, highlight_index) =
            arg_context!(self.update_data(&cursor_direction))?;

        self.path = new_path;

        let selected_entries = {
            let column = arg_context!(self.page.model().rght_table().hide_columns().get(0))?;
            arg_context!(column.try_borrow())?.entries().clone()
        };

        let new_columns = arg_context!(nodes_in_path_to_columns(
            &self.root,
            &self.path,
            node_index_offset,
            &selected_entries,
            self.preferences.left_table_column_width()
        ))?;

        raw_context!(self.page.try_with_model_mut(|model| {
            arg_context!(model.left_table_mut().update(
                &cursor_direction,
                columns_drain_index,
                highlight_index,
                &new_columns,
            ))?;

            anyhow::Ok(())
        }))?;

        raw_context!(self.status_textline.with_model_mut(|model| {
            model.hide_spans_update(vec![(
                format!(
                    "[{}:{}]: {}",
                    arg_context!(self.path.len().checked_sub(1))?,
                    arg_context!(self.path.last())?,
                    arg_context!(node_by_path(&self.root, &self.path))?
                        .entry()
                        .name()
                ),
                Style::default(),
            )]);

            anyhow::Ok(())
        }))?;

        anyhow::Ok(())
    }

    pub fn cursor_select(&mut self) -> anyhow::Result<()> {
        let hovered_entry = arg_context!(node_by_path(&self.root, &self.path))?
            .entry()
            .clone();

        let [new_left_width, new_rght_width] =
            arg_context!(self.preferences.apply_term_width(self.terminal_size.width))?;

        raw_context!(self.page.try_with_model_mut(|model| {
            if let Some(new_width) = new_left_width {
                arg_context!(model.set_left_table_width(new_width))?;
            }
            if let Some(new_width) = new_rght_width {
                arg_context!(model.set_rght_table_width(new_width))?;
            }

            let before = {
                let rght_table_column = arg_context!(model.rght_table().hide_columns().get(0))?;
                let rght_table_column = arg_context!(rght_table_column.try_borrow())?;

                rght_table_column.entries().is_empty()
            };

            let entry_in_rght_table = {
                let rght_table_column =
                    arg_context!(model.rght_table_mut().hide_columns_mut().get_mut(0))?;
                let mut rght_table_column = arg_context!(rght_table_column.try_borrow_mut())?;

                let is_entry_in_rght_table = rght_table_column.entries().contains(&hovered_entry);

                if is_entry_in_rght_table {
                    rght_table_column
                        .entries_mut()
                        .retain(|entry| *entry != hovered_entry);
                } else {
                    rght_table_column
                        .entries_mut()
                        .insert(0, hovered_entry.clone());
                }

                is_entry_in_rght_table
            };

            let after = {
                let rght_table_column = arg_context!(model.rght_table().hide_columns().get(0))?;
                let rght_table_column = arg_context!(rght_table_column.try_borrow())?;

                rght_table_column.entries().is_empty()
            };

            if after != before {
                arg_context!(model.set_rght_table_width(if after {
                    0
                } else {
                    self.preferences.rght_table_column_width().clone()
                }))?;
            }

            raw_context!(model
                .left_table_mut()
                .hide_columns_mut()
                .iter_mut()
                .try_for_each(|column| {
                    if entry_in_rght_table {
                        arg_context!(column.try_borrow_mut())?
                            .selected_entries_mut()
                            .retain(|entry| *entry != hovered_entry);
                    } else {
                        arg_context!(column.try_borrow_mut())?
                            .selected_entries_mut()
                            .insert(0, hovered_entry.clone());
                    }

                    anyhow::Ok(())
                }))?;

            raw_context!(model
                .rght_table_mut()
                .hide_columns_mut()
                .iter_mut()
                .try_for_each(|column| {
                    if entry_in_rght_table {
                        arg_context!(column.try_borrow_mut())?
                            .selected_entries_mut()
                            .retain(|entry| *entry != hovered_entry);
                    } else {
                        arg_context!(column.try_borrow_mut())?
                            .selected_entries_mut()
                            .insert(0, hovered_entry.clone());
                    }

                    anyhow::Ok(())
                }))?;

            anyhow::Ok(())
        }))?;

        anyhow::Ok(())
    }

    pub fn print(&self) -> anyhow::Result<()> {
        let rght_table_column = arg_context!(self.page.model().rght_table().hide_columns().get(0))?;

        arg_context!(rght_table_column.try_borrow())?
            .entries()
            .iter()
            .for_each(|entry| {
                println!("{}", entry.name());
            });

        anyhow::Ok(())
    }

    pub fn clear_selected(&mut self) -> anyhow::Result<()> {
        raw_context!(self.page.try_with_model_mut(|model| {
            raw_context!(model
                .left_table_mut()
                .hide_columns_mut()
                .iter_mut()
                .try_for_each(|column| {
                    let mut column = arg_context!(column.try_borrow_mut())?;
                    column.selected_entries_update(Vec::default());

                    anyhow::Ok(())
                }))?;

            raw_context!(model
                .rght_table_mut()
                .hide_columns_mut()
                .iter_mut()
                .try_for_each(|column| {
                    let mut column = arg_context!(column.try_borrow_mut())?;
                    column.entries_update(Vec::default());
                    column.selected_entries_update(Vec::default());

                    anyhow::Ok(())
                }))?;

            anyhow::Ok(())
        }))?;

        anyhow::Ok(())
    }
}

impl<'a> Render for App<'a> {
    fn render(&mut self, frame: &mut Frame) {
        frame.render_widget(Clear, frame.area());

        self.page.render(frame);
        self.bottom_textline.render(frame);
        self.status_textline.render(frame);
        self.flpath_textline.render(frame);
    }
}
