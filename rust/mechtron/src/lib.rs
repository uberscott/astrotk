pub mod membrane;

#[macro_use]
extern crate wasm_bindgen;

#[macro_use]
extern crate lazy_static;

use wasm_bindgen::prelude::*;
use std::sync::atomic::Ordering;
use core::mem;
use mechtron_core::state::State;
use mechtron_core::artifact::{ArtifactRepository, ArtifactBundle, ArtifactCache, Artifact};
use std::sync::{Arc, RwLock};
use std::collections::{HashMap, HashSet};
use crate::membrane::{mechtronium_consume_string, mechtronium_consume_buffer, mechtronium_cache, wasm_write_string, mechtronium_load, log};
use mechtron_core::error::Error;
use mechtron_core::core::*;
use mechtron_core::configs::*;

lazy_static! {
  static ref CONFIGS: Configs<'static> = Configs::new(Arc::new(WasmArtifactRepository::new()));
}

extern "C"
{
    fn mechtron(kind: &str, state: State )->Box<dyn Mechtron>;
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

trait Mechtron
{

}
