use anyhow::anyhow;
use clap::Parser;
use locker::types::Step;
use ratatui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};
use serde_json::value::RawValue;
use std::{
    fs::File,
    io::{stderr, BufReader},
    path::PathBuf,
};

use wrap_context::{arg_context, raw_context, wohyna};

mod app;
mod column_model;
mod column_view;
mod directional_constraint;
mod event;
mod handler;
mod node;
mod page_model;
mod page_view;
mod paginated_map;
mod paginated_vec;
mod paginator;
mod preferences;
mod render;
mod table_model;
mod table_view;
mod textline_model;
mod textline_view;
mod tui;
mod types;
mod utils;

use crate::{app::App, event::EventHandler, tui::Tui, types::DEBUG_PRINT_LIMIT};

/// JSON reader
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// File
    #[arg(value_name = "FILE")]
    file: PathBuf,

    /// Initial path
    #[arg(short, long, value_parser = path_parser, default_value = "[0]")]
    path: Box<[Step]>,

    /// Debug print limit
    #[arg(short, long, default_value = "1")]
    debug_print_limit: usize,
}

fn path_parser(value: &str) -> anyhow::Result<Box<[Step]>> {
    let value = value.trim_start_matches('[').trim_end_matches(']');

    let steps = raw_context!(value
        .split(',')
        .map(|word| raw_context!(word.trim().parse::<Step>()))
        .collect::<anyhow::Result<Box<[Step]>>>())?;

    anyhow::Ok(steps)
}

fn exit<B: Backend>(tui: &mut Tui<B>) {
    if let Err(err) = tui.exit() {
        eprintln!(
            "Failed to restore terminal. Run `reset` / `stty sane` or restart your terminal to recover: {}",
            err
        );
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    raw_context!(DEBUG_PRINT_LIMIT
        .set(args.debug_print_limit)
        .map_err(|err| wohyna!("Second initialization with value: {:?}", err)))?;

    let backend = CrosstermBackend::new(stderr());
    let terminal = raw_context!(Terminal::new(backend))?;
    let terminal_size = arg_context!(terminal.size())?;
    let events = EventHandler::new(250);

    let mut tui = Tui::new(terminal, events);
    arg_context!(tui.init()).map_err(|err| {
        exit(&mut tui);
        err
    })?;

    let mut bufreader = BufReader::new(arg_context!(File::open(&args.file)).map_err(|err| {
        exit(&mut tui);
        err
    })?);
    let raw_value: Box<RawValue> =
        arg_context!(serde_json::from_reader(&mut bufreader)).map_err(|err| {
            exit(&mut tui);
            err
        })?;

    let mut app = arg_context!(App::new(
        terminal_size,
        &args.file,
        &raw_value,
        args.path.clone()
    ))
    .map_err(|err| {
        exit(&mut tui);
        err
    })?;

    arg_context!(app.run(&mut tui).await).map_err(|err| {
        exit(&mut tui);
        err
    })?;

    exit(&mut tui);

    arg_context!(app.print())?;

    anyhow::Ok(())
}
