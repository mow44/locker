use impl_helper::ImplHelper;
use serde::de::DeserializeSeed;
use serde_json::{from_str, value::RawValue, Value};
use std::{fmt, rc::Rc};

use wrap_context::{arg_context, raw_context};

use crate::{
    paginated_map::*,
    paginated_vec::*,
    paginator::Paginator,
    types::{Entry, Step},
    utils::{raw_value_type, RawValueType},
};

#[derive(Clone, Default, ImplHelper)]
pub struct Node<'a> {
    raw_value: Option<&'a RawValue>,

    #[helper(all)]
    entry: Rc<Entry>,

    #[helper(all)]
    children: Vec<Node<'a>>,

    #[helper(all)]
    paginator: Paginator,
}

impl<'a> PartialEq for Node<'a> {
    fn eq(&self, other: &Self) -> bool {
        let mut result = true;

        // FIXME
        match (self.raw_value, other.raw_value) {
            (Some(self_raw_value), Some(other_raw_value)) => {
                if self_raw_value.get() != other_raw_value.get() {
                    result = false;
                }
            }
            (None, None) => {}
            _ => {
                result = false;
            }
        }

        if self.entry.as_ref().clone() != other.entry.as_ref().clone() {
            result = false;
        }

        if self.children != other.children {
            result = false;
        }

        if self.paginator != other.paginator {
            result = false;
        }

        result
    }
}

impl<'a> fmt::Debug for Node<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Node")
            .field("entry", &self.entry)
            .field("children", &self.children)
            .finish()
    }
}

impl<'a> Node<'a> {
    pub fn new(raw_value: Option<&'a RawValue>, entry: Rc<Entry>, paginator: Paginator) -> Self {
        Self {
            raw_value,
            entry,
            children: vec![],
            paginator,
        }
    }

    pub fn kill_children(&mut self) {
        self.children.clear();
    }

    pub fn make_children(&mut self, mut target_index: Step) -> anyhow::Result<usize> {
        // TODO maybe add liab if !self.children.is_empty()

        let mut children = vec![];
        let mut path = self.entry.path().clone();

        if let Some(raw_value) = self.raw_value {
            match raw_value_type(raw_value) {
                RawValueType::Object => {
                    let mut treemap = PaginatedMap::<String, &'a RawValue>::new(self.paginator);
                    let mut deserializer = serde_json::Deserializer::from_str(raw_value.get());

                    raw_context!(PaginatedMapWrapper(&mut treemap, &mut target_index)
                        .deserialize(&mut deserializer))?;

                    self.paginator = treemap.paginator().clone();

                    path.push(self.paginator.start().clone());

                    let paginator = Paginator::new(self.paginator.size().clone(), 0, None);

                    for (name, raw_value) in treemap.data().iter() {
                        children.push(Node::new(
                            Some(raw_value),
                            Rc::new(Entry::new(name.clone(), path.clone())),
                            paginator,
                        ));

                        if let Some(last_step) = path.last_mut() {
                            *last_step = arg_context!(last_step.checked_add(1))?;
                        }
                    }
                }
                RawValueType::Array => {
                    let mut vector = PaginatedVec::<&'a RawValue>::new(self.paginator);
                    let mut deserializer = serde_json::Deserializer::from_str(raw_value.get());

                    raw_context!(PaginatedVecWrapper(&mut vector, &mut target_index)
                        .deserialize(&mut deserializer))?;

                    self.paginator = vector.paginator().clone();

                    path.push(self.paginator.start().clone());

                    for raw_value in vector.data().iter() {
                        let node = match raw_value_type(raw_value) {
                            RawValueType::Other => {
                                let value = arg_context!(from_str::<Value>(raw_value.get()))?;
                                let entry_name = if value.is_string() {
                                    let chars: Vec<char> = value.to_string().chars().collect();

                                    if chars.len() <= 2 {
                                        String::default()
                                    } else {
                                        chars[1..chars.len() - 1].iter().collect()
                                    }
                                } else {
                                    value.to_string()
                                };

                                Node::new(
                                    None,
                                    Rc::new(Entry::new(entry_name, path.clone())),
                                    Paginator::new(self.paginator.size().clone(), 0, Some(0)),
                                )
                            }
                            RawValueType::Object => Node::new(
                                Some(*raw_value),
                                Rc::new(Entry::new(format!("{{...}}"), path.clone())),
                                Paginator::new(self.paginator.size().clone(), 0, None),
                            ),
                            RawValueType::Array => Node::new(
                                Some(*raw_value),
                                Rc::new(Entry::new(format!("[...]"), path.clone())),
                                Paginator::new(self.paginator.size().clone(), 0, None),
                            ),
                        };

                        if let Some(last_step) = path.last_mut() {
                            *last_step = arg_context!(last_step.checked_add(1))?;
                        }

                        children.push(node);
                    }
                }
                RawValueType::Other => {
                    target_index = 0;

                    let first_step = self.paginator.start().clone();
                    path.push(first_step);

                    self.paginator.total_update(Some(1));

                    let value = arg_context!(from_str::<Value>(raw_value.get()))?;
                    let entry_name = if value.is_string() {
                        let chars: Vec<char> = value.to_string().chars().collect();

                        if chars.len() <= 2 {
                            String::default()
                        } else {
                            chars[1..chars.len() - 1].iter().collect()
                        }
                    } else {
                        value.to_string()
                    };

                    let node = Node::new(
                        None,
                        Rc::new(Entry::new(entry_name, path.clone())),
                        Paginator::new(self.paginator.size().clone(), 0, Some(0)),
                    );

                    children.push(node);
                }
            }
        }

        self.children = children;

        anyhow::Ok(target_index)
    }
}
