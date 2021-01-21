

use std::fs::File;
use std::io::prelude::*;

mod wasm;
mod data;
mod artifact;
mod actor;

use crate::data::{AstroTK};
use crate::wasm::{WasmBinder, Buffers, WasmActorEngine};
use astrotk_config::artifact_config::ArtifactFile;
use astrotk_config::actor_config::ActorConfig;
use crate::actor::ActorEngine;

fn main() -> Result<(),Box<std::error::Error>>{

    let mut astroTK = data::new();

    let artifact_file = ArtifactFile::from( "astrotk:actor:1.0.0-alpha:actor.yaml" )?;
    astroTK.load_actor_config(&artifact_file)?;
    let actor_config = astroTK.get_actor_config(&artifact_file).unwrap();
    let mut actor = WasmActorEngine::new(&astroTK, actor_config)?;

    let mut create_message = astroTK.create_buffer(&actor_config.create_message)?;
    create_message.set( &[&"name"], "Scott Williams");
    create_message.set( &[&"age"], 48 );
    create_message.list_push( &[&"tags"], "tag A" );
    create_message.list_push( &[&"tags"], "tag B" );

    actor.create( &create_message )?;

    return Ok(());

}


