#[macro_use]
extern crate wasm_bindgen;

use wasm_bindgen::prelude::*;
use wasm_bindgen::__rt::std::alloc::{Layout,alloc,dealloc};
use wasm_bindgen::__rt::std::mem;
//use mechtron_core::error::Error;
use wasm_bindgen::__rt::core::slice;
use std::borrow::BorrowMut;

extern "C"
{
    fn mechtronium_log( ptr: *const u8, len: i32);
    fn mechtronium_cache( ptr: *const u8, len: i32);
}

pub fn log( string: &str ){
    unsafe
        {
            mechtronium_log(string.as_ptr(), string.len() as _ );
        }
}

fn mechtronium_read_string( ptr: *mut u8, len: i32 ) -> String
{
    unsafe{
        String::from_raw_parts( ptr, len.clone() as _, len as _ )
    }
}

fn mechtronium_read_buffer( ptr: *mut u8, len: i32 ) -> Vec<u8>
{
    unsafe{
        Vec::from_raw_parts( ptr, len.clone() as _, len as _ )
    }
}

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

#[wasm_bindgen]
pub fn wasm_dealloc(ptr: *mut u8, len: i32) {
    unsafe {
        let align = mem::align_of::<u8>();
        let size = mem::size_of::<u8>();
        let layout = Layout::from_size_align(size*(len as usize),align).unwrap();
        dealloc(ptr, layout)
    };
}

#[wasm_bindgen]
pub fn wasm_test_log( ptr: *mut u8, len: i32 )
{
    let str = mechtronium_read_string(ptr, len);
    log( str.as_str() );
}

#[wasm_bindgen]
pub fn wasm_cache( key_ptr: *mut u8, key_len: i32, ptr: *mut u8, len: i32)
{
    let key = mechtronium_read_string(key_ptr, key_len );
    let buffer = mechtronium_read_buffer(ptr,len);
    log(format!("CACHE KEY {}",key).as_str() );
}

#[wasm_bindgen]
pub fn wasm_test_cache( ptr: *mut u8, len: i32 )
{
    let str = mechtronium_read_string(ptr, len);
    unsafe
    {
     mechtronium_cache(ptr,len);
    /*        let (ptr,len) = mechtronium_cache(ptr,len);
            let mut ptr = ptr.as_ref().unwrap().raw;
            let string = mechtronium_read_string(ptr,len);
            log( format!("WASM CACHED : {}", string).as_str() );

     */
        }
    }
