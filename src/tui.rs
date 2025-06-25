use crossterm::terminal;
use ratatui::{
    backend::Backend,
    crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen},
    Terminal,
};
use std::io;

use wrap_context::{arg_context, raw_context};

use crate::{app::App, event::EventHandler, render::Render};

#[derive(Debug)]
pub struct Tui<B: Backend> {
    terminal: Terminal<B>,
    pub events: EventHandler,
}

impl<B: Backend> Tui<B> {
    pub fn new(terminal: Terminal<B>, events: EventHandler) -> Self {
        Self { terminal, events }
    }

    pub fn init(&mut self) -> anyhow::Result<()> {
        arg_context!(crossterm::execute!(io::stderr(), EnterAlternateScreen))?;
        arg_context!(terminal::enable_raw_mode())?;

        let panic_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |panic_info| {
            let _ = Self::reset();
            panic_hook(panic_info);
        }));

        anyhow::Ok(())
    }

    pub fn draw(&mut self, app: &mut App) -> anyhow::Result<()> {
        raw_context!(self.terminal.draw(|frame| app.render(frame)))?;
        anyhow::Ok(())
    }

    fn reset() -> anyhow::Result<()> {
        arg_context!(crossterm::execute!(io::stderr(), LeaveAlternateScreen))?;
        arg_context!(terminal::disable_raw_mode())?;
        anyhow::Ok(())
    }

    pub fn exit(&mut self) -> anyhow::Result<()> {
        arg_context!(Self::reset())?;
        arg_context!(self.terminal.show_cursor())?;
        anyhow::Ok(())
    }
}
