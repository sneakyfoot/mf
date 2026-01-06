use super::App;

use ratatui::{
    Frame,
    layout::{Constraint, Flex, Layout, Rect},
    widgets::{Block, Clear, Paragraph},
};

pub enum ConfirmAction {
    CancelJob { controller: String },
    CheckoutNode { schedulable: bool },
}

impl App {
    pub fn show_confirmation(&mut self, frame: &mut Frame) {
        if self.confirmation_popup {
            let area = frame.area();
            let block = Block::bordered().title("Confirmation");
            let text = "\nAre you sure?\n\n(y/n)";
            let content = Paragraph::new(text).centered().block(block);
            let area = popup_area(area, 60, 20);
            frame.render_widget(Clear, area);
            frame.render_widget(content, area);
        }
    }

    pub fn yes_key(&mut self) {
        self.confirmation_popup = false;
        if let Some(action) = self.pending_confirmation.take() {
            match action {
                ConfirmAction::CancelJob { controller } => self.run_cancel_jobs(controller),
                ConfirmAction::CheckoutNode { schedulable } => self.run_checkout(schedulable),
            }
        }
    }

    pub fn no_key(&mut self) {
        self.confirmation_popup = false;
        self.pending_confirmation = None;
    }
}

fn popup_area(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);
    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);
    area
}
