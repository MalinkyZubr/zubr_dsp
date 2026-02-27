#![feature(trait_alias)]
#![feature(mpmc_channel, portable_simd, test)]
#![feature(let_chains)]
#![feature(adt_const_params)]
#![allow(dead_code)]

use log::Level;
use crate::pipeline::orchestration_layer::logging::{init_default_logger, init_stdout_logger};

pub mod pipeline;


pub fn initiate_pipeline(log_level: Level) {
    init_stdout_logger(log_level).expect("Failed to initialize logger");
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
