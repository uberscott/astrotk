pub mod error;

#[macro_use]
extern crate wasm_bindgen;

#[macro_use]
extern crate lazy_static;


use wasm_bindgen::prelude::*;
use wasm_bindgen::__rt::std::alloc::{Layout,alloc,dealloc};
use wasm_bindgen::__rt::std::mem;
//use mechtron_core::error::Error;
use wasm_bindgen::__rt::core::slice;
use std::borrow::BorrowMut;
use std::sync::RwLock;
use wasm_bindgen::__rt::std::collections::HashMap;
use wasm_bindgen::__rt::std::sync::atomic::{Ordering, AtomicPtr};
use wasm_bindgen::__rt::std::sync::atomic::AtomicI32;
use crate::error::Error;


lazy_static! {
  static ref BUFFERS: RwLock<HashMap<i32,BufferInfo>> = RwLock::new(HashMap::new());
  static ref BUFFER_INDEX: AtomicI32 = AtomicI32::new(0);
}

struct BufferInfo
{
    len: usize,
    ptr: AtomicPtr<u8>
}
extern "C"
{

    fn wasm_init();

    fn mechtronium_log( type_ptr: *const u8, type_len: i32, message_ptr: *const u8, message_len: i32);

    fn mechtronium_cache( ptr: *const u8, len: i32);
}

pub fn log( log_type: &str, string: &str ){
    unsafe
        {
            mechtronium_log(log_type.as_ptr(), log_type.len() as _, string.as_ptr(), string.len() as _ );
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
    log( "wasm-test", str.as_str() );
}

#[wasm_bindgen]
pub fn wasm_cache( key_ptr: *mut u8, key_len: i32, ptr: *mut u8, len: i32)
{
    let key = mechtronium_read_string(key_ptr, key_len );
    let buffer = mechtronium_read_buffer(ptr,len);
}

#[wasm_bindgen]
pub fn wasm_test_cache(ptr: *mut u8, len: i32)
{
    let str = mechtronium_read_string(ptr, len);
    unsafe
        {
            mechtronium_cache(ptr, len);
        }
}

#[wasm_bindgen]
pub fn wasm_get_buffer_ptr(id: i32)->*const u8
{
    let buffer_info = BUFFERS.read();
    let buffer_info = buffer_info.unwrap();
    let buffer_info = buffer_info.get(&id).unwrap();
    buffer_info.ptr.load(Ordering::Relaxed)
}

#[wasm_bindgen]
pub fn wasm_get_buffer_len(id: i32)->i32
{
    let buffer_info = BUFFERS.read();
    let buffer_info = buffer_info.unwrap();
    let buffer_info = buffer_info.get(&id).unwrap();
    buffer_info.len.clone() as _
}

#[wasm_bindgen]
pub fn wasm_dealloc_buffer(id: i32)
{
    let buffer_info = BUFFERS.read();
    let buffer_info = buffer_info.unwrap();
    let buffer_info = buffer_info.get(&id).unwrap();
    let ptr = buffer_info.ptr.load(Ordering::Relaxed);
    wasm_dealloc(ptr, buffer_info.len as _);
}

#[wasm_bindgen]
pub fn wasm_alloc_buffer(len: i32) ->i32
{

    let buffer_id = BUFFER_INDEX.fetch_add(1, Ordering::Relaxed );
    let buffer_ptr = wasm_alloc(len);
    {
        let mut buffers = BUFFERS.write().unwrap();
        let buffer_info = BufferInfo{
            ptr: AtomicPtr::new(buffer_ptr),
            len: len as _
        };
        buffers.insert( buffer_id, buffer_info  );
    }

    buffer_id
}

#[wasm_bindgen]
pub fn wasm_assign_buffer(ptr: *mut u8, len: i32) -> i32{
    let buffer_id = BUFFER_INDEX.fetch_add(1, Ordering::Relaxed );
    let buffer_info = BufferInfo{
        ptr: AtomicPtr::new(ptr),
        len: len as _
    };
    {
        let mut buffers = BUFFERS.write().unwrap();
        buffers.insert( buffer_id, buffer_info  );
    }
    buffer_id
}



/*

#[wasm_bindgen]
pub fn wasm_stateful_invoke(struct_ptr: *mut u8, struct_len: i32,
                            state_buffer_id: i32,
                            method_ptr: *mut u8, method_len: i32,
                            request_buffer_ptr: *mut u8, request_buffer_len: i32) -> i32
{
    let struct_key = mechtronium_read_string(struct_ptr, struct_len);
    let method_key = mechtronium_read_string(method_ptr, method_len);
    let request_buffer = mechtronium_read_buffer(request_buffer_ptr, request_buffer_len);
    let mut state_buffer_info = {
        BUFFERS.write().unwrap().remove(&state_buffer_id).unwrap()
    };

    let mut state_buffer = mechtronium_read_buffer(state_buffer_info.ptr.load(Ordering::Relaxed), state_buffer_info.len as _ );

    let method = {
        let structs = STRUCTS.read().unwrap();
        structs.get( &struct_key ).unwrap().methods.get( &method_key ).unwrap()
    };

    let (response_buffer,mut state_buffer)=match method.invoke(state_buffer, request_buffer)
    {
        Ok((response,state)) => {
            (response,state)
        }
        Err(_) => {
            return -1;
        }
    };


    // do something
    let response_buffer_len = response_buffer.len();
    let response_buffer_id = wasm_alloc_buffer(response_buffer_len as _);
    {
        let mut buffers = BUFFERS.write().unwrap();
        state_buffer_info.ptr.swap( state_buffer.as_mut_ptr(), Ordering::Relaxed );
        state_buffer_info.len = state_buffer.len();
        buffers.insert(state_buffer_id, state_buffer_info);
    }

    mem::forget(state_buffer );
    mem::forget(response_buffer);

    return response_buffer_id
}

 */
