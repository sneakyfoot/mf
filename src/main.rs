// use color_eyre::Result;
mod app;
mod data;
mod k8s;
use crate::app::App;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let terminal = ratatui::init();
    let app_result = App::new()?.run(terminal);
    ratatui::restore();
    app_result
}
