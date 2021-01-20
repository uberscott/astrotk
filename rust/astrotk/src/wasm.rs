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
use std::fmt;
use std::rc::Rc;
use bytes::{Bytes,BytesMut,BufMut,Buf};
use std::string::FromUtf8Error;
use std::error::Error;
use astrotk_config::artifact_config::{ArtifactFile, Artifact, ArtifactRepository};
use astrotk_config::actor_config::ActorConfig;


pub struct WasmBinder<'a>
{
    module: &'a Module,
    guest: Guest,
    host: Arc<Mutex<Host>>
}

impl <'a> WasmBinder <'a> {
    pub fn write_string( &mut self, str: &str ) -> i32
    {
        let buffer_id = self.guest.alloc_buffer(str.len() as i32 );
        for b in str.bytes()
        {
            self.guest.append_to_buffer(buffer_id,b);
        }
        return buffer_id;
    }
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

        return Ok(binder);
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

pub trait GuestActor
{
    fn actor_init(&mut self,
                  actor_config_buffer_id: i32,
                  actor_config_artifact_buffer_id: i32,
                  content_config_buffer_id: i32) -> Result<i32,Box<std::error::Error>>;
}

pub trait BufferConsume
{
    fn consume_buffer(&mut self, id: i32) -> Option<Bytes>;
}

impl <'a> GuestActor for Guest
{
    fn actor_init(&mut self,
                  actor_config_buffer_id: i32,
                  actor_config_artifact_buffer_id: i32,
                  content_config_buffer_id: i32) -> Result<i32,Box<std::error::Error>>
    {
        return Ok(self.instance.exports.get_native_function::<(i32, i32, i32),i32>("actor_init").unwrap().call(actor_config_buffer_id,actor_config_artifact_buffer_id, content_config_buffer_id)?);
    }
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


pub struct WasmActor<'a>
{
    config: &'a ActorConfig,
    wasm: WasmBinder<'a>
}

impl <'a> WasmActor<'a>
{
    pub fn create( astroTK: &'a AstroTK, config: &'a ActorConfig ) -> Result<Self,Box<Error>>
    {
        let module = astroTK.get_wasm_module(&config.wasm).unwrap();
        let mut binder = WasmBinder::new(module)?;

        let actor_config_buffer_id = binder.write_string(astroTK.artifact_repository.get_cached_string( &config.source ).unwrap());
        let actor_config_artifact_buffer_id = binder.write_string( config.source.to().as_str() );
        let content_config_buffer_id = binder.write_string(astroTK.artifact_repository.get_cached_string(&config.content ).unwrap() );
        binder.guest.actor_init(actor_config_buffer_id,actor_config_artifact_buffer_id,content_config_buffer_id)?;

        binder.guest.dealloc_buffer(actor_config_buffer_id);
        binder.guest.dealloc_buffer(actor_config_artifact_buffer_id);
        binder.guest.dealloc_buffer(content_config_buffer_id);

        return Ok(WasmActor{config:config,wasm:binder});
    }

}


