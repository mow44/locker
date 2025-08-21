use std::{collections::VecDeque, str::from_utf8};

use impl_helper::ImplHelper;
use memchr::{memchr, memchr_iter};
use wrap_context::{arg_context, liab, raw_context};

use crate::utils::{Location, SliceFromLocation};

fn count_quotes(haystack: &[u8]) -> anyhow::Result<usize> {
    let mut it = memchr_iter(b'"', haystack);
    let mut quote_counter = 0;

    while let Some(mut i) = it.next() {
        let mut backslash_counter = 0;

        while i > 0 {
            i -= 1;

            if arg_context!(haystack.get(i))? == &b'\\' {
                backslash_counter += 1;
            } else {
                break;
            }
        }

        if backslash_counter % 2 == 0 {
            quote_counter += 1;
        }
    }

    anyhow::Ok(quote_counter)
}

fn count_needles(needle: u8, haystack: &[u8], quotes: &mut usize) -> anyhow::Result<usize> {
    if let Some(position) = memchr(needle, haystack) {
        *quotes += arg_context!(count_quotes(&haystack[..position]))?;

        if arg_context!(position.checked_add(1))? >= haystack.len() {
            if *quotes % 2 == 0 {
                return anyhow::Ok(1);
            } else {
                return anyhow::Ok(0);
            }
        } else {
            if *quotes % 2 == 0 {
                return anyhow::Ok(
                    1 + raw_context!(count_needles(
                        needle,
                        &haystack[arg_context!(position.checked_add(1))?..],
                        quotes,
                    ))?,
                );
            } else {
                return anyhow::Ok(
                    0 + raw_context!(count_needles(
                        needle,
                        &haystack[arg_context!(position.checked_add(1))?..],
                        quotes,
                    ))?,
                );
            }
        }
    } else {
        *quotes += arg_context!(count_quotes(&haystack))?;

        return anyhow::Ok(0);
    }
}

pub fn row_col_position(source: &[u8]) -> anyhow::Result<String> {
    let mut it = memchr_iter(b'\n', source);

    let mut row = 1; // because rows start from 1

    let mut last_newline_position = 0;
    while let Some(i) = it.next() {
        row += 1;
        last_newline_position = i + 1; // + 1 because columns start from 1
    }

    let col = source.len() - last_newline_position;

    anyhow::Ok(format!("{row}:{col}"))
}

fn find_block(
    state: &mut LexerState,
    data: &LexerData,
    openning: u8,
    ending: u8,
) -> anyhow::Result<Location> {
    let start = state.pos().clone();

    let mut search_start = arg_context!(start.checked_add(1))?;
    if search_start > *data.location().finish() {
        liab!(
            "Could not find '{}' for '{}' at [{}]",
            from_utf8(&[ending])?.to_string(),
            from_utf8(&[openning])?.to_string(),
            row_col_position(&data.source()[..=start])?,
        );
    }

    let mut quotes = 0;
    let mut opennings = 1;

    loop {
        if let Some(position) = memchr(
            ending,
            &data.source()[search_start..=*data.location().finish()],
        ) {
            let finish = search_start + position;

            opennings +=
                count_needles(openning, &data.source()[search_start..finish], &mut quotes)?;

            if quotes % 2 != 0 {
                search_start = arg_context!(finish.checked_add(1))?;
                continue;
            }

            opennings -= 1;

            if opennings == 0 {
                state.pos_update(finish);
                state.byt_update(Some(ending));

                return anyhow::Ok(Location::new(start, finish));
            } else {
                search_start = arg_context!(finish.checked_add(1))?;
            }
        } else {
            liab!(
                "Could not find '{}' for '{}' at [{}]",
                from_utf8(&[ending])?.to_string(),
                from_utf8(&[openning])?.to_string(),
                row_col_position(&data.source()[..=start])?,
            );
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum TokenKind {
    LastChar,
    Comma,
    Colon,
    Object,
    Array,
    String,
    Sequence,
}

#[derive(Debug, Clone, ImplHelper)]
pub struct Token {
    #[helper(all)]
    kind: TokenKind,

    #[helper(all)]
    location: Location,
}

impl Token {
    pub fn new(kind: TokenKind, location: Location) -> Self {
        Self { kind, location }
    }
}

#[derive(Debug, Clone, ImplHelper)]
pub struct LexerState {
    #[helper(all)]
    pos: usize,

    #[helper(all)]
    byt: Option<u8>,
}

impl LexerState {
    pub fn new(pos: usize, byt: Option<u8>) -> Self {
        Self { pos, byt }
    }
}

#[derive(Debug, Clone, ImplHelper)]
pub struct LexerData<'a> {
    #[helper(get)]
    source: &'a [u8],

    #[helper(get)]
    location: Location,
}

impl<'a> LexerData<'a> {
    pub fn new(source: &'a [u8], location: Location) -> Self {
        Self { source, location }
    }
}

#[derive(Debug, Clone, ImplHelper)]
pub struct Lexer<'a> {
    #[helper(all)]
    data: LexerData<'a>,

    #[helper(all)]
    state: LexerState,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a [u8], location: Location) -> Self {
        let data = LexerData::new(source, location);

        let pos = location.start().clone();
        let byt = data.source().get(pos).cloned();
        let state = LexerState::new(pos, byt);

        Self { data, state }
    }

    pub fn read_next_char(&mut self) -> anyhow::Result<()> {
        if self.state.byt().is_some() {
            let next_pos = arg_context!(self.state.pos().checked_add(1))?;
            let next_byt = self.data.source().get(next_pos).cloned();

            self.state.pos_update(next_pos);
            self.state.byt_update(next_byt);
        }

        if self.state.pos() > self.data.location.finish() {
            self.state.byt_update(None);
        }

        anyhow::Ok(())
    }

    pub fn peek_next_char(&mut self) -> anyhow::Result<Option<u8>> {
        let next_pos = arg_context!(self.state.pos().clone().checked_add(1))?;
        let mut next_byt = self.data.source().get(next_pos).cloned();

        if next_pos > *self.data.location.finish() {
            next_byt = None;
        }

        anyhow::Ok(next_byt)
    }

    pub fn skip_spaces(&mut self) -> anyhow::Result<()> {
        loop {
            if let Some(current_byte) = self.state.byt() {
                let spaces = &[b' ', b'\t', b'\n', b'\r'];
                if spaces.contains(current_byte) {
                    arg_context!(self.read_next_char())?;
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        anyhow::Ok(())
    }

    pub fn next_token(&mut self) -> anyhow::Result<Token> {
        arg_context!(self.skip_spaces())?;

        let token = match self.state.byt() {
            None => Token::new(
                TokenKind::LastChar,
                Location::new(
                    arg_context!(self.state.pos().checked_sub(1))?,
                    arg_context!(self.state.pos().checked_sub(1))?,
                ),
            ),
            Some(b':') => Token::new(
                TokenKind::Colon,
                Location::new(self.state.pos().clone(), self.state.pos().clone()),
            ),
            Some(b',') => Token::new(
                TokenKind::Comma,
                Location::new(self.state.pos().clone(), self.state.pos().clone()),
            ),
            Some(b'{') => Token::new(
                TokenKind::Object,
                arg_context!(find_block(&mut self.state, &self.data, b'{', b'}'))?,
            ),
            Some(b'[') => Token::new(
                TokenKind::Array,
                arg_context!(find_block(&mut self.state, &self.data, b'[', b']'))?,
            ),
            Some(b'"') => {
                let start = self.state.pos().clone();

                let mut search_start = arg_context!(start.checked_add(1))?;
                if search_start > *self.data.location().finish() {
                    liab!(
                        "Could not find '\"' for '\"' at [{}]",
                        row_col_position(&self.data.source()[..=start])?,
                    );
                }

                let mut finish;

                loop {
                    if let Some(local_position) = memchr(
                        b'"',
                        &self.data.source()[search_start..=*self.data.location.finish()],
                    ) {
                        finish = search_start + local_position;

                        let mut backslash_counter = 0;

                        let mut i = finish;

                        loop {
                            if i > 0 {
                                i -= 1;

                                if *arg_context!(self.data.source().get(i))? == b'\\' {
                                    backslash_counter += 1;
                                } else {
                                    break;
                                }
                            } else {
                                break;
                            }
                        }

                        if backslash_counter % 2 == 0 {
                            self.state.pos_update(finish);
                            self.state.byt_update(Some(b'"'));

                            break;
                        } else {
                            search_start = arg_context!(finish.checked_add(1))?;
                        }
                    } else {
                        liab!(
                            "Could not find '\"' for '\"' at [{}]",
                            row_col_position(&self.data.source()[..=start])?,
                        );
                    }
                }

                Token::new(TokenKind::String, Location::new(start, finish))
            }
            _ => {
                let sequence_start = self.state.pos().clone();

                loop {
                    if let Some(next_byt) = arg_context!(self.peek_next_char())? {
                        let terminators = &[b':', b',', b' ', b'\n', b'\t', b'\r'];
                        if terminators.contains(&next_byt) {
                            break;
                        } else {
                            arg_context!(self.read_next_char())?;
                        }
                    } else {
                        break;
                    }
                }

                let sequence_finish = self.state.pos().clone();

                Token::new(
                    TokenKind::Sequence,
                    Location::new(sequence_start, sequence_finish),
                )
            }
        };

        arg_context!(self.read_next_char())?;

        anyhow::Ok(token)
    }

    pub fn expect_kinds(&mut self, source: &'a [u8], kinds: &[TokenKind]) -> anyhow::Result<Token> {
        let token = arg_context!(self.next_token())?;
        if !kinds.contains(token.kind()) {
            liab!(
                "Expected one of {:?}, but got {:?} at [{}]",
                kinds,
                token.kind(),
                arg_context!(row_col_position(&source[..=*token.location().start()]))?
            );
        }

        anyhow::Ok(token)
    }
}

pub fn get_object_items<'a>(source: &'a [u8], location: Location) -> anyhow::Result<Vec<Location>> {
    let mut items = Vec::default();

    let mut lexer = Lexer::new(source, location);

    loop {
        let key_token = arg_context!(lexer.expect_kinds(
            &source,
            &[TokenKind::String, TokenKind::Sequence, TokenKind::LastChar]
        ))?;

        if key_token.kind() == &TokenKind::LastChar {
            break;
        }

        arg_context!(lexer.expect_kinds(&source, &[TokenKind::Colon]))?;

        let value_token = arg_context!(lexer.expect_kinds(
            &source,
            &[
                TokenKind::String,
                TokenKind::Sequence,
                TokenKind::Object,
                TokenKind::Array
            ]
        ))?;

        items.push(Location::new(
            key_token.location().start().clone(),
            value_token.location().finish().clone(),
        ));

        if arg_context!(lexer.expect_kinds(&source, &[TokenKind::Comma, TokenKind::LastChar]))?
            .kind()
            == &TokenKind::LastChar
        {
            break;
        }
    }

    anyhow::Ok(items)
}

pub fn get_array_items<'a>(source: &'a [u8], location: Location) -> anyhow::Result<Vec<Location>> {
    let mut items = Vec::default();

    let mut lexer = Lexer::new(source, location);

    loop {
        let value_token = arg_context!(lexer.expect_kinds(
            &source,
            &[
                TokenKind::String,
                TokenKind::Sequence,
                TokenKind::Object,
                TokenKind::Array,
                TokenKind::LastChar
            ]
        ))?;

        if value_token.kind() == &TokenKind::LastChar {
            break;
        }

        items.push(Location::new(
            value_token.location().start().clone(),
            value_token.location().finish().clone(),
        ));

        if arg_context!(lexer.expect_kinds(&source, &[TokenKind::Comma, TokenKind::LastChar]))?
            .kind()
            == &TokenKind::LastChar
        {
            break;
        }
    }

    anyhow::Ok(items)
}

pub fn items_to_vec<'a>(
    source: &'a [u8],
    items: &[Location],
) -> anyhow::Result<VecDeque<(String, Option<Location>)>> {
    let mut vec = VecDeque::new();

    for item in items.iter() {
        let mut lexer = Lexer::new(source, *item);

        let name;
        let location;

        let token_a = lexer.next_token()?;

        if lexer.next_token()?.kind() == &TokenKind::LastChar {
            match token_a.kind() {
                TokenKind::Object => {
                    name = String::from("{...}");
                    location = Some(Location::new(
                        token_a.location().start().clone(),
                        token_a.location().finish().clone(),
                    ));
                }
                TokenKind::Array => {
                    name = String::from("[...]");
                    location = Some(Location::new(
                        token_a.location().start().clone(),
                        token_a.location().finish().clone(),
                    ));
                }
                TokenKind::String => {
                    let name_start =
                        arg_context!(token_a.location().start().checked_add(1))?.clone();
                    let name_finish =
                        arg_context!(token_a.location().finish().checked_sub(1))?.clone();
                    name = if name_start <= name_finish && name_finish < source.len() {
                        arg_context!(from_utf8(&source[name_start..=name_finish]))?.to_string()
                    } else {
                        String::default()
                    };
                    location = None;
                }
                TokenKind::Sequence => {
                    name = arg_context!(from_utf8(&source.slice(token_a.location())))?.to_string();
                    location = None;
                }
                _ => {
                    liab!("Unexpected token: {:?}", token_a)
                }
            }
        } else {
            let token_b = lexer.expect_kinds(
                &source,
                &[
                    TokenKind::String,
                    TokenKind::Sequence,
                    TokenKind::Object,
                    TokenKind::Array,
                ],
            )?;

            let (name_start, name_finish) = match token_a.kind() {
                &TokenKind::String => (
                    arg_context!(token_a.location().start().checked_add(1))?.clone(),
                    arg_context!(token_a.location().finish().checked_sub(1))?.clone(),
                ),
                &TokenKind::Sequence => (
                    token_a.location().start().clone(),
                    token_a.location().finish().clone(),
                ),
                _ => {
                    liab!("Unexpected token: {:?}", token_a)
                }
            };

            name = if name_start <= name_finish && name_finish < source.len() {
                arg_context!(from_utf8(&source[name_start..=name_finish]))?.to_string()
            } else {
                String::default()
            };

            location = Some(Location::new(
                token_b.location().start().clone(),
                token_b.location().finish().clone(),
            ));
        }

        vec.push_back((name, location));
    }

    anyhow::Ok(vec)
}
