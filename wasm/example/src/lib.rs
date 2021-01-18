mod utils;

use wasm_bindgen::prelude::*;
use wasm_bindgen::__rt::std::collections::HashMap;
use bytes::{BytesMut, BufMut, Buf, Bytes};
use std::sync::Mutex;
use std::sync::atomic::{{AtomicUsize,Ordering}};

#[macro_use]
extern crate lazy_static;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

extern "C"
{
    fn host_alloc_buffer( len: i32 ) -> i32;
    fn host_append_to_buffer( id: i32, value: i32 );
    fn host_dealloc_buffer( id: i32 );
    fn host_log( buffer_id: i32 );
   // fn the_big_func(ptr:i32);
}

lazy_static! {
    static ref buffers: Mutex<HashMap<i32, BytesMut>> = {
        let mut m = HashMap::new();
        Mutex::new(m)
    };

    static ref buffer_index : AtomicUsize = { AtomicUsize::new(0) };
}


pub fn logme( str: &str )
{
    unsafe
    {
      let buffer_id = host_alloc_buffer(str.len() as _);
      for b in str.as_bytes()
      {
          host_append_to_buffer(buffer_id,*b as i32);
      }
      host_log(buffer_id);
      host_dealloc_buffer(buffer_id);
    }
}

#[wasm_bindgen]
pub fn alloc_buffer( len: i32 ) -> i32 {
    let mut buffer = BytesMut::with_capacity(len as usize );

    let mut buffer_id = buffer_index.fetch_add(1,Ordering::Relaxed) as i32;
    buffers.lock().unwrap().insert(buffer_id,buffer);

    return buffer_id;
}

#[wasm_bindgen]
pub fn append_to_buffer( id: i32, value: i32 )
{
    let b= buffers.lock();
    let mut unwrapped = b.unwrap();
    let option = unwrapped.get_mut(&id);
    let buffer = option.unwrap();
    buffer.put_u8(value as u8);
}

#[wasm_bindgen]
pub fn dealloc_buffer( id: i32 )
{
   buffers.lock().unwrap().remove(&id);
}

pub fn consume_buffer( buffer_id: i32 ) -> Option<Bytes>
{
    let option = buffers.lock().unwrap().remove(&buffer_id);
    match option {
        None => {return Option::None;}
        Some(buffer) => {
            return Option::Some(buffer.freeze());
        }
    }
}


/*
#[wasm_bindgen]
pub fn read_from_buffer( id: i32, index: i32 )->i32
{
    let b= buffers.lock();
    let mut unwrapped = b.unwrap();
    let option = unwrapped.get_mut(&id);
    let mut buffer = option.unwrap();
    return buffer[index as usize] as i32;
}
 */


#[wasm_bindgen]
pub fn init()
{
    logme( &"hello from WebAssembly XOXO" );
}
