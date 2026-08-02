#![warn(clippy::all, rust_2018_idioms)]

mod app;
mod sensors_ui;
mod measurements_plot_ui;
mod send_requests;
mod model;
pub use app::SchiliApp;
