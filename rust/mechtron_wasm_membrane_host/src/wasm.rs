use std::any::Any;
use std::borrow::{Borrow, BorrowMut};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::fmt::Debug;
use std::ops::Deref;
use std::pin::Pin;
use std::rc::Rc;
use std::string::FromUtf8Error;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, Weak, RwLock};

use wasmer::{imports, Array, Function, FunctionType, ImportObject, Instance, Module, Resolver, Val, ValType, Value, WasmPtr, WasmerEnv, InstantiationError, ExportError, NativeFunc, RuntimeError};
use crate::error::Error;

pub struct WasmMembrane{
    module: Module,
    instance: Instance,
    guest: Arc<RwLock<WasmGuest>>,
    host:  Arc<RwLock<WasmHost>>,
}

impl WasmMembrane{
    /*
    fn alloc_buffer(&mut self, len: i32) -> i32 {
        return self
            .instance
            .exports
            .get_native_function::<i32, i32>("alloc_buffer")
            .unwrap()
            .call(len)
            .unwrap();
    }
     */

    pub fn write_string(&self, string: &str )->Result<WasmBuffer,Error>
    {
        let string = string.as_bytes();
        let mut memory = self.instance.exports.get_memory("memory")?;
        let buffer = self.wasm_alloc(string.len() as _ )?;
        let values = buffer.ptr.deref(memory, 0, string.len() as u32).unwrap();
        for i in 0..string.len() {
            values[i].set(string[i] );
        }

        Ok(buffer)
    }

    pub fn write_buffer(&self, bytes: Vec<u8> )->Result<WasmBuffer,Error>
    {
        let mut memory = self.instance.exports.get_memory("memory")?;
        let buffer = self.wasm_alloc(bytes.len() as _ )?;
        let values = buffer.ptr.deref(memory, 0, bytes.len() as u32).unwrap();
        for i in 0..bytes.len() {
            values[i].set(bytes[i] );
        }

        Ok(buffer)
    }


    pub fn wasm_alloc( &self, len: i32 )->Result<WasmBuffer,Error>
    {
        let buffer = WasmBuffer::new( self.instance.exports.get_native_function::<i32,WasmPtr<u8,Array>>("wasm_alloc").unwrap().call(len.clone())?, len as u32);
        Ok(buffer)
    }

    pub fn wasm_dealloc(&self, buffer: WasmBuffer ) ->Result<(),Error>
    {
        self.instance.exports.get_native_function::<(WasmPtr<u8,Array>,i32),()>("wasm_dealloc").unwrap().call(buffer.ptr,buffer.len as _)?;
        Ok(())
    }

    pub fn wasm_cache(&self, key: WasmBuffer, buffer: WasmBuffer ) ->Result<(),Error>
    {
        self.instance.exports.get_native_function::<(WasmPtr<u8,Array>,i32,WasmPtr<u8,Array>,i32),()>("wasm_cache").unwrap().call(key.ptr,key.len as _,buffer.ptr,buffer.len as _)?;
        Ok(())
    }

    pub fn wasm_test_log(&self, message: &str)->Result<(),Error>
    {
        let buffer = self.write_string(message )?;
        self.instance.exports.get_native_function::<(WasmPtr<u8,Array>,i32),()>("wasm_test_log").unwrap().call(buffer.ptr,buffer.len as _)?;
        Ok(())
    }

    pub fn wasm_test_cache(&self, message: &str)->Result<(),Error>
    {
        let buffer = self.write_string(message )?;
        self.instance.exports.get_native_function::<(WasmPtr<u8,Array>,i32),()>("wasm_test_cache").unwrap().call(buffer.ptr,buffer.len as _)?;
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

struct WasmGuest {
    membrane: Option<Weak<WasmMembrane>>,
}

impl WasmGuest {

    fn new() ->Self
    {
        WasmGuest{
            membrane: Option::None,
        }
    }

}

struct WasmHost {
    membrane: Option<Weak<WasmMembrane>>,
    buffers: HashMap<i32,Vec<u8>>
}


impl WasmHost{

    fn new() ->Self
    {
        WasmHost{
            membrane: Option::None,
            buffers: HashMap::new()
        }
    }

    fn panic(&self, error: Error)
    {
        println!("{:?}",error);
    }
}

#[derive(WasmerEnv, Clone)]
struct Env {
    host: Arc<RwLock<WasmHost>>,
}

impl WasmMembrane {
    pub fn new(module: Module) -> Result<Arc<Self>, Error> {

        let mut guest = Arc::new(RwLock::new(WasmGuest::new() ));
        let mut host  = Arc::new(RwLock::new(WasmHost::new()));

        let imports = imports! { "env"=>{
        "mechtronium_log"=>Function::new_native_with_env(module.store(),Env{host:host.clone()},|env:&Env,ptr:WasmPtr<u8,Array>,len:i32| {
                let host = env.host.read();
                if host.is_err( )
                {
                  return;
                }

                let host = host.unwrap();

                let membrane = host.membrane.as_ref();
                if membrane.is_none()
                {
                  return;
                }
                let membrane = membrane.unwrap().upgrade();

                if membrane.is_none()
                {
                  return;
                }
                let membrane = membrane.unwrap();

                let mut memory = membrane.instance.exports.get_memory("memory");

                if memory.is_err()
                {
                  return;
                }

                let memory = memory.unwrap();

                let str = ptr.get_utf8_string(memory, len as u32).unwrap();
                println!("FROM WEBASSEMBLY: {}",str);
            }),
            "mechtronium_cache"=>Function::new_native_with_env(module.store(),Env{host:host.clone()},|env:&Env,ptr:WasmPtr<u8,Array>,len:i32| {
                let host = env.host.read();
                if host.is_err( )
                {
                  return;
                }

                let host = host.unwrap();

                let membrane = host.membrane.as_ref();
                if membrane.is_none()
                {
                  return;
                }
                let membrane = membrane.unwrap().upgrade();

                if membrane.is_none()
                {
                  return;
                }
                let membrane = membrane.unwrap();

                let mut memory = membrane.instance.exports.get_memory("memory");

                if memory.is_err()
                {
                  return;
                }

                let memory = memory.unwrap();

                let str = ptr.get_utf8_string(memory, len as u32).unwrap();

                let key = membrane.write_string("cache_key");
                if key.is_err()
                {
                  return;
                }
                let key = key.unwrap();

                let buffer = membrane.write_buffer( "some buffer".as_bytes().to_vec() );
                if buffer.is_err()
                {
                  return;
                }
                let buffer = buffer.unwrap();

                membrane.wasm_cache(key,buffer);
            })

        } };


//        let imports = imports!{};

        let instance = Instance::new(&module, &imports)?;

        let mut membrane = Arc::new(WasmMembrane{
            module: module,
            instance: instance,
            guest: guest.clone(),
            host: host.clone(),
        });

        {
            guest.write().unwrap().membrane = Option::Some(Arc::downgrade(&membrane));
        }
        {
            host.write().unwrap().membrane = Option::Some(Arc::downgrade(&membrane));
        }

        return Ok(membrane);
    }


    fn log(&self, ptr: i32, len: i32) {}
}

#[cfg(test)]
mod test
{
    use wasmer::{Store, JIT, Cranelift, Module};
    use std::fs::File;
    use std::io::Read;
    use crate::error::Error;
    use crate::wasm::WasmMembrane;

    #[test]
    fn test_wasm() -> Result<(), Error>
    {
        let path = "../../repo/mechtron.io/examples/0.0.1/hello-world/wasm/hello-world.wasm";

        let mut file = File::open(path)?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;

        let store = Store::new(&JIT::new(Cranelift::default()).engine());
        println!("Compiling module...");
        let module = Module::new(&store, data)?;

        let membrane = WasmMembrane::new(module).unwrap();

        let len = 2048;
        let buffer = membrane.wasm_alloc(len.clone())?;

        let memory = membrane.instance.exports.get_memory("memory")?;

        membrane.wasm_dealloc(buffer)?;

        membrane.write_string("Hello From Mechtronium");

        Ok(())
    }


    #[test]
    fn test_logs() -> Result<(), Error>
    {
        let path = "../../repo/mechtron.io/examples/0.0.1/hello-world/wasm/hello-world.wasm";

        let mut file = File::open(path)?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;

        let store = Store::new(&JIT::new(Cranelift::default()).engine());
        println!("Compiling module...");
        let module = Module::new(&store, data)?;

        let membrane = WasmMembrane::new(module).unwrap();

        membrane.wasm_test_log("Helllo this is a log");

        Ok(())
    }


    #[test]
    fn test_cache() -> Result<(), Error>
    {
        let path = "../../repo/mechtron.io/examples/0.0.1/hello-world/wasm/hello-world.wasm";

        let mut file = File::open(path)?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;

        let store = Store::new(&JIT::new(Cranelift::default()).engine());
        println!("Compiling module...");
        let module = Module::new(&store, data)?;

        let membrane = WasmMembrane::new(module).unwrap();

        membrane.wasm_test_cache("CACHE TEST");

        Ok(())
    }
}