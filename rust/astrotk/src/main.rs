use std::fs::File;
use std::io::prelude::*;

use astrotk_config::actor_config::ActorConfig;
use astrotk_config::artifact_config::ArtifactFile;
use astrotk_config::buffers::BufferFactories;
use astrotk_config::message::{Address, Message, MessageKind};

use crate::actor::ActorEngine;
use crate::data::AstroTK;
use crate::wasm::{Buffers, WasmActorEngine, WasmBinder};

mod wasm;
mod data;
mod artifact;
mod actor;

#[macro_use]
extern crate lazy_static;


fn main() -> Result<(),Box<std::error::Error>>{

    let mut astroTK = AstroTK::new();
    let artifact_file = ArtifactFile::from( "astrotk:actor:1.0.0-alpha:actor.yaml" )?;
    astroTK.load_actor_config(&artifact_file)?;
    let actor_config = astroTK.get_actor_config(&artifact_file).unwrap();
    let mut actor = WasmActorEngine::new(&astroTK, actor_config)?;

    let mut create_message = astroTK.create_buffer(&actor_config.create_message)?;
    create_message.set( &[&"name"], "Scott Williams");
    create_message.set( &[&"age"], 48 );
    create_message.list_push( &[&"tags"], "tag A" );
    create_message.list_push( &[&"tags"], "tag B" );

    let create_message = Message::new( MessageKind::Create,
    Address{actor:0},
    Address{actor:1},
     "hello".to_string(),
    create_message,
    ArtifactFile::from("astrotk:actor:1.0.0-alpha:content.json")?);

    actor.create( &create_message )?;

    return Ok(());

}


