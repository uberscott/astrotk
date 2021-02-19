use std::sync::Arc;

use wasmer::{Module, Store};

use mechtron_core::configs::{Configs, Keeper};

pub struct Cache<'configs>
{
    pub wasm_store: Arc<Store>,
    pub configs: Configs<'configs>,
    pub wasms: Keeper<Module>,
}

impl<'configs> Cache<'configs>
{}