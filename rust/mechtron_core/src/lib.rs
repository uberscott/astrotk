use mechtron::mechtron::{Context, Mechtron, BlankMechtron};
use crate::neutron::Neutron;
use crate::simtron::Simtron;
use mechtron::membrane::log;

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

pub extern "C" fn mechtron(kind: &str, context: Context )->Box<dyn Mechtron>
{
    match kind{
        "Neutron"=>Box::new(Neutron::new(context.clone())),
        "Simtron"=>Box::new(Simtron::new(context.clone())),
        _ => Box::new(BlankMechtron::new(context.clone() ))
    }
}