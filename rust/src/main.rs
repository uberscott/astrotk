
extern crate yaml_rust;
#[macro_use]
extern crate lazy_static;

use yaml_rust::YamlLoader;
use std::fs::File;
use std::io::prelude::*;

mod actor;
mod data;

use crate::data::{AstroTK};
use crate::actor::{WasmBinder,Buffers};
use std::convert::TryInto;

fn main(){

    let mut astroTK = data::new();
    {
        let mut buffer = astroTK.create_buffer("src/content-config.json");

        let rtn = buffer.set(&["name"], "There is no name for this").unwrap();

        println!("{}", buffer.get::<&str>(&["name"]).unwrap().unwrap());
    }

    astroTK.load_wasm_module( "../wasm/example/pkg/example_bg.wasm");

    let mut wasm = WasmBinder::new( astroTK.get_wasm_module("../wasm/example/pkg/example_bg.wasm").unwrap() ).unwrap();





}
