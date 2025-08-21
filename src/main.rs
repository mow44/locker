use anyhow::anyhow;
use clap::Parser;
use locker::types::Step;
use memmap2::Mmap;
use ratatui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};
use std::{fs::File, io::stderr, path::PathBuf};

use wrap_context::{arg_context, raw_context, wohyna};

mod app;
mod column_model;
mod column_view;
mod directional_constraint;
mod event;
mod handler;
mod lexer;
mod node;
mod page_model;
mod page_view;
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

use crate::{app::App, event::EventHandler, tui::Tui, utils::DEBUG_PRINT_LIMIT};

/// JSON reader
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    #[arg(value_name = "FILE")]
    file: PathBuf,

    /// Initial cursor position. Specified as a sequence of positive indices separated by commas, for example: "[0,0,0]", "[3,0,1]", or "4,1,1,0".
    #[arg(short, long, value_parser = path_parser, default_value = "[0]")]
    path: Box<[Step]>,

    /// Controls the amount of function argument info shown in tracebacks after a crash. Only useful when debugging.
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

// fn count_bytes(needle: u8, haystack: &[u8]) -> anyhow::Result<usize> {
//     if let Some(position) = memchr(needle, haystack) {
//         if arg_context!(position.checked_add(1))? >= haystack.len() {
//             return anyhow::Ok(1);
//         } else {
//             return anyhow::Ok(
//                 1 + raw_context!(count_bytes(
//                     needle,
//                     &haystack[arg_context!(position.checked_add(1))?..]
//                 ))?,
//             );
//         }
//     } else {
//         return anyhow::Ok(0);
//     }
// }

// fn count_open_square(counted_before: usize, haystack: &[u8]) -> anyhow::Result<usize> {
//     if let Some(position) = memchr(b'[', haystack) {
//         if arg_context!(position.checked_add(1))? >= haystack.len() {
//             return anyhow::Ok(1);
//         }

//         let quote_counter =
//             arg_context!(count_bytes(b'"', &haystack[..position]))? + counted_before;

//         if quote_counter % 2 == 0 {
//             return anyhow::Ok(
//                 1 + raw_context!(count_open_square(
//                     quote_counter,
//                     &haystack[arg_context!(position.checked_add(1))?..]
//                 ))?,
//             );
//         } else {
//             return anyhow::Ok(
//                 0 + raw_context!(count_open_square(
//                     quote_counter,
//                     &haystack[arg_context!(position.checked_add(1))?..]
//                 ))?,
//             );
//         }
//     } else {
//         return anyhow::Ok(0);
//     }
// }

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // let haystack = b"[a[b]";

    // let mut position: usize = 0;
    // let mut current_byte = Some(&b'[');

    // let mut square_start = position;
    // let mut square_end = 0;
    // let mut search_start = arg_context!(square_start.checked_add(1))?;
    // if search_start >= haystack.len() {
    //     liab!("overflow");
    // }

    // let mut counter = 0;
    // let mut open_quote_counted = 0;

    // loop {
    //     if let Some(pos) = memchr(b']', &haystack[search_start..]) {
    //         square_end = search_start + pos;

    //         position = square_end;
    //         current_byte = Some(&b']');

    //         open_quote_counted +=
    //             arg_context!(count_bytes(b'"', &haystack[search_start..square_end]))?;
    //         if open_quote_counted % 2 != 0 {
    //             search_start = square_end + 1;
    //             continue;
    //         }

    //         counter += arg_context!(count_open_square(0, &haystack[search_start..square_end]))?;

    //         if counter == 0 {
    //             break;
    //         } else {
    //             counter -= 1;
    //             search_start = square_end + 1;
    //         }
    //     } else {
    //         liab!("Not found ]");
    //     }
    // }

    // liab!("{:?} - {:?}", square_start, square_end);

    // return anyhow::Ok(());

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

    // let mut buf_reader = BufReader::new(arg_context!(File::open(&args.file)).map_err(|err| {
    //     exit(&mut tui);
    //     err
    // })?);
    // let mut raw_value = String::default();
    // arg_context!(buf_reader.read_to_string(&mut raw_value)).map_err(|err| {
    //     exit(&mut tui);
    //     err
    // })?;
    let file = arg_context!(File::open(&args.file)).map_err(|err| {
        exit(&mut tui);
        err
    })?;
    let mmap = unsafe {
        arg_context!(Mmap::map(&file)).map_err(|err| {
            exit(&mut tui);
            err
        })?
    };
    let bytes = &mmap[..];

    let mut app = arg_context!(App::new(
        terminal_size,
        &args.file,
        bytes,
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
