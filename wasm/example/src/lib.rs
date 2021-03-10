#[macro_use]
extern crate wasm_bindgen;

use mechtron::membrane::log;


#[no_mangle]
pub extern "C" fn wasm_init()
{
    log("init","hello from WASM");
}






