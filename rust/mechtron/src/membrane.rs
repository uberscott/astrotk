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
use mechtron_core::error::Error;

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
    pub fn wasm_init();

    pub fn mechtronium_log( type_ptr: *const u8, type_len: i32, message_ptr: *const u8, message_len: i32);
    pub fn mechtronium_cache( artifact_id: i32 );
    pub fn mechtronium_load( artifact_id: i32 )->i32;
}

pub fn log( log_type: &str, string: &str ){
    unsafe
        {
            mechtronium_log(log_type.as_ptr(), log_type.len() as _, string.as_ptr(), string.len() as _ );
        }
}

pub fn mechtronium_read_string(buffer_id: i32) -> Result<String, Error>
{
    let buffers = BUFFERS.read()?;
    let buffer = buffers.get(&buffer_id);
    if buffer.is_none()
    {
        return Err("could not find string buffer".into());
    }
    let buffer_info = buffer.unwrap();

    unsafe {
        Ok(String::from_raw_parts(buffer_info.ptr.load(Ordering::Relaxed), buffer_info.len.clone() as _, buffer_info.len.clone() as _))
    }
}

pub fn mechtronium_consume_string(buffer_id: i32) -> Result<String, Error>
{
    let mut buffers = BUFFERS.write()?;
    let buffer = buffers.remove(&buffer_id);
    if buffer.is_none()
    {
        return Err("could not find string buffer".into());
    }
    let buffer_info = buffer.unwrap();

    unsafe {
        Ok(String::from_raw_parts(buffer_info.ptr.load(Ordering::Relaxed), buffer_info.len.clone() as _, buffer_info.len.clone() as _))
    }
}

pub fn mechtronium_read_buffer(buffer_id: i32) -> Result<Vec<u8>, Error>
{
    let buffers = BUFFERS.read()?;
    let buffer = buffers.get(&buffer_id);
    if buffer.is_none()
    {
        return Err("could not find string buffer".into());
    }
    let buffer_info = buffer.unwrap();

    unsafe {
        Ok(Vec::from_raw_parts(buffer_info.ptr.load(Ordering::Relaxed), buffer_info.len.clone() as _, buffer_info.len.clone() as _))
    }
}

pub fn mechtronium_consume_buffer(buffer_id: i32) -> Result<Vec<u8>, Error>
{
    let mut buffers = BUFFERS.write()?;
    let buffer = buffers.remove(&buffer_id);
    if buffer.is_none()
    {
        return Err("could not find string buffer".into());
    }
    let buffer_info = buffer.unwrap();

    unsafe {
        Ok(Vec::from_raw_parts(buffer_info.ptr.load(Ordering::Relaxed), buffer_info.len.clone() as _, buffer_info.len.clone() as _))
    }
}


fn wasm_alloc(len: i32) -> *mut u8 {
    let rtn = unsafe {
        let align = mem::align_of::<u8>();
        let size = mem::size_of::<u8>();
        let layout = Layout::from_size_align(size * (len as usize), align).unwrap();
        alloc(layout)
    };
    rtn
}

fn wasm_dealloc(ptr: *mut u8, len: i32) {
    unsafe {
        let align = mem::align_of::<u8>();
        let size = mem::size_of::<u8>();
        let layout = Layout::from_size_align(size * (len as usize), align).unwrap();
        dealloc(ptr, layout)
    };
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

pub fn wasm_write_string(mut string: String) -> i32{
    let rtn = wasm_assign_buffer(string.as_mut_ptr(), string.len() as _ );
    mem::forget(string);
    rtn
}



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



