use mechtron::mechtron::{Mechtron, BlankMechtron};
use crate::neutron::Neutron;
use crate::simtron::Simtron;
use mechtron::membrane::log;
use mechtron_common::mechtron::Context;
use mechtron_common::state::State;
use std::rc::Rc;
use std::cell::{Cell, RefCell};
use mechtron_common::logger::replace_logger;
use mechtron::WasmLogger;

#[macro_use]
extern crate mechtron_common;

pub mod neutron;
pub mod simtron;

#[no_mangle]
pub extern "C" fn mechtron_init()
{
    log("core", "core initialized.");
}

#[no_mangle]
pub extern "C" fn mechtron(kind: &str, context: Context, state: Rc<RefCell<Option<Box<State>>>> )->Box<dyn Mechtron>
{
    log("typal", kind);
    match kind{
        "Neutron"=>Box::new(Neutron::new(context, state)),
        "Simtron"=>Box::new(Simtron::new(context, state)),
        _ => Box::new(BlankMechtron::new(context, state))
    }
}