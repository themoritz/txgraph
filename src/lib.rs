#![warn(clippy::all, rust_2018_idioms)]

mod annotations;
mod app;
mod bezier;
mod bitcoin;
mod client;
mod components;
mod export;
mod flight;
mod framerate;
mod graph;
mod layout;
mod loading;
mod modal;
mod notifications;
mod platform;
mod projects;
mod style;
mod transform;
mod widgets;
pub use app::App;
