mod hello_world;

#[macro_use]
extern crate wasm_bindgen;

use mechtron::membrane::log;
use mechtron::mechtron::{BlankMechtron, Mechtron};
use std::cell::RefCell;
use std::rc::Rc;
use mechtron_common::mechtron::Context;
use mechtron_common::state::State;
use crate::hello_world::HelloWorldMechtron;

#[no_mangle]
pub extern "C" fn mechtron_init()
{
    log("core", "HelloWorld initialized.");
}

#[no_mangle]
pub extern "C" fn mechtron(kind: &str, context: Context, state: Rc<RefCell<Option<Box<State>>>> )->Box<dyn Mechtron>
{
    log("typal", kind);
    match kind{
        "HelloWorldMechtron"=>Box::new(HelloWorldMechtron::new(context, state)),
        _ => Box::new(BlankMechtron::new(context, state))
    }
}