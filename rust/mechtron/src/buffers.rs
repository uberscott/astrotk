use wasm_bindgen::prelude::*;
use std::{ptr, mem};
use std::alloc::{Layout, System, alloc};

extern "C"
{
    fn mechtronium_buffer_scan( buffer_id: i32, len: i32 );
}




