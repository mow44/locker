use impl_helper::ImplHelper;
use std::sync::OnceLock;
use wrap_context::{arg_context, liab};

use crate::{
    node::Node,
    types::{CursorDirection, Path, Step},
};

pub static DEBUG_PRINT_LIMIT: OnceLock<usize> = OnceLock::new();

pub fn clip_string(mut string: String, ending: &str, length: usize) -> String {
    if length == 0 {
        string = String::default();
    } else if string.chars().count() > length {
        string = format!(
            "{}{}",
            string
                .chars()
                .take(length.saturating_sub(ending.chars().count()))
                .collect::<String>(),
            ending
        );
    }

    string
}

#[derive(Debug, Clone, Copy, ImplHelper, PartialEq)]
pub struct Location {
    #[helper(all)]
    start: usize,

    #[helper(all)]
    finish: usize,
}

impl Location {
    pub fn new(start: usize, finish: usize) -> Self {
        Self { start, finish }
    }
}

pub trait SliceFromLocation<T> {
    fn slice(&self, location: &Location) -> &[T];
}

impl<T> SliceFromLocation<T> for [T] {
    fn slice(&self, location: &Location) -> &[T] {
        &self[location.start..=location.finish]
    }
}

#[derive(Debug)]
pub enum PathChangeLocation {
    Down(usize),
    NextPage(usize),

    Up(usize),
    PrevPage(usize),

    Prohibited,
}

#[derive(Debug, Clone, Copy)]
pub enum UpdatePath {
    Up(usize),
    Down(usize),
    Right,
    Left,
}

pub fn node_by_path<'a>(root: &'a Node, path: &[Step]) -> anyhow::Result<&'a Node<'a>> {
    let mut current_node = root;

    for (i, step) in path.iter().enumerate() {
        let shifted_step = arg_context!(step.checked_sub(*current_node.paginator().start()))?;

        if let Some(next_node) = current_node.children().get(shifted_step) {
            current_node = next_node;
        } else {
            liab!(
                "In the path: {:?}, the node at step: {:?} (index: {:?}) does not exist",
                path,
                step,
                i
            );
        }
    }

    anyhow::Ok(current_node)
}

pub fn path_change_location(
    root: &Node,
    path: &[Step],
    cursor_direction: &CursorDirection,
) -> anyhow::Result<PathChangeLocation> {
    let mut current_node = root;
    let mut nodes_in_path = vec![];

    for (i, step) in path.iter().enumerate() {
        nodes_in_path.push(current_node);

        let shifted_step = arg_context!(step.checked_sub(*current_node.paginator().start()))?;

        if let Some(next_node) = current_node.children().get(shifted_step) {
            current_node = next_node;
        } else {
            liab!(
                "In the path: {:?}, the node at step: {:?} (index: {:?}) does not exist",
                path,
                step,
                i
            );
        }
    }

    for (i, (node, step)) in nodes_in_path.iter().zip(path.iter()).enumerate().rev() {
        match cursor_direction {
            CursorDirection::Up => {
                if *step > 0 {
                    if step == node.paginator().start() {
                        return anyhow::Ok(PathChangeLocation::PrevPage(i));
                    } else {
                        return anyhow::Ok(PathChangeLocation::Up(i));
                    }
                }
            }
            CursorDirection::Down => {
                let shifted_step = arg_context!(step.checked_sub(*node.paginator().start()))?;

                if shifted_step < node.children().len() - 1 {
                    return anyhow::Ok(PathChangeLocation::Down(i));
                } else if node.paginator().start() + node.paginator().size()
                    < arg_context!(node.paginator().total())?
                {
                    return anyhow::Ok(PathChangeLocation::NextPage(i));
                }
            }
            _ => {
                liab!("Wrong cursor_direction: {:?}", cursor_direction)
            }
        }
    }

    anyhow::Ok(PathChangeLocation::Prohibited)
}

pub fn set_path_steps_to_min(root: &mut Node, path_slice: &[Step]) -> anyhow::Result<Path> {
    let mut path = path_slice.to_vec();

    let mut current_node = root;
    let mut drain_index = None;

    let last_index = arg_context!(path.len().checked_sub(1))?;

    for (i, step) in path.iter_mut().enumerate() {
        // The last node in the tree doesn't have any children, so the total is None
        if let Some(total) = current_node.paginator().total() {
            // In this case, the page will change, so there's no need to keep this node's children in memory
            if total > current_node.paginator().size() {
                current_node.kill_children();
            }
        }

        // There's no need to calculate the shifted step because the step will be 0
        *step = arg_context!(current_node.make_children(Step::MIN))?;

        if let Some(next_node) = current_node.children_mut().get_mut(*step) {
            current_node = next_node;
        } else {
            drain_index = Some(i);
            break;
        }

        if i == last_index && current_node.children().is_empty() {
            arg_context!(current_node.make_children(Step::MIN))?;
        }
    }

    if let Some(index) = drain_index {
        path.drain(index..);
    }

    anyhow::Ok(path)
}

pub fn set_path_steps_to_max(root: &mut Node, path_slice: &[Step]) -> anyhow::Result<Path> {
    let mut path = path_slice.to_vec();

    let mut current_node = root;
    let mut drain_index = None;

    let last_index = path.len().saturating_sub(1);

    for (i, step) in path.iter_mut().enumerate() {
        // The last node in the tree doesn't have any children, so the total is None
        if let Some(total) = current_node.paginator().total() {
            // In this case, the page will change, so there's no need to keep this node's children in memory
            if total > current_node.paginator().size() {
                current_node.kill_children();
            }
        }

        *step = arg_context!(current_node.make_children(Step::MAX))?;
        let shifted_step = arg_context!(step.checked_sub(*current_node.paginator().start()))?;

        if let Some(next_node) = current_node.children_mut().get_mut(shifted_step) {
            current_node = next_node;
        } else {
            drain_index = Some(i);
            break;
        }

        if i == last_index && current_node.children().is_empty() {
            arg_context!(current_node.make_children(Step::MIN))?;
        }
    }

    if let Some(index) = drain_index {
        path.drain(index..);
    }

    anyhow::Ok(path)
}

pub fn validate_path(root: &mut Node, path_slice: &[Step]) -> anyhow::Result<Path> {
    let mut path = path_slice.to_vec();

    let mut current_node = root;
    let mut drain_index = None;

    let last_index = path.len().saturating_sub(1);

    for (i, step) in path.iter_mut().enumerate() {
        if current_node.children().is_empty() {
            // if let Err(e) = current_node.make_children(*step) {
            //     // liab!("{}", from_utf8(current_node.source().unwrap())?.to_string());
            //     liab!("{:#?}", e);
            // }
            *step = arg_context!(current_node.make_children(*step))?;
        }

        let shifted_step = arg_context!(step.checked_sub(*current_node.paginator().start()))?;

        if let Some(next_node) = current_node.children_mut().get_mut(shifted_step) {
            current_node = next_node;
        } else {
            drain_index = Some(i);
            break;
        }

        if i == last_index && current_node.children().is_empty() {
            arg_context!(current_node.make_children(0))?;
        }
    }

    if let Some(index) = drain_index {
        path.drain(index..);
    }

    anyhow::Ok(path)
}

pub fn update_path(
    root: &mut Node,
    path_slice: &[Step],
    change: UpdatePath,
) -> anyhow::Result<Path> {
    let mut path = path_slice.to_vec();

    let mut current_node = root;

    let mut path_extension: Path = vec![];
    let mut drain_index: Option<usize> = None;

    let path_len = path.len();

    for (i, step) in path.iter_mut().enumerate() {
        match change {
            UpdatePath::Down(change_index) => {
                if i == change_index {
                    *step = step.saturating_add(1);

                    if *step >= current_node.paginator().start() + current_node.paginator().size() {
                        current_node.kill_children();
                        *step = arg_context!(current_node.make_children(*step))?;
                    }
                } else if i > change_index {
                    current_node.kill_children();
                    *step = arg_context!(current_node.make_children(Step::MIN))?;
                }
            }
            UpdatePath::Up(change_index) => {
                if i == change_index {
                    *step = step.saturating_sub(1);

                    if *step < *current_node.paginator().start() {
                        current_node.kill_children();
                        *step = arg_context!(current_node.make_children(*step))?;
                    }
                } else if i > change_index {
                    current_node.kill_children();
                    *step = arg_context!(current_node.make_children(Step::MAX))?;
                }
            }
            _ => {}
        }

        let shifted_step = arg_context!(step.checked_sub(*current_node.paginator().start()))?;

        if let Some(next_node) = current_node.children_mut().get_mut(shifted_step) {
            current_node = next_node;
        } else {
            drain_index = Some(i);
            break;
        }

        match change {
            // This code should be executed when moving up or down to show previews
            UpdatePath::Down(_) | UpdatePath::Up(_) => {
                // If the current_node is the last node in the tree
                if path_len.checked_sub(1).is_some_and(|result| result == i) {
                    if current_node.children().is_empty() {
                        arg_context!(current_node.make_children(Step::MIN))?;
                    }
                }
            }
            _ => {}
        }

        match change {
            UpdatePath::Left => {
                // If the current_node is the second-to-last node in the tree
                if path_len.checked_sub(2).is_some_and(|result| result == i) {
                    drain_index = Some(path_len.saturating_sub(1));
                    current_node.kill_children(); // FIXME
                    current_node.paginator_mut().start_update(Step::MIN);
                    arg_context!(current_node.make_children(Step::MIN))?;
                }
            }
            UpdatePath::Right => {
                // If the current_node is the last node in the tree
                if path_len.checked_sub(1).is_some_and(|result| result == i) {
                    let new_step = Step::MIN;

                    if let Some(new_node) = current_node.children_mut().get_mut(new_step) {
                        path_extension.push(new_step);
                        arg_context!(new_node.make_children(new_step))?;
                    }
                }
            }
            _ => {}
        }
    }

    if let Some(index) = drain_index {
        path.drain(index..);
    }

    path.extend(path_extension);

    anyhow::Ok(path)
}

pub fn kill_children_at_index(root: &mut Node, path: &[Step], index: usize) -> anyhow::Result<()> {
    let mut current_node = root;
    let mut current_index = 0;

    if index == current_index {
        current_node.kill_children();
        return anyhow::Ok(());
    }

    for (i, step) in path.iter().enumerate() {
        let shifted_step = arg_context!(step.checked_sub(*current_node.paginator().start()))?;

        if let Some(next_node) = current_node.children_mut().get_mut(shifted_step) {
            current_node = next_node;
        } else {
            liab!(
                "In the path {:?}, the node at step {:?} (index {:?}) does not exist",
                path,
                step,
                i
            );
        }

        current_index += 1;

        if index == current_index {
            current_node.kill_children();
            return anyhow::Ok(());
        }
    }

    liab!(
        "In the path {:?}, the specified index {:?} is unreachable",
        path,
        index
    );
}
