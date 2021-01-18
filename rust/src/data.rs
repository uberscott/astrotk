use yaml_rust::Yaml;
use std::borrow::Borrow;
use no_proto::error::NP_Error;
use no_proto::NP_Factory;
use no_proto::collection::table::NP_Table;
use no_proto::buffer::NP_Buffer;
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use no_proto::pointer::{NP_Value, NP_Scalar};
use wasmer::{Module, Store, Cranelift, JIT};

struct TK_Buffer_Struct<'a>
{
    buffer: NP_Buffer<'a>
}

pub trait TK_Buffer
{
}

pub struct AstroTK<'a>
{
    buffer_factories : HashMap<String, NP_Factory<'a>>,
    wasm_modules : HashMap<String, Module>,
    wasm_store: Store
}

impl <'a> AstroTK <'a> {

  pub fn create_buffer(&mut self, name:&str)->NP_Buffer{
      return self.get_buffer_factory(name).empty_buffer(None);
  }

  pub fn get_buffer_factory(&mut self, name:&str)->&NP_Factory
  {
      if ! self.buffer_factories.contains_key(name )
      {
          let mut file = File::open(name).expect("Unable to open file");

          let mut contents = String::new();

          file.read_to_string(&mut contents)
              .expect("Unable to read file");

          let factory = NP_Factory::new(contents).unwrap();
          self.buffer_factories.insert(name.to_string(), factory);
      }

      return &self.buffer_factories.get(name ).unwrap();
  }

    pub fn load_wasm_module(&mut self, name:&str)
    {
      let mut file = File::open(name).expect("Unable to open file");
      let mut data = Vec::new();
      file.read_to_end(&mut data);
      let module = Module::new( &self.wasm_store, data ).unwrap();
      self.wasm_modules.insert(name.to_string(), module);
    }

    pub fn get_wasm_module(&self, name:&str)->Option<&Module>
    {
        return self.wasm_modules.get(name);
    }

}

pub fn new()->AstroTK<'static>
{
    return AstroTK {
        buffer_factories: HashMap::new(),
        wasm_modules: HashMap::new(),
        wasm_store: Store::new(&JIT::new(Cranelift::default()).engine())
    }
}




