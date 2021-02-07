use bytes::{Buf, BufMut, Bytes, BytesMut};
use mechtron_common::artifact::{Artifact, ArtifactRepository};
use mechtron_common::mechtron_config::MechtronConfig;
use mechtron_common::buffers::BufferFactories;
use mechtron_common::message::Message;
use no_proto::buffer::NP_Buffer;
use no_proto::buffer_ro::NP_Buffer_RO;
use no_proto::error::NP_Error;
use no_proto::pointer::{NP_Scalar, NP_Value};
use std::any::Any;
use std::borrow::{Borrow, BorrowMut};
use std::cell::RefCell;
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::fmt::Debug;
use std::ops::Deref;
use std::pin::Pin;
use std::rc::Rc;
use std::string::FromUtf8Error;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{{AtomicUsize, Ordering}};
use wasmer::{Array, Function, FunctionType, ImportObject, imports, Instance, Module, Resolver, Val, ValType, Value, WasmerEnv, WasmPtr};


pub struct WasmBinder<'a>
{
    module: &'a Module,
    guest: WasmGuest,
    host: Arc<Mutex<WasmHost>>
}

struct WasmGuest
{
    instance: Instance
}

struct WasmHost
{
    buffer_map: HashMap<i32,BytesMut>,
    buffer_index: i32,
    content_buffer_id: Option<i32>,
    messages_buffer_id: Option<i32>
}

impl WasmHost
{
    fn new() -> Self
    {
        WasmHost {
            buffer_map: HashMap::new(),
            buffer_index: 0,
            content_buffer_id: Option::None,
            messages_buffer_id: Option::None
        }
    }

    fn consume_string( &mut self, buffer_id: i32 ) -> Option<String>
    {
        let option = self.consume_buffer(buffer_id);
        return match option {
            None => {
                Option::None
            }
            Some(bytes) => {
                let option = String::from_utf8(bytes.to_vec());
                match option {
                    Ok(string) => {
                        Option::Some(string)
                    }
                    Err(e) => {
                        Option::None
                    }
                }
            }
        }
    }

    fn has_messages( &self ) -> bool
    {
        return self.messages_buffer_id.is_some();
    }

    fn consume_content( &mut self ) -> Result<Bytes,Box<std::error::Error>>
    {
        let content_buffer_id_option = self.content_buffer_id;
        if content_buffer_id_option.is_none()
        {
            return Err("content buffer was not created by wasm guest".into());
        }
        let content_buffer_option = self.consume_buffer(content_buffer_id_option.unwrap());
        if content_buffer_option.is_none()
        {
            return Err("content buffer is not available".into());
        }
        let content_buffer = content_buffer_option.unwrap();
        self.content_buffer_id = Option::None;
        return Ok(content_buffer);
    }

    fn consume_messages( &mut self ) -> Result<Bytes,Box<std::error::Error>>
    {
        let messages_buffer_id = self.messages_buffer_id;
        if messages_buffer_id.is_none()
        {
            return Err("messages buffer was not created by wasm guest".into());
        }
        let messages_buffer_option = self.consume_buffer(messages_buffer_id.unwrap());
        if messages_buffer_option.is_none()
        {
            return Err("messages buffer is not available".into());
        }
        let messages_buffer = messages_buffer_option.unwrap();
        self.messages_buffer_id= Option::None;
        return Ok(messages_buffer);
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

    fn content_update( &mut self, buffer_id: i32 )
    {
println!("content_update: {}",buffer_id);
        self.content_buffer_id = Option::Some(buffer_id);
    }

    fn messages_update( &mut self, buffer_id: i32 )
    {
        self.messages_buffer_id = Option::Some(buffer_id);
    }
}

#[derive(WasmerEnv, Clone)]
struct Env {
    host: Arc<Mutex<WasmHost>>
}


impl <'a> WasmBinder<'a>
{
    pub fn new( module: &'a Module ) -> Result<Self,Box<dyn Error>>
    {

        let host = Arc::new( Mutex::new( WasmHost::new() ));
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

        "host_content_update"=>Function::new_native_with_env(module.store(),Env{host:host.clone()},|env:&Env,buffer_id:i32| {
                 env.host.lock().unwrap().content_update(buffer_id);
            } ),
        "host_messages"=>Function::new_native_with_env(module.store(),Env{host:host.clone()},|env:&Env,buffer_id:i32| {
                 env.host.lock().unwrap().messages_update(buffer_id);
            } ),
        } };

        let instance = Instance::new( module, &imports )?;

        let guest = WasmGuest { instance: instance };

        let binder = WasmBinder{ module: module, guest: guest, host:host };

        return Ok(binder);
    }

    fn log( &self, ptr: i32, len: i32 )
    {}
}


pub trait Buffers
{
    fn alloc_buffer(&mut self, len: i32) -> i32;
    fn dealloc_buffer(&mut self,id: i32);
    fn append_to_buffer(&mut self,id: i32, value: u8 );
}

pub trait BuffersSupport
{
    fn write_string( &mut self, str: &str ) -> i32;
    fn write_buffer( &mut self, bytes: &[u8] ) -> i32;
}

pub trait BufferConsume
{
    fn consume_buffer(&mut self, id: i32) -> Option<Bytes>;
}

impl <'a> Buffers for WasmGuest
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

impl <'a> BuffersSupport for WasmGuest
{
    fn write_string( &mut self, str: &str ) -> i32
    {
        let buffer_id = self.alloc_buffer(str.len() as i32 );
        for b in str.bytes()
        {
            self.append_to_buffer(buffer_id,b);
        }
        return buffer_id;
    }

    fn write_buffer( &mut self, bytes: &[u8] ) -> i32
    {
        let buffer_id = self.alloc_buffer(bytes.len() as i32 );
        for b in bytes
        {
            self.append_to_buffer(buffer_id, *b );
        }
        return buffer_id;
    }
}

impl <'a> WasmGuest
{

    pub fn bind_message_artifact(&mut self, artifact_file: &Artifact, artifact_file_contents: &str ) -> Result<(),Box<std::error::Error>>
    {
        let artifact_file_buffer_id = self.write_string(&artifact_file.to() );
        let artifact_file_contents_buffer_id = self.write_string(artifact_file_contents);
        self.instance.exports.get_function("bind_message_artifact").unwrap().call(&[Value::I32(artifact_file_buffer_id), Value::I32(artifact_file_contents_buffer_id)])?;
        return Ok(());
    }
}

impl Buffers for WasmHost
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

impl BufferConsume for WasmHost
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


