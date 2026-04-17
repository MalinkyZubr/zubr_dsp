#![feature(trait_alias)]
#![feature(lazy_type_alias)]
#![feature(mpmc_channel, portable_simd, test)]
#![feature(adt_const_params)]
#![allow(dead_code)]
#![feature(generic_const_exprs)]
#![feature(array_try_from_fn)]
extern crate core;

use crate::pipeline::orchestration_layer::logging::init_stdout_logger;
use log::Level;
use std::sync::Once;

pub mod dsp;
pub mod pipeline;
pub mod general;

static INIT: Once = Once::new();

pub fn initiate_pipeline(log_level: Level) {
    INIT.call_once(|| if let Err(_) = init_stdout_logger(log_level) {});
}

// use pipeline::orchestration_layer::logging::initialize_logger;
//
//
// mod pipeline;
// mod ddp;
// mod dsp;
// mod general;
// mod tests;
// //mod frontend;
//
// use color_eyre::Result;
// use ratatui::{
//     style::Stylize
//
//     ,
//     widgets::Widget
//     ,
// };
// use strum::IntoEnumIterator;
//
fn main() -> Result<(), ()> {
    Ok(())
}
