pub mod membrane;
pub mod mechtron;

#[macro_use]
extern crate wasm_bindgen;

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate mechtron_common;

use wasm_bindgen::prelude::*;
use crate::mechtron::{Context, Mechtron, StateLocker};
use crate::membrane::{log, mechtronium_cache, wasm_write_string, mechtronium_load, mechtronium_consume_buffer};
use mechtron_common::error::Error;
use std::sync::{RwLock, Arc,MutexGuard,Mutex};
use std::collections::HashSet;
use mechtron_common::artifact::{Artifact, ArtifactCache};
use mechtron_common::configs::Configs;
use mechtron_common::core::*;
use mechtron_common::mechtron::Context;
use std::rc::Rc;



lazy_static! {
  pub static ref CONFIGS: Configs<'static> = Configs::new(Arc::new(WasmArtifactRepository::new()));
}

extern "C"
{
    fn mechtron_init();
    pub fn mechtron(kind: &str, context: Context, state: Rc<StateLocker> )->Option<Box<dyn Mechtron>>;
}

#[wasm_bindgen]
pub fn mechtron_test_cache()
{
    match CONFIGS.artifacts.cache(&CORE_SCHEMA_META_API)
    {
        Ok(_) => {
            log( "cache", "cache worked");
            match CONFIGS.artifacts.load(&CORE_SCHEMA_META_API)
            {
                Ok(l) => {
                    log( "cache", "loaded bytes");
                }
                Err(e) => {

                    log( "cache", format!("load bytes failed! {:?}", e).as_str());
                }
            }
        }
        Err(_) => {
            log( "cache", "cache failed");
        }
    }
}

#[wasm_bindgen]
pub fn wasm_test_panic()
{
    log("wasm", "testing panic!");
    panic!()
}

#[wasm_bindgen]
pub fn wasm_test_ok()
{
    log("wasm", "testing ok");
}





                             #[wasm_bindgen]
pub fn mechtron_message_port(kind: i32,
                             state: i32,
                             port: i32,
                             message: i32) -> i32
{
    match mechtron_message_port_result(kind,state,port,message)
    {
        Ok(rtn) => rtn,
        Err(e) => {
            panic!(e);
        }
    }
}



fn mechtron_message_port_result(kind : i32,
                                state: i32,
                                port: i32,
                                message: i32 ) -> Result<i32,Error>
{
/*    let kind = mechtronium_consume_string(kind)?;
    let port = mechtronium_consume_string(port)?;
    let message = mechtronium_consume_buffer(message)?;
    let mut state = mechtronium_consume_buffer(state)?;

//    mechtron_kernel(kind);

    mem::forget(state );
    mem::forget(responses );

    Ok(responses)

 */
    Ok(0)
}


pub struct WasmArtifactRepository
{
    cache: RwLock<HashSet<Artifact>>
}

impl WasmArtifactRepository
{
    pub fn new()->Self
    {
        WasmArtifactRepository{
            cache: RwLock::new(HashSet::new())
        }
    }
}

impl ArtifactCache for WasmArtifactRepository
{

    fn cache(&self, artifact: &Artifact) -> Result<(), Error> {
        let mut cache = self.cache.write()?;

        if cache.contains(artifact)
        {
            return Ok(());
        }
        log("mechtron", format!("caching: {}",artifact.to()).as_str());

        let artifact_string_id = wasm_write_string(artifact.to() );
        unsafe{ mechtronium_cache( artifact_string_id ) };

        cache.insert( artifact.clone() );
        Ok(())
    }

    fn load(&self, artifact: &Artifact) -> Result<Vec<u8>, Error> {
        {
            let cache = self.cache.read()?;
            if !cache.contains(artifact)
            {
                return Err(format!("must call cache before load: {}",artifact.to()).into());
            }
        }

        let artifact_string_id = wasm_write_string(artifact.to() );
        let buffer_id = unsafe{ mechtronium_load( artifact_string_id ) };
        let buffer = mechtronium_consume_buffer(buffer_id)?;
        Ok(buffer)
    }

}

