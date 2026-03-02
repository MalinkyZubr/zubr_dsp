#![feature(trait_alias)]
#![feature(mpmc_channel, portable_simd, test)]
#![feature(let_chains)]
#![feature(adt_const_params)]
#![allow(dead_code)]

use crate::pipeline::orchestration_layer::logging::init_stdout_logger;
use log::Level;
use std::sync::OnceLock;

pub mod pipeline;

static INIT: OnceLock<()> = OnceLock::new();
pub fn initiate_pipeline(log_level: Level) {
    match INIT.get() {
        None => {
            init_stdout_logger(log_level).expect("Failed to initialize logger");
            let _ = INIT.set(());
        }
        _ => (),
    }
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
