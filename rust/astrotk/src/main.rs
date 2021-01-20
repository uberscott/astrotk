

use std::fs::File;
use std::io::prelude::*;

mod wasm;
mod data;
mod artifact;

use crate::data::{AstroTK};
use crate::wasm::{WasmBinder, Buffers, WasmActor};
use astrotk_config::artifact_config::ArtifactFile;
use astrotk_config::actor_config::ActorConfig;

fn main() -> Result<(),Box<std::error::Error>>{

    let mut astroTK = data::new();

    let artifact_file = ArtifactFile::from( "astrotk:actor:1.0.0-alpha:actor.yaml" )?;
    astroTK.load_actor_config(&artifact_file)?;
    let actor_config = astroTK.get_actor_config(&artifact_file).unwrap();
    let actor = WasmActor::create(&astroTK,actor_config);

    return Ok(());

}


