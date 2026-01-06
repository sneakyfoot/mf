use super::App;
use ratatui::{
    DefaultTerminal, Frame,
    crossterm::event::{self, Event, KeyCode, KeyEventKind},
    layout::{Constraint, Flex, Layout, Rect},
    style::Stylize,
    widgets::{Block, Clear, Paragraph, Wrap},
};

pub enum ConfirmAction {
    CancelJob { controller: String },
    //CheckoutNode { schedulable: bool },
}

impl App {
    pub fn show_confirmation(&mut self, frame: &mut Frame) {
        if self.confirmation_popup {
            let area = frame.area();
            let block = Block::bordered().title("Popup");
            let area = popup_area(area, 60, 20);
            frame.render_widget(Clear, area); //this clears out the background
            frame.render_widget(block, area);
        }
    }
    pub fn yes_key(&mut self) {
        if let Some(action) = self.pending_confirmation.take() {
            match action {
                ConfirmAction::CancelJob { controller } => self.delete_key(),
            }
        }
        self.confirmation_popup = false;
        self.pending_confirmation = None;
    }
    pub fn no_key(&mut self) {
        self.confirmation_popup = false;
        self.pending_confirmation = None;
    }
}
/// helper function to create a centered rect using up certain percentage of the available rect `r`
fn popup_area(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);
    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);
    area
}
