use crate::artifact::FileSystemArtifactRepository;
use crate::router::{GlobalRouter, Router};
use crate::nucleus::{Nuclei, Nucleus};
use mechtron_core::artifact::Artifact;
use mechtron_core::configs::{Configs, Keeper, Parser};
use mechtron_core::id::{Id, IdSeq};
use std::alloc::System;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use wasmer::{Cranelift, Module, Store, JIT};
use std::cell::Cell;
use mechtron_core::error::Error;

pub struct Node<'configs> {
    pub local: Local<'configs>,
    pub net: Network,
    pub router: GlobalRouter<'configs>,
}

impl <'configs> Node<'configs> {


    fn new() -> Self {
        let rtn = Node {
            local: Local::new(),
            net: Network::new(),
            router: GlobalRouter::new(),
        };

        //rtn.init()

        rtn
    }

    fn init(&'configs self) {
        self.local.init(self);
    }

    pub fn local<'get>(&'get self) -> &'get Local<'configs> {
        &self.local
    }

    pub fn configs<'get>(&'get self) -> &'get Configs<'configs> {
        &self.local.configs
    }

    pub fn net(&self) -> &Network {
        &self.net
    }

    pub fn router<'get>(&'get self) -> &'get dyn Router
    {
        &self.router
    }
}

pub struct Local<'configs> {
    wasm_store: Arc<Store>,
    configs: Configs<'configs>,
    wasm_module_keeper: Keeper<Module>,
    nuclei: Nuclei<'configs>,
    node: Cell<Option<&'configs Node<'configs>>>
}


impl <'configs> Local <'configs>{
    fn new() -> Self {
        let repo = Arc::new(FileSystemArtifactRepository::new("../../repo/"));
        let wasm_store = Arc::new(Store::new(&JIT::new(Cranelift::default()).engine()));

        let rtn  = Local {
            node: Cell::new(Option::None),
            wasm_store: wasm_store.clone(),
            configs: Configs::new(repo.clone() ),
            wasm_module_keeper: Keeper::new(
                repo.clone(),
                Box::new(WasmModuleParser {
                    wasm_store: wasm_store.clone(),
                }),
            ),
            nuclei: Nuclei::new()
        };

        rtn
    }

    pub fn nuclei<'get>(&'get self)->&'get Nuclei<'configs>
    {
        &self.nuclei
    }

    pub fn node(&'configs self) -> &Node<'configs>
    {
        self.node.get().unwrap()
    }


    fn init(&'configs self,  sys: &'configs Node<'configs>) {
        self.node.set(Option::Some(sys) );
        self.nuclei.init(self)
    }

    pub fn configs<'get>(&'get self) -> &'get Configs<'configs> {
        &self.configs
    }
}

#[derive(Clone)]
pub struct NucleusContext<'context> {
    sys: Arc<Node<'context>>,
}

impl <'context> NucleusContext <'context>{
    pub fn new(sys: Arc<Node<'context>>) -> Self {
        NucleusContext { sys: sys }
    }
    pub fn sys<'get>(&'get self) -> Arc<Node<'context>> {

        self.sys.clone()
    }
}

pub struct Network {
    seq: Arc<IdSeq>,
}

impl Network {
    fn new() -> Self {
        Network {
            seq: Arc::new(IdSeq::new(0)),
        }
    }

    pub fn seq(&self)-> Arc<IdSeq>
    {
        self.seq.clone()
    }
}
struct WasmModuleParser {
    wasm_store: Arc<Store>,
}

impl Parser<Module> for WasmModuleParser {
    fn parse(&self, artifact: &Artifact, str: &str) -> Result<Module, Error> {
        let result = Module::new(&self.wasm_store, str);
        match result {
            Ok(module) => Ok(module),
            Err(e) => Err("wasm compile error".into()),
        }
    }
}
