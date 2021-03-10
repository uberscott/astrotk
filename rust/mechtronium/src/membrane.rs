use std::any::Any;
use std::borrow::{Borrow, BorrowMut};
use std::cell::{RefCell, Cell};
use std::collections::HashMap;
use std::fmt;
use std::fmt::Debug;
use std::ops::Deref;
use std::pin::Pin;
use std::rc::Rc;
use std::string::FromUtf8Error;
use std::sync::{Arc, Mutex, RwLock, Weak};
use std::sync::atomic::{AtomicUsize, Ordering};
use mechtron_core::artifact::Artifact;
use wasmer::{Array, ExportError, Function, FunctionType, ImportObject, imports, Instance, InstantiationError, Memory, Module, NativeFunc, Resolver, RuntimeError, Val, ValType, Value, WasmerEnv, WasmPtr};

use crate::error::Error;
use crate::cache::Cache;

pub struct WasmMembrane {
    module: Module,
    instance: Instance,
    host: Arc<RwLock<WasmHost>>,
    cache: Arc<Cache<'static>>
}

impl WasmMembrane{

    pub fn init(&self)->bool
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


        pass
    }

    pub fn log( &self, log_type:&str, message: &str )
    {
        println!("{} : {}",log_type,message);
    }

    pub fn write_string(&self, string: &str )->Result<i32,Error>
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

    pub fn write_buffer(&self, bytes: &Vec<u8> )->Result<i32,Error>
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

   pub fn alloc_buffer(&self, len: i32 ) ->Result<i32,Error>
    {
        let buffer_id= self.instance.exports.get_native_function::<i32,i32>("wasm_alloc_buffer").unwrap().call(len.clone())?;
        Ok(buffer_id)
    }


    fn get_buffer_ptr( &self, buffer_id: i32 )->Result<WasmPtr<u8,Array>,Error>
    {
        Ok(self.instance.exports.get_native_function::<i32, WasmPtr<u8, Array>>("wasm_get_buffer_ptr").unwrap().call(buffer_id)?)
    }

    pub fn read_buffer(&self, buffer_id: i32 ) ->Result<Vec<u8>,Error>
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

    pub fn read_string(&self, buffer_id: i32 ) ->Result<String,Error>
    {
        let raw = self.read_buffer(buffer_id)?;
        let rtn = String::from_utf8(raw)?;

        Ok(rtn)
    }



    pub fn wasm_dealloc_buffer( &self, buffer_id: i32 )->Result<(),Error>
    {
        self.instance.exports.get_native_function::<i32,()>("wasm_dealloc_buffer").unwrap().call(buffer_id.clone())?;
        Ok(())
    }

    pub fn wasm_dealloc(&self, buffer: WasmBuffer ) ->Result<(),Error>
    {
        self.instance.exports.get_native_function::<(WasmPtr<u8,Array>,i32),()>("wasm_dealloc").unwrap().call(buffer.ptr,buffer.len as _)?;
        Ok(())
    }

    /*
    pub fn wasm_cache(&self, key: WasmBuffer, buffer: WasmBuffer ) ->Result<(),Error>
    {
        self.instance.exports.get_native_function::<(WasmPtr<u8,Array>,i32,WasmPtr<u8,Array>,i32),()>("wasm_cache").unwrap().call(key.ptr,key.len as _,buffer.ptr,buffer.len as _)?;
        Ok(())
    }



    pub fn wasm_test_invoke_stateful(&self) ->Result<(),Error>
    {
        let struct_key= self.write_string("mechtronium_test_struct")?;
        let method_key= self.write_string("mechtronium_test_method")?;

        let request_buffer_key = self.write_string("+Goodbye" )?;
        let state_buffer_out = "Hello".as_bytes().to_vec();
        let state_buffer_id = self.store_buffer( &state_buffer_out  )?;

        let call = self.instance.exports.get_native_function::<(WasmPtr<u8,Array>,i32,i32,WasmPtr<u8,Array>,i32,WasmPtr<u8,Array>,i32),i32>("wasm_stateful_invoke")?;
        let response_buffer_id = call.call(struct_key.ptr,struct_key.len as _, state_buffer_id, method_key.ptr,method_key.len as _, request_buffer_key.ptr,request_buffer_key.len as _ )?;

        let response_buffer = self.read_buffer(response_buffer_id)?;
        let state_buffer= self.read_buffer(state_buffer_id)?;

//        self.wasm_dealloc_buffer(state_buffer_id);
//        self.wasm_dealloc_buffer(response_buffer_id);
        println!("Response: {}",response_buffer.len());
        println!("State: {}",state_buffer.len());
        Ok(())
    }

     */

    pub fn test_cache(&self)->Result<(),Error>
    {
        println!("TESTING CACHE!");
        self.instance.exports.get_native_function::<(),()>("mechtron_test_cache").unwrap().call()?;
        println!("SUCCESS!");
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

impl  WasmMembrane {
    pub fn new(module: Module, cache: Arc<Cache<'static>>) -> Result<Arc<Self>, Error> {
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
                       membrane.cache.configs.cache(&artifact);
                   },
                   Err(e)=>{
                     println!("error");
                   }
                }
            }),
            "mechtronium_load"=>Function::new_native_with_env(module.store(),Env{host:host.clone()},|env:&Env,artifact_id:i32|->i32 {

println!("MECHTRONIUM LOAD");
                match env.unwrap()
                {
                   Ok(membrane)=>{
                       let artifact = membrane.read_string(artifact_id).unwrap();
                       let artifact = Artifact::from(&artifact).unwrap();
                       let buffer = membrane.cache.configs.artifacts.load(&artifact).unwrap();
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

        let mut membrane = Arc::new(WasmMembrane{
            module: module,
            instance: instance,
            host: host.clone(),
            cache: cache
        });

        {
            host.write().unwrap().membrane = Option::Some(Arc::downgrade(&membrane));
        }

        return Ok(membrane);
    }
}

struct WasmBufferLocker
{
    membrane: Arc<WasmMembrane>,
    buffers: RwLock<HashMap<String,i32>>
}

impl WasmBufferLocker
{
    pub fn new( membrane: Arc<WasmMembrane> ) -> Self
    {
        WasmBufferLocker
        {
            membrane: membrane.clone(),
            buffers: RwLock::new( HashMap::new() )
        }
    }

    pub fn store_buffer( &self, key: &str, buffer: &Vec<u8>)->Result<(),Error>
    {
        let buffer_id = self.membrane.write_buffer(buffer)?;
        let mut buffers = self.buffers.write()?;
        if buffers.contains_key(&key.to_string() )
        {
            return Err(format!("buffer locker already contains buffer named {} ", key).into());
        }

        buffers.insert( key.to_string(), buffer_id );

        Ok(())
    }

    pub fn remove_buffer( &self, key: &str ) -> Result<(),Error>
    {
        let mut buffers = self.buffers.write()?;
        let buffer_id = buffers.remove(&key.to_string() );

        if buffer_id.is_none()
        {
            return Ok(());
        }

        let buffer_id = buffer_id.unwrap();

        self.membrane.wasm_dealloc_buffer(buffer_id)?;

        Ok(())
    }

    pub fn remove_all(&self)->Result<(),Error>
    {
        let mut buffers = self.buffers.write()?;

        for buffer_id in buffers.values()
        {
            self.membrane.wasm_dealloc_buffer(buffer_id.clone())?;
        }

        buffers.clear();

        Ok(())
    }
}

impl  Drop for WasmBufferLocker
{
    fn drop(&mut self) {
        self.remove_all();
    }
}






#[cfg(test)]
mod test
{
    use std::fs::File;
    use std::io::Read;
    use std::sync::Arc;

    use wasmer::{Cranelift, JIT, Module, Store};

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
        let mut membrane = WasmMembrane::new(module, Node::default_cache()).unwrap();
        membrane.init();

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




}