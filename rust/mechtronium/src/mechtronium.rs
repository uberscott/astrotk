use crate::artifact::FileSystemArtifactRepository;
use crate::message::GlobalMessageRouter;
use crate::nucleus::{Nuclei, Nucleus};
use mechtron_core::artifact::Artifact;
use mechtron_core::configs::{Configs, Keeper, Parser};
use mechtron_core::id::{Id, IdSeq};
use std::alloc::System;
use std::collections::HashMap;
use std::error::Error;
use std::sync::{Arc, RwLock};
use wasmer::{Cranelift, Module, Store, JIT};

pub struct Mechtronium<'configs> {
    pub local: Local<'configs>,
    pub net: Network,
    pub router: GlobalMessageRouter<'configs>,
}

impl <'configs> Mechtronium <'configs> {
    fn new() -> Arc<Self> {
        /*
        let mut rtn = Arc::new(Mechtronium {
            local: Local::new(),
            net: Network::new(),
            router: GlobalMessageRouter::new(),
        });
        rtn.init(rtn.clone());
        rtn
         */
        unimplemented!()
    }

    fn init(&mut self, sys: Arc<Mechtronium<'configs>>) {
        self.local.init(sys.clone());
        self.router.init(sys.clone());
    }

    pub fn local<'get>(&'get self) -> &'get Local<'configs> {
        &self.local
    }

    pub fn configs<'get>(&'get self) -> &'get Configs<'configs> {
        &self.local.configs
    }

    pub fn net(&mut self) -> &mut Network {
        &mut self.net
    }
}

pub struct Local<'configs> {
    pub wasm_store: Arc<Store>,
    pub configs: Configs<'configs>,
    pub wasm_module_keeper: Keeper<Module>,
    pub nuclei: Nuclei<'configs>,
}

impl <'configs> Local <'configs>{
    fn new() -> Self {
        let repo = Arc::new(FileSystemArtifactRepository::new("../../repo/".to_string()));
        let wasm_store = Arc::new(Store::new(&JIT::new(Cranelift::default()).engine()));

        Local {
            wasm_store: wasm_store.clone(),
            configs: Configs::new(repo.clone()),
            wasm_module_keeper: Keeper::new(
                repo.clone(),
                Box::new(WasmModuleParser {
                    wasm_store: wasm_store.clone(),
                }),
            ),
            nuclei: Nuclei::new(),
        }
    }

    pub fn init(&mut self, sys: Arc<Mechtronium<'configs>>) {
        self.nuclei.init(sys);
    }

    pub fn configs<'get>(&'get self) -> &'get Configs<'configs> {
        &self.configs
    }
}

#[derive(Clone)]
pub struct NucleusContext<'context> {
    sys: Arc<Mechtronium<'context>>,
}

impl <'context> NucleusContext <'context>{
    pub fn new(sys: Arc<Mechtronium<'context>>) -> Self {
        NucleusContext { sys: sys }
    }
    pub fn sys<'get>(&'get self) -> Arc<Mechtronium<'context>> {

        self.sys.clone()
    }
}

pub struct Network {
    pub id_seq: IdSeq,
}

impl Network {
    fn new() -> Self {
        Network {
            id_seq: IdSeq::new(0),
        }
    }
}
struct WasmModuleParser {
    wasm_store: Arc<Store>,
}

impl Parser<Module> for WasmModuleParser {
    fn parse(&self, artifact: &Artifact, str: &str) -> Result<Module, Box<dyn Error>> {
        let result = Module::new(&self.wasm_store, str);
        match result {
            Ok(module) => Ok(module),
            Err(e) => Err(e.into()),
        }
    }
}
