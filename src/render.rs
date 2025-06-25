use ratatui::Frame;

pub trait Render {
    fn render(&mut self, frame: &mut Frame);
}
