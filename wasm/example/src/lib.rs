#[macro_use]
extern crate wasm_bindgen;

use mechtron_wasm_membrane_guest::{log};

#[no_mangle]
pub extern "C" fn wasm_init()
{
    log("init","hello from WASM");
}



