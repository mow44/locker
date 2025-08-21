use impl_helper::ImplHelper;
use std::{fmt, rc::Rc, str::from_utf8};

use wrap_context::{arg_context, liab, raw_context};

use crate::{
    lexer::{get_array_items, get_object_items, items_to_vec, row_col_position, Lexer, TokenKind},
    paginator::Paginator,
    types::{Entry, Step},
    utils::{Location, SliceFromLocation},
};

#[derive(Clone, ImplHelper, PartialEq)]
struct TokenInfo {
    #[helper(all)]
    kind: TokenKind,

    #[helper(all)]
    items: Vec<Location>,
}

impl TokenInfo {
    pub fn new<'a>(source: &'a [u8], location: Location) -> anyhow::Result<Self> {
        let mut lexer = Lexer::new(source, location);
        let token = arg_context!(lexer.next_token())?;
        let kind = token.kind().clone();

        let items = match kind {
            TokenKind::Object => {
                let start = arg_context!(token.location().start().checked_add(1))?;
                let end = arg_context!(token.location().finish().checked_sub(1))?;

                if start <= end && end < source.len() {
                    arg_context!(get_object_items(&source, Location::new(start, end)))?
                } else {
                    vec![]
                }
            }
            TokenKind::Array => {
                let start = arg_context!(token.location().start().checked_add(1))?;
                let end = arg_context!(token.location().finish().checked_sub(1))?;

                if start <= end && end < source.len() {
                    arg_context!(get_array_items(&source, Location::new(start, end)))?
                } else {
                    vec![]
                }
            }
            TokenKind::String => {
                let start = arg_context!(token.location().start().checked_add(1))?;
                let end = arg_context!(token.location().finish().checked_sub(1))?;

                if start <= end && end < source.len() {
                    vec![Location::new(start, end)]
                } else {
                    vec![]
                }
            }
            TokenKind::Sequence => {
                let start = token.location().start().clone();
                let end = token.location().finish().clone();

                vec![Location::new(start, end)]
            }
            _ => liab!(
                "Expected Object, Array, String or Sequence, but got {:?} at [{}]",
                token.kind(),
                arg_context!(row_col_position(&source[..=*token.location().start()]))?
            ),
        };

        anyhow::Ok(Self { kind, items })
    }
}

#[derive(Clone, ImplHelper, PartialEq)]
pub struct Node<'a> {
    #[helper(all)]
    source: &'a [u8],
    location: Option<Location>,
    token_info: Option<TokenInfo>,

    #[helper(all)]
    entry: Rc<Entry>,

    #[helper(all)]
    children: Vec<Node<'a>>,

    #[helper(all)]
    paginator: Paginator,
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
    pub fn new(
        source: &'a [u8],
        location: Option<Location>,
        entry: Rc<Entry>,
        paginator: Paginator,
    ) -> anyhow::Result<Self> {
        let token_info = None;

        anyhow::Ok(Self {
            source,
            location,
            token_info,
            entry,
            children: vec![],
            paginator,
        })
    }

    pub fn kill_children(&mut self) {
        self.children.clear();
    }

    pub fn make_children(&mut self, mut target: Step) -> anyhow::Result<usize> {
        if self.token_info.is_none() {
            if let Some(location) = self.location {
                let token_info = arg_context!(TokenInfo::new(self.source, location))?;
                self.paginator.total_update(Some(token_info.items().len()));
                self.token_info = Some(token_info);
            }
        }

        // TODO maybe add liab if !self.children.is_empty()
        let mut children = vec![];
        let mut path = self.entry.path().clone();

        if let Some(token_info) = &self.token_info {
            match token_info.kind() {
                TokenKind::Object => {
                    if !token_info.items().is_empty() {
                        target = target.min(arg_context!(token_info.items().len().checked_sub(1))?);

                        let page_location = arg_context!(self
                            .paginator
                            .page_location(target, token_info.items().len()))?;
                        self.paginator.start_update(page_location.start().clone());
                        path.push(page_location.start().clone());

                        let mut blanks =
                            items_to_vec(&self.source, &token_info.items().slice(&page_location))?;

                        while let Some((name, location)) = blanks.pop_front() {
                            children.push(raw_context!(Node::new(
                                self.source,
                                location,
                                Rc::new(Entry::new(name, path.clone())),
                                Paginator::new(self.paginator.size().clone(), 0, None)
                            ))?);

                            if let Some(last_step) = path.last_mut() {
                                *last_step = arg_context!(last_step.checked_add(1))?;
                            }
                        }
                    }
                }
                TokenKind::Array => {
                    if !token_info.items().is_empty() {
                        target = target.min(arg_context!(token_info.items().len().checked_sub(1))?);

                        let page_location = arg_context!(self
                            .paginator
                            .page_location(target, token_info.items().len()))?;
                        self.paginator.start_update(page_location.start().clone());
                        path.push(page_location.start().clone());

                        let mut blanks =
                            items_to_vec(&self.source, &token_info.items().slice(&page_location))?;

                        while let Some((name, location)) = blanks.pop_front() {
                            let node = raw_context!(Node::new(
                                self.source,
                                location,
                                Rc::new(Entry::new(name, path.clone())),
                                Paginator::new(self.paginator.size().clone(), 0, None)
                            ))?;

                            children.push(node);

                            if let Some(last_step) = path.last_mut() {
                                *last_step = arg_context!(last_step.checked_add(1))?;
                            }
                        }
                    }
                }
                TokenKind::String | TokenKind::Sequence => {
                    target = 0;

                    let name = if let Some(location) = token_info.items().get(0) {
                        arg_context!(from_utf8(&self.source.slice(location)))?.to_string()
                    } else {
                        String::default()
                    };

                    let first_step = self.paginator.start().clone();
                    path.push(first_step);

                    self.paginator.total_update(Some(1));

                    let node = Node::new(
                        &self.source,
                        None,
                        Rc::new(Entry::new(name, path.clone())),
                        Paginator::new(self.paginator.size().clone(), 0, Some(0)),
                    )?;

                    children.push(node);
                }
                _ => liab!("Unexpected token kind: {:?}", token_info.kind()),
            }
        }

        self.children = children;

        anyhow::Ok(target)
    }
}
