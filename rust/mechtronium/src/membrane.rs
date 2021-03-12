use std::any::Any;
use std::borrow::{Borrow, BorrowMut};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::fmt;
use std::fmt::Debug;
use std::ops::Deref;
use std::pin::Pin;
use std::rc::Rc;
use std::string::FromUtf8Error;
use std::sync::{Arc, Mutex, MutexGuard, RwLock, Weak};
use std::sync::atomic::{AtomicUsize, Ordering};

use wasmer::{Array, ExportError, Function, FunctionType, ImportObject, imports, Instance, InstantiationError, Memory, Module, NativeFunc, Resolver, RuntimeError, Val, ValType, Value, WasmerEnv, WasmPtr};

use mechtron_common::artifact::Artifact;
use mechtron_common::configs::{Configs, MechtronConfig};
use mechtron_common::message::{Message, MessageBuilder};
use mechtron_common::state::{ReadOnlyState, State, StateMeta, ReadOnlyStateMeta};

use crate::cache::Cache;
use crate::error::Error;
use mechtron_common::mechtron::Context;
use mechtron_common::buffers::Buffer;

pub struct WasmMembrane {
    module: Arc<Module>,
    instance: Instance,
    host: Arc<RwLock<WasmHost>>,
    configs: Arc<Configs>
}

impl WasmMembrane {

    pub fn init(&self)->Result<(),Error>
    {
        let mut pass = true;
        match self.instance.exports.get_memory("memory")
        {
            Ok(_) => {
                self.log("wasm", "verified: memory");
            }
            Err(_) => {
                self.log("wasm", "failed: memory. could not access wasm memory. (expecting the memory module named 'memory')");
                pass=false
            }
        }


        match self.instance.exports.get_native_function::<i32,i32>("wasm_alloc_buffer"){
            Ok(_) => {
                self.log("wasm", "verified: wasm_alloc_buffer( i32 ) -> i32");
            }
            Err(_) => {
                self.log("wasm", "failed: wasm_alloc_buffer( i32 ) -> i32");
                pass=false
            }
        }
        match self.instance.exports.get_native_function::<i32,WasmPtr<u8,Array>>("wasm_get_buffer_ptr"){
            Ok(_) => {
                self.log("wasm", "verified: wasm_get_buffer_ptr( i32 ) -> *const u8");
            }
            Err(_) => {
                self.log("wasm", "failed: wasm_get_buffer_ptr( i32 ) -> *const u8");
                pass=false
            }
        }
        match self.instance.exports.get_native_function::<i32,i32>("wasm_get_buffer_len"){
            Ok(_) => {
                self.log("wasm", "verified: wasm_get_buffer_len( i32 ) -> i32");
            }
            Err(_) => {
                self.log("wasm", "failed: wasm_get_buffer_len( i32 ) -> i32");
                pass=false
            }
        }
        match self.instance.exports.get_native_function::<i32,()>("wasm_dealloc_buffer"){
            Ok(_) => {
                self.log("wasm", "verified: wasm_dealloc_buffer( i32 )");
            }
            Err(_) => {
                self.log("wasm", "failed: wasm_dealloc_buffer( i32 )");
                pass=false
            }
        }

        {
            let test = "Test write string";
            match self.write_string(test){
                Ok(string_buffer) => {

                    self.log("wasm", "passed: write_string()");
                },
                Err(e) => {
                    self.log("wasm", format!("failed: write_string() test {:?}", e).as_str());
                    pass = false;

                }
            };
        }

        match self.instance.exports.get_native_function::<(),()>("wasm_init"){
            Ok(func) => {

                self.log("wasm", "verified: wasm_init( )");
                match func.call()
                {
                    Ok(_) => {
                        self.log("wasm", "passed: wasm_init( )");
                    }
                    Err(e) => {

                        self.log("wasm", format!("failed: wasm_init( ).  {:?}", e).as_str());
                    }
                }
            }
            Err(e) => {
                self.log("wasm", format!("failed: wasm_init( ) {:?}", e).as_str());
                pass=false
            }
        }


        match self.instance.exports.get_native_function::<(),()>("mechtron_init"){
            Ok(func) => {

                self.log("wasm", "verified: mechtron_init( )");
                match func.call()
                {
                    Ok(_) => {
                        self.log("wasm", "passed: mechtron_init( )");
                    }
                    Err(e) => {

                        self.log("wasm", format!("failed: mechtron_init( ).  {:?}", e).as_str());
                    }
                }
            }
            Err(e) => {
                self.log("wasm", format!("failed: mechtron_init( ) {:?}", e).as_str());
                pass=false
            }
        }


        match pass{
            true => Ok(()),
            false => Err("init failed".into())
        }

    }


    pub fn log( &self, log_type:&str, message: &str )
    {
        println!("{} : {}",log_type,message);
    }

    fn write_string(&self, string: &str )->Result<i32,Error>
    {
        let string = string.as_bytes();
        let mut memory = self.instance.exports.get_memory("memory")?;
        let buffer_id = self.alloc_buffer(string.len() as _ )?;
        let buffer_ptr = self.get_buffer_ptr(buffer_id)?;
        let values = buffer_ptr.deref(memory, 0, string.len() as u32).unwrap();
        for i in 0..string.len() {
            values[i].set(string[i] );
        }

        Ok(buffer_id)
    }

    fn write_buffer(&self, bytes: &Vec<u8> )->Result<i32,Error>
    {
        let mut memory = self.instance.exports.get_memory("memory")?;
        let buffer_id = self.alloc_buffer(bytes.len() as _ )?;
        let buffer_ptr = self.get_buffer_ptr(buffer_id)?;
        let values = buffer_ptr.deref(memory, 0, bytes.len() as u32).unwrap();
        for i in 0..bytes.len() {
            values[i].set(bytes[i] );
        }

        Ok(buffer_id)
    }


    fn alloc_buffer(&self, len: i32 ) ->Result<i32,Error>
    {
        let buffer_id= self.instance.exports.get_native_function::<i32,i32>("wasm_alloc_buffer").unwrap().call(len.clone())?;
        Ok(buffer_id)
    }

    fn get_buffer_ptr( &self, buffer_id: i32 )->Result<WasmPtr<u8,Array>,Error>
    {
        Ok(self.instance.exports.get_native_function::<i32, WasmPtr<u8, Array>>("wasm_get_buffer_ptr").unwrap().call(buffer_id)?)
    }

    fn read_buffer(&self, buffer_id: i32 ) ->Result<Vec<u8>,Error>
    {
        let ptr = self.instance.exports.get_native_function::<i32,WasmPtr<u8,Array>>("wasm_get_buffer_ptr").unwrap().call(buffer_id )?;
        let len = self.instance.exports.get_native_function::<i32,i32>("wasm_get_buffer_len").unwrap().call(buffer_id )?;
        let memory = self.instance.exports.get_memory("memory")?;
        let values = ptr.deref(memory, 0, len as u32).unwrap();
        let mut rtn = vec!();
        for i in 0..values.len() {
           rtn.push( values[i].get() )
        }

        Ok(rtn)
    }

    fn read_string(&self, buffer_id: i32 ) ->Result<String,Error>
    {
        let raw = self.read_buffer(buffer_id)?;
        let rtn = String::from_utf8(raw)?;

        Ok(rtn)
    }

    fn wasm_dealloc_buffer( &self, buffer_id: i32 )->Result<(),Error>
    {
        self.instance.exports.get_native_function::<i32,()>("wasm_dealloc_buffer")?.call(buffer_id.clone())?;
        Ok(())
    }

    fn wasm_dealloc(&self, buffer: WasmBuffer ) ->Result<(),Error>
    {
        self.instance.exports.get_native_function::<(WasmPtr<u8,Array>,i32),()>("wasm_dealloc").unwrap().call(buffer.ptr,buffer.len as _)?;
        Ok(())
    }






    fn inject_state(&self, state: &ReadOnlyState) ->Result<i32,Error>
    {
        let state_buffer_id = self.write_buffer( &state.to_bytes(&self.configs)?)?;
        self.instance.exports.get_native_function::<i32,()>("wasm_inject_state").unwrap().call(state_buffer_id.clone() )?;
        Ok(state_buffer_id)
    }

    fn extract_state(&self, state: i32) ->Result<State,Error>
    {
        self.instance.exports.get_native_function::<i32,()>("wasm_move_state_to_buffers").unwrap().call(state.clone() )?;
        let bytes = self.read_buffer(state)?;
        self.wasm_dealloc_buffer(state.clone());
        let state = State::from_bytes(bytes,&self.configs)?;
        Ok(state)
    }

    fn test_modify_state(&self, state: i32 )->Result<(),Error>
    {
        self.instance.exports.get_native_function::<i32,()>("wasm_test_modify_state").unwrap().call(state.clone() )?;
        Ok(())
    }

    fn test_cache(&self)->Result<(),Error>
    {
        self.instance.exports.get_native_function::<(),()>("mechtron_test_cache").unwrap().call()?;
        Ok(())
    }

    fn test_panic(&self)->Result<(),Error>
    {
        self.instance.exports.get_native_function::<(),()>("wasm_test_panic").unwrap().call()?;
        Ok(())
    }

    fn test_ok(&self)->Result<(),Error>
    {
        self.instance.exports.get_native_function::<(),()>("wasm_test_ok").unwrap().call()?;
        Ok(())
    }

    fn test_log(&self)->Result<(),Error>
    {
        self.instance.exports.get_native_function::<(),()>("wasm_test_log").unwrap().call()?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct WasmBuffer
{
    ptr: WasmPtr<u8,Array>,
    len: u32
}

impl WasmBuffer
{
   pub fn new( ptr: WasmPtr<u8,Array>,
               len: u32 )->Self
   {
       WasmBuffer{
           ptr: ptr,
           len: len
       }
   }
}




struct WasmHost {
    membrane: Option<Weak<WasmMembrane>>,
    buffers: HashMap<i32,Vec<u8>>
}


impl  WasmHost{

    fn new() ->Self
    {
        WasmHost{
            membrane: Option::None,
            buffers: HashMap::new()
        }
    }

    fn panic(&self, error: Error)
    {
        println!("{:?}", error);
    }
}

#[derive(WasmerEnv, Clone)]
struct Env {
    host: Arc<RwLock<WasmHost>>,
}

impl Env
{
    pub fn unwrap(&self) -> Result<Arc<WasmMembrane>, Error>
    {
        let host = self.host.read();
        if host.is_err()
        {
            println!("WasmMembrane: could not acquire host lock");
            return Err("could not acquire host lock".into());
        }

        let host = host.unwrap();

        let membrane = host.membrane.as_ref();
        if membrane.is_none()
        {
            println!("WasmMembrane: membrane is not set");
            return Err("membrane is not set".into());
        }
        let membrane = membrane.unwrap().upgrade();

        if membrane.is_none()
        {
            println!("WasmMembrane: could not upgrade membrane reference");
            return Err("could not upgrade membrane reference".into());
        }
        let membrane = membrane.unwrap();
        let mut memory = membrane.instance.exports.get_memory("memory");
        if memory.is_err()
        {
            println!("WasmMembrane: could not access wasm memory");
            return Err("could not access wasm memory".into());
        }
        Ok(membrane)
    }
}

impl WasmMembrane {
    pub fn new(module: Arc<Module>, configs: Arc<Configs>) -> Result<Arc<Self>, Error> {
        let mut host = Arc::new(RwLock::new(WasmHost::new()));

        let imports = imports! { "env"=>{
        "mechtronium_log"=>Function::new_native_with_env(module.store(),Env{host:host.clone()},|env:&Env,type_ptr:WasmPtr<u8,Array>,type_len:i32,ptr:WasmPtr<u8,Array>,len:i32| {
                match env.unwrap()
                {
                   Ok(membrane)=>{
                        let mut memory = membrane.instance.exports.get_memory("memory").unwrap();
                        let log_type= type_ptr.get_utf8_string(memory, type_len as u32).unwrap();
                        let str = ptr.get_utf8_string(memory, len as u32).unwrap();
                        membrane.log(log_type.as_str(),str.as_str());
                   },
                   Err(_)=>{}
                }
            }),
        "mechtronium_cache"=>Function::new_native_with_env(module.store(),Env{host:host.clone()},|env:&Env,artifact_id:i32| {
                match env.unwrap()
                {
                   Ok(membrane)=>{
                       let artifact = membrane.read_string(artifact_id).unwrap();
                       let artifact = Artifact::from(&artifact).unwrap();
                       membrane.configs.cache(&artifact);
                   },
                   Err(e)=>{
                     println!("error");
                   }
                }
            }),
            "mechtronium_load"=>Function::new_native_with_env(module.store(),Env{host:host.clone()},|env:&Env,artifact_id:i32|->i32 {

                match env.unwrap()
                {
                   Ok(membrane)=>{
                       let artifact = membrane.read_string(artifact_id).unwrap();
                       let artifact = Artifact::from(&artifact).unwrap();
                       let buffer = membrane.configs.artifacts.load(&artifact).unwrap();
                       let buffer_id = membrane.write_buffer(&buffer).unwrap();
                       return buffer_id;
                   },
                   Err(e)=>{
                     println!("error");
                     return -1
                   }
                }
            })
        } };


        let instance = Instance::new(&module, &imports)?;

        let mut membrane = Arc::new(WasmMembrane {
            module: module,
            instance: instance,
            host: host.clone(),
            configs:configs
        });

        {
            host.write().unwrap().membrane = Option::Some(Arc::downgrade(&membrane));
        }

        return Ok(membrane);
    }
}

pub struct BufferLock
{
    id: i32,
    membrane: Arc<WasmMembrane>
}

impl BufferLock
{
    pub fn new( id: i32, membrane: Arc<WasmMembrane> )->Self
    {
        BufferLock{
           id: id,
           membrane: membrane
        }
    }

    pub fn id(&self)->i32
    {
        self.id.clone()
    }

    pub fn release(&self) -> Result<(),Error>
    {
        self.membrane.wasm_dealloc_buffer(self.id)?;
        Ok(())
    }
}

impl Drop for BufferLock
{
    fn drop(&mut self) {
        self.release();
    }
}

pub enum MechtronMembraneStatus
{
    None,
    Ejected(Arc<Mutex<State>>),
    Injected(i32),
    Extracted
}

pub enum StateExtraction
{
    Unchanged(Arc<ReadOnlyState>),
    Changed(Arc<Mutex<State>>)
}

pub struct MechtronMembrane
{
    wasm_membrane: Arc<WasmMembrane>,
    original_state: Arc<ReadOnlyState>,
    status: MechtronMembraneStatus
}

impl MechtronMembrane
{
    pub fn new(membrane: Arc<WasmMembrane>, state: Arc<ReadOnlyState> ) -> Self
    {
        MechtronMembrane {
            wasm_membrane: membrane,
            original_state: state,
            status: MechtronMembraneStatus::None
        }
    }

    fn state(&mut self) ->Result<i32,Error>
    {
        self.inject()
    }

    pub fn current_state(&mut self)->Result<StateExtraction,Error>
    {
        self.eject()
    }

    pub fn original_state(&self)->Arc<ReadOnlyState>
    {
        self.original_state.clone()
    }

    pub fn is_tainted(&self) -> Result<bool, Error>
    {
        match &self.status{
            MechtronMembraneStatus::None => {
                Ok(self.original_state.is_tainted()?)
            }
            MechtronMembraneStatus::Ejected(state) => {
                let state = state.lock()?;
                Ok(state.is_tainted()?)
            }
            MechtronMembraneStatus::Injected(state_buffer_id) => {
                 Ok(0 != self.wasm_membrane.instance.exports.get_native_function::<i32,i32>("mechtron_is_tainted").unwrap().call(state_buffer_id.clone() )?)
            }
            MechtronMembraneStatus::Extracted => {
                Err("this MechtronMembrane's state has already been extracted and can no longer be used.".into())
            }
        }
    }

    pub fn set_taint(&mut self, reason: &str ) -> Result<(), Error>
    {
        println!("tainted reason: {}",reason);
        let state = self.eject_changed()?;
        let mut state = state.lock()?;
        state.set_taint(true);
        Ok(())
    }

    fn write_buffer(&self, bytes: &Vec<u8>) ->Result<BufferLock,Error>
    {
        let buffer_id = self.wasm_membrane.write_buffer(&bytes)?;
        let lock = BufferLock::new(buffer_id,self.wasm_membrane.clone());
        Ok(lock)
    }

    fn write_string(&self, string: &str ) ->Result<BufferLock,Error>
    {
        let buffer_id = self.wasm_membrane.write_string(string)?;
        let lock = BufferLock::new(buffer_id,self.wasm_membrane.clone());
        Ok(lock)
    }

    fn write_context(&self, context: &Context) ->Result<BufferLock,Error>
    {
        let bytes = context.to_bytes(&self.wasm_membrane.configs)?;
        self.write_buffer(&bytes)
    }

    fn write_message(&self, message: &Message) ->Result<BufferLock,Error>
    {
        let bytes = message.to_bytes(&self.wasm_membrane.configs)?;
        self.write_buffer(&bytes)
    }

    fn consume_buffer(&self, buffer_id: i32)->Result<Vec<u8>,Error>
    {
        let bytes = self.wasm_membrane.read_buffer(buffer_id)?;
        self.wasm_membrane.wasm_dealloc_buffer(buffer_id)?;
        Ok(bytes)
    }

    fn consume_builders(&self, buffer_id: i32)->Result<Vec<MessageBuilder>,Error>
    {
        let bytes = self.consume_buffer(buffer_id)?;
        let builders = MessageBuilder::from_buffer(bytes,&self.wasm_membrane.configs)?;
        Ok(builders)
    }


    pub fn create(&mut self, context: &Context, message: &Message) ->Result<Vec<MessageBuilder>,Error>
    {
        let context_lock = self.write_context(context)?;
        let message_lock = self.write_message(message)?;

        let call = self.wasm_membrane.instance.exports.get_native_function::<(i32, i32, i32),i32>("mechtron_create")?;
        let builders = call.call(context_lock.id(), self.state()?, message_lock.id())?;

        if builders == -1
        {
            Ok(vec![])
        }
        else {
            let builders = self.consume_builders(builders)?;
            Ok(builders)
        }
    }

    pub fn update(&mut self, context: &Context) ->Result<Vec<MessageBuilder>,Error>
    {
        let context_lock = self.write_context(context)?;
        let builders = self.wasm_membrane.instance.exports.get_native_function::<(i32,i32),i32>("mechtron_update").unwrap().call(context_lock.id(), self.state()?)?;
        if builders == -1
        {
            Ok(vec![])
        }
        else {
            let builders = self.consume_builders(builders)?;
            Ok(builders)
        }
    }


    pub fn message(&mut self, context: &Context, message: &Message) ->Result<Vec<MessageBuilder>,Error>
    {
        let port = message.to.port.clone();
        let context_lock = self.write_context(context)?;
        let message_lock = self.write_message(message)?;
println!("Fascinating.");
        let result = self.wasm_membrane.instance.exports.get_native_function::<(i32, i32, i32),i32>("mechtron_message").unwrap().call(context_lock.id(), self.state()?, message_lock.id());
println!("Doctor.");
        match result{
            Ok(builders) => {
               match builders{
                   -1 => Ok(vec!()),
                   -2 => Err(format!("{}.inbound.{} wasm return -2 ERROR code.",self.original_state.config.kind.clone(),port).into()),
                   builders=> {
                       let builders = self.consume_builders(builders)?;
println!("Ok Builders.");
                       Ok(builders)
                   }
               }
            }
            Err(error) => {
                Err(format!("{}.inbound.{} received error from wasm: {:?}",self.original_state.config.kind.clone(),port,error).into())
            }
        }
    }

    pub fn extra(&mut self, context: &Context, message: &Message) ->Result<Vec<MessageBuilder>,Error>
    {
        let context_lock = self.write_context(context)?;
        let message_lock = self.write_message(message)?;
        let builders = self.wasm_membrane.instance.exports.get_native_function::<(i32, i32, i32),i32>("mechtron_extra").unwrap().call(context_lock.id(), self.state()?, message_lock.id())?;
        if builders == -1
        {
            Ok(vec![])
        }
        else {
            let builders = self.consume_builders(builders)?;
            Ok(builders)
        }
    }


    pub fn get_mechtron_config(&self) -> Arc<MechtronConfig>
    {
        self.original_state.config.clone()
    }

    fn inject(&mut self) -> Result<i32, Error>
    {
        self.status = MechtronMembraneStatus::Injected(match &self.status
        {
            MechtronMembraneStatus::None => {
                let state_buffer_id = self.wasm_membrane.inject_state(&self.original_state)?;
                state_buffer_id
            }
            MechtronMembraneStatus::Ejected(state) => {
                let state = state.lock()?;
                let state_buffer_id = self.wasm_membrane.inject_state(&state.read_only()? )?;
                state_buffer_id
            }
            MechtronMembraneStatus::Injected(state_buffer_id) => {
                state_buffer_id.clone()
            }
            MechtronMembraneStatus::Extracted => {
                return Err("this MechtronMembrane's state has already been extracted and can no longer be used.".into());
            }
        });

        return match self.status{
            MechtronMembraneStatus::Injected(state_buffer_id) => Ok(state_buffer_id),
            _ => Err("MechtronMembrane.inject(): Seems like the impossible has happened".into())
        }

    }

    fn eject_changed(&mut self) -> Result<Arc<Mutex<State>>, Error>
    {
        Ok( match self.eject()? {
            StateExtraction::Unchanged(state) => {
                Arc::new(Mutex::new(state.copy()) )
            }
            StateExtraction::Changed(state) => state
        })
    }

    fn eject(&mut self) -> Result<StateExtraction, Error>
    {
        let rtn = match &self.status
        {
            MechtronMembraneStatus::None => {
                StateExtraction::Unchanged(self.original_state.clone())
            }
            MechtronMembraneStatus::Ejected(state) => {
                StateExtraction::Changed(state.clone())
            }
            MechtronMembraneStatus::Injected(state_buffer_id) => {
                let state = Arc::new(Mutex::new(self.wasm_membrane.extract_state(state_buffer_id.clone() )?));
                self.status = MechtronMembraneStatus::Ejected(state.clone());
                StateExtraction::Changed(state)
            }
            MechtronMembraneStatus::Extracted => {
                return Err("this MechtronMembrane's state has already been extracted and can no longer be used.".into());
            }
        };
        Ok(rtn)
    }

    pub fn extract(&mut self) -> Result<StateExtraction, Error>
    {
        let rtn = match &self.status
        {
            MechtronMembraneStatus::None => {
               StateExtraction::Unchanged(self.original_state.clone())
            }
            MechtronMembraneStatus::Ejected(state) => {
                StateExtraction::Changed(state.clone())
            }
            MechtronMembraneStatus::Injected(state_buffer_id) => {
                let state = self.wasm_membrane.extract_state(state_buffer_id.clone() )?;
                StateExtraction::Changed(Arc::new(Mutex::new(state)))
            }
            MechtronMembraneStatus::Extracted => {
                return Err("this MechtronMembrane's state has already been extracted and can no longer be used.".into());
            }
        };

        self.status = MechtronMembraneStatus::Extracted;

        Ok(rtn)
    }



}

impl  Drop for MechtronMembrane
{
    fn drop(&mut self) {
        self.extract();
    }
}






#[cfg(test)]
mod test
{
    use std::fs::File;
    use std::io::Read;
    use std::sync::Arc;

    use wasmer::{Cranelift, JIT, Module, Store};

    use mechtron_common::core::*;
    use mechtron_common::state::{ReadOnlyStateMeta, State, StateMeta};

    use crate::error::Error;
    use crate::membrane::WasmMembrane;
    use crate::node::Node;

    fn membrane() -> Result<Arc<WasmMembrane>, Error>
    {
        let path = "../../repo/mechtron.io/examples/0.0.1/hello-world/wasm/hello-world.wasm";

        let mut file = File::open(path)?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;

        let store = Store::new(&JIT::new(Cranelift::default()).engine());
        let module = Module::new(&store, data)?;
        let mut membrane = WasmMembrane::new(Arc::new(module), Node::default_cache().configs.clone()).unwrap();
        membrane.init()?;

        Ok(membrane)
    }


    #[test]
    fn test_wasm() -> Result<(), Error>
    {
        let membrane = membrane()?;

        let buffer_id = membrane.write_string("Hello From Mechtronium")?;

        membrane.wasm_dealloc_buffer(buffer_id)?;

        Ok(())
    }


    #[test]
    fn test_cache() -> Result<(), Error>
    {
        let membrane = membrane()?;

        membrane.test_cache()?;

        Ok(())
    }


    #[test]
    fn test_panic() -> Result<(), Error>
    {
        let membrane = membrane()?;

        match membrane.test_panic()
        {
            Ok(_) => {
                assert!(false)
            }
            Err(_) => {}
        }

        membrane.test_ok()?;

        Ok(())
    }


    #[test]
    fn test_mechtron_create() -> Result<(), Error>
    {
        let membrane = membrane()?;

        match membrane.test_panic()
        {
            Ok(_) => {
                assert!(false)
            }
            Err(_) => {}
        }

        membrane.test_ok()?;

        Ok(())
    }

    #[test]
    fn test_inject_and_extract_state() -> Result<(), Error>
    {
        let cache = Node::default_cache();
        let membrane = membrane()?;

        let config = cache.configs.mechtrons.get(&CORE_MECHTRON_NEUTRON )?;
        let mut state = State::new(&cache.configs,config.clone())?;
        state.set_taint(true);

        let state_buffer_id= membrane.inject_state(&state.read_only()? )?;
        let extracted_state = membrane.extract_state(state_buffer_id)?;

        assert_eq!(state.is_tainted()?, extracted_state.is_tainted()?);

        Ok(())
    }

    #[test]
    fn test_grow_state() -> Result<(), Error>
    {
        let cache = Node::default_cache();
        let membrane = membrane()?;

        let config = cache.configs.mechtrons.get(&CORE_MECHTRON_NEUTRON )?;
        let mut state = State::new(&cache.configs,config.clone())?;
        state.set_taint(false);

        let state_buffer_id= membrane.inject_state(&state.read_only()?)?;

        membrane.test_modify_state(state_buffer_id);

        let updated_state = membrane.extract_state(state_buffer_id)?;

        assert!(!state.is_tainted()?);
        assert!(updated_state.is_tainted()?);

        Ok(())
    }

}