#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate wasm_bindgen;

use wasm_bindgen::prelude::*;
use wasm_bindgen::__rt::std::mem;
use wasm_bindgen::__rt::std::alloc::{alloc,Layout};

pub mod buffers;
pub mod cache;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
pub fn wasm_alloc(len: i32) -> *mut u8 {
    let rtn = unsafe {
        let align = mem::align_of::<u8>();
        let size = mem::size_of::<u8>();
        let layout = Layout::from_size_align(size*(len as usize),align).unwrap();
        alloc(layout)
    };
    rtn
}

