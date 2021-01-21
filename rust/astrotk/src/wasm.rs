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
use no_proto::buffer::NP_Buffer;
use std::cell::RefCell;
use std::any::Any;
use no_proto::buffer_ro::NP_Buffer_RO;
use crate::actor::ActorEngine;


pub struct WasmBinder<'a>
{
    module: &'a Module,
    guest: Guest,
    host: Arc<Mutex<Host>>
}

impl <'a> WasmBinder <'a> {

}

struct Guest
{
    instance: Instance
}

struct Host
{
    buffer_map: HashMap<i32,BytesMut>,
    buffer_index: i32,
    content_buffer_id: Option<i32>
}

impl Host
{
    fn new() -> Self
    {
        Host {
            buffer_map: HashMap::new(),
            buffer_index: 0,
            content_buffer_id: Option::None
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
}

#[derive(WasmerEnv, Clone)]
struct Env {
    host: Arc<Mutex<Host>>
}


    impl <'a> WasmBinder<'a>
{
    pub fn new( module: &'a Module ) -> Result<Self>
    {

        let host = Arc::new( Mutex::new( Host::new() ));
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
        } };

        let instance = Instance::new( module, &imports )?;

        let guest = Guest { instance: instance };

        let binder = WasmBinder{ module: module, guest: guest, host:host };

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

pub trait BuffersSupport
{
    fn write_string( &mut self, str: &str ) -> i32;
    fn write_buffer( &mut self, bytes: &[u8] ) -> i32;
}

pub trait GuestActor
{
    fn add_config( &mut self,
                   artifact_file: &ArtifactFile,
                   config_str: &str );

    fn actor_init(&mut self,
                  actor_config: &ActorConfig,
                  raw_actor_config_yaml: &str ) -> Result<i32,Box<std::error::Error>>;

    fn actor_create( &mut self, create_message: &NP_Buffer ) -> Result<(),Box<std::error::Error>>;
}

pub trait BufferConsume
{
    fn consume_buffer(&mut self, id: i32) -> Option<Bytes>;
}

impl <'a> GuestActor for Guest
{
    fn add_config(&mut self, artifact_file: &ArtifactFile, config_str: &str) {

    }

    fn actor_init(&mut self,
                  actor_config: &ActorConfig,
                  raw_actor_config_yaml: &str,
                  ) -> Result<i32,Box<std::error::Error>>
    {
        let actor_config_artifact_buffer_id = self.write_string(&actor_config.source.to() );
        let actor_config_buffer_id = self.write_string(raw_actor_config_yaml );
        return Ok(self.instance.exports.get_native_function::<(i32, i32),i32>("actor_init").unwrap().call(actor_config_buffer_id,actor_config_artifact_buffer_id)?);
    }

    fn actor_create(&mut self, create_message: &NP_Buffer)->Result<(),Box<std::error::Error>> {
        let len = create_message.calc_bytes().unwrap().current_buffer;
        let create_message_buffer_id = self.write_buffer(create_message.read_bytes() );

        self.instance.exports.get_function("astrotk_actor_create").unwrap().call(&[Value::I32(create_message_buffer_id)]);
        return Ok(());
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

impl <'a> BuffersSupport for Guest
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

impl <'a> Guest
{

    pub fn bind_message_artifact(&mut self, artifact_file: &ArtifactFile, artifact_file_contents: &str ) -> Result<(),Box<std::error::Error>>
    {
        let artifact_file_buffer_id = self.write_string(&artifact_file.to() );
        let artifact_file_contents_buffer_id = self.write_string(artifact_file_contents);
        self.instance.exports.get_function("bind_message_artifact").unwrap().call(&[Value::I32(artifact_file_buffer_id), Value::I32(artifact_file_contents_buffer_id)])?;
        return Ok(());
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


pub struct WasmActorEngine<'a>
{
    config: &'a ActorConfig,
    wasm: WasmBinder<'a>,
    astroTK: &'a AstroTK<'a>
}

impl <'a> WasmActorEngine<'a>
{
    pub fn new(astroTK: &'a AstroTK, config: &'a ActorConfig) -> Result<Self, Box<Error>>
    {
        let module = astroTK.get_wasm_module(&config.wasm).unwrap();
        let mut wasm = WasmBinder::new(module)?;

        for artifact_file in config.message_artifact_files()
        {
            match astroTK.artifact_repository.get_cached_string(&artifact_file)
            {
                None => {
                    println!("missing artifact content for {}", artifact_file.to())
                }
                Some(content) => {
                    wasm.guest.bind_message_artifact(&artifact_file, content);
                }
            }
        }

        wasm.guest.actor_init(config, astroTK.artifact_repository.get_cached_string(&config.source).unwrap());

        return Ok(WasmActorEngine { config: config, wasm: wasm, astroTK: astroTK });
    }

    fn to_np_buffer( &self, bytes: Bytes, artifact_file: &ArtifactFile ) -> Result<NP_Buffer,Box<Error>>
    {
        let factory_option = self.astroTK.get_buffer_factory(&self.config.content );

        if factory_option.is_none()
        {
            return Err(format!("cannot find factory for artifact {} ",self.config.content.to()).into());
        }

        let factory = factory_option.unwrap();
        let content = factory.open_buffer(bytes.to_vec() );
        return Ok(content);
    }
}

impl <'a> ActorEngine for WasmActorEngine<'a>
{
    fn create( &mut self, create_message: &NP_Buffer ) -> Result<NP_Buffer,Box<std::error::Error>>
    {
        self.wasm.guest.actor_create(create_message)?;
        let mut host = self.wasm.host.lock().unwrap();

        let content_buffer = host.consume_content()?;
        let content = self.to_np_buffer(content_buffer,&self.config.content)?;

        let name = content.get::<String>(&[&"name"]).unwrap().unwrap();
        let age = content.get::<i32>(&[&"age"]).unwrap().unwrap();
println!("CONTENT name: {} and age: {}",name, age);

        return Ok(content);
    }

    fn update(&mut self, content: &NP_Buffer, messages: Box<[&NP_Buffer]>) -> Result<(NP_Buffer,Box<[NP_Buffer]>),Box<std::error::Error>>
    {
        unimplemented!()
    }
}


