use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use no_proto::error::NP_Error;

use no_proto::pointer::{NP_Value, NP_Scalar};

use crate::data::{AstroTK};
use wasmer::{Instance, Module, imports, WasmPtr, Array, ValType, Value, Val, Function, FunctionType, ImportObject, Resolver,WasmerEnv};
use anyhow::Result;
use std::sync::{Mutex, Arc};
use std::pin::Pin;
use std::sync::atomic::{{AtomicUsize,Ordering}};
use std::ops::Deref;
use std::borrow::{Borrow, BorrowMut};
use std::fmt::Debug;
use serde::export::Formatter;
use std::fmt;
use std::rc::Rc;
use bytes::{Bytes,BytesMut,BufMut,Buf};
use std::string::FromUtf8Error;


pub struct WasmBinder<'a>
{
    module: &'a Module,
    guest: Guest,
    host: Arc<Mutex<Host>>
}

struct Guest
{
    instance: Instance
}

struct Host
{
    buffer_map: HashMap<i32,BytesMut>,
    buffer_index: i32
}

impl Host
{
    fn new() -> Self
    {
        Host {
            buffer_map: HashMap::new(),
            buffer_index: 0
        }
    }

    fn consume_string( &mut self, buffer_id: i32 ) -> Option<String>
    {
        let option = self.consume_buffer(buffer_id);
        match option {
            None => {
                return Option::None;
            }
            Some(bytes) => {
                let option = String::from_utf8(bytes.to_vec());
                match option {
                    Ok(string) => {
                        return Option::Some(string);
                    }
                    Err(e) => {
                        return Option::None;
                    }
                }
            }
        }
    }

    fn log(&mut self, buffer_id: i32)
    {
        let option = self.consume_string(buffer_id);
        match option {
            None => {
                println!("host log: could not acquire buffer");
            }
            Some(string) => {
                println!("{}",string);

            }
        }
    }
}

impl <'a> WasmBinder<'a>
{
    pub fn new( module: &'a Module ) -> Result<Self>
    {

        let host = Arc::new( Mutex::new( Host::new() ));
        #[derive(WasmerEnv, Clone)]
        struct Env {
            host: Arc<Mutex<Host>>
        }

        let imports = imports!{ "env"=>{
        "host_alloc_buffer"=>Function::new_native_with_env(module.store(),Env{host:host.clone()},|env:&Env,len:i32| {
                 return env.host.lock().unwrap().alloc_buffer(len);
            } ),
        "host_append_to_buffer"=>Function::new_native_with_env(module.store(),Env{host:host.clone()},|env:&Env,id:i32,value:i32| {
                 env.host.lock().unwrap().append_to_buffer(id,value as u8);
            } ),
        "host_dealloc_buffer"=>Function::new_native_with_env(module.store(),Env{host:host.clone()},|env:&Env,id:i32| {
                 env.host.lock().unwrap().dealloc_buffer(id);
            } ),
        "host_log"=>Function::new_native_with_env(module.store(),Env{host:host.clone()},|env:&Env,buffer_id:i32| {
                 env.host.lock().unwrap().log(buffer_id);
            } ),
        } };



        let instance = Instance::new( module, &imports )?;

        let guest = Guest { instance: instance };

        let binder = WasmBinder{ module: &module, guest: guest, host:host };

        binder.init();
        return Ok(binder);
    }


    pub fn init(&self)
    {
        let init = self.guest.instance.exports.get_function("init").unwrap();
        init.call(&[]);
    }

}

impl <'a> WasmBinder <'a>
{
    fn log( &self, ptr: i32, len: i32 )
    {}
}


pub trait Buffers
{
    fn alloc_buffer(&mut self, len: i32) -> i32;
    fn dealloc_buffer(&mut self,id: i32);
    fn append_to_buffer(&mut self,id: i32, value: u8 );
}

pub trait BufferConsume
{
    fn consume_buffer(&mut self, id: i32) -> Option<Bytes>;
}


impl <'a> Buffers for Guest
{
    fn alloc_buffer(&mut self, len: i32) -> i32 {
        return self.instance.exports.get_native_function::<i32, i32>("alloc_buffer").unwrap().call(len).unwrap();
    }

    fn dealloc_buffer(&mut self, id: i32) {
        self.instance.exports.get_function("dealloc_buffer").unwrap().call(&[Value::I32(id)]);
    }

    fn append_to_buffer(&mut self, id: i32, value: u8) {
        self.instance.exports.get_function("append_to_buffer").unwrap().call(&[Value::I32(id), Value::I32(value as i32)]);
    }
}

impl Buffers for Host
{
    fn alloc_buffer(&mut self, len: i32) -> i32 {
        self.buffer_index = self.buffer_index+1;
        let ptr = self.buffer_index.clone();

        let mut buffer = BytesMut::with_capacity(len as usize );

        self.buffer_map.insert( ptr, buffer );

        return ptr;
    }

    fn dealloc_buffer(&mut self, id: i32) {
        self.buffer_map.remove(&id);
    }

    fn append_to_buffer(&mut self, id: i32, value: u8) {
        let option = self.buffer_map.get_mut(&id);
        let buffer = option.unwrap();
        buffer.put_u8(value as u8)
    }
}

impl BufferConsume for Host
{
    fn consume_buffer(&mut self, id: i32) -> Option<Bytes> {

        let option = self.buffer_map.remove(&id);

        match option {
            None => return Option::None,
            Some(buffer) => {
                return Some(buffer.freeze());
            }
        }
    }
}



