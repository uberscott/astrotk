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
use std::cell::{Cell, RefCell};
use crate::error::Error;
use std::borrow::{BorrowMut, Borrow};
use mechtron_core::message::Message;

pub struct Node<'configs> {
    pub local: Local<'configs>,
    pub net: Arc<Network>,
}

impl <'configs> Node<'configs> {

    pub fn new<'get>() -> Node<'get> {
        let network = Arc::new(Network::new());
        let mut rtn = Node {
            local: Local::new(network.clone() ),
            net: network.clone()
        };
        rtn
    }


    pub fn local<'get>(&'get self) -> &'get Local<'configs> {
        &self.local
    }

    pub fn configs<'get>(&'get self) -> &'get Configs<'configs> {
        &self.local().configs()
    }

    pub fn net(&self) -> &Network {
        &self.net
    }

}

impl <'configs> Drop for Node<'configs>
{
    fn drop(&mut self) {
        self.local.destroy();
    }
}

impl <'configs> NodeContext for Node<'configs>{

}

trait NodeContext
{

}

pub struct Local<'configs> {
    wasm_store: Arc<Store>,
    configs: Configs<'configs>,
    wasms: Keeper<Module>,
    nuclei: Nuclei<'configs>,
    net: Arc<Network>
}

impl <'configs> Local <'configs>{
    fn new(network: Arc<Network>) -> Self {
        let repo = Arc::new(FileSystemArtifactRepository::new("../../repo/"));
        let wasm_store = Arc::new(Store::new(&JIT::new(Cranelift::default()).engine()));


        let rtn  = Local {
            net: network,
            wasm_store: wasm_store.clone(),
            configs: Configs::new(repo.clone() ),
            wasms: Keeper::new(
                repo.clone(),
                Box::new(WasmModuleParser {
                    wasm_store: wasm_store.clone(),
                },
                ),
                Option::None
            ),
            nuclei: Nuclei::new(),
        };

        rtn
    }

    pub fn nuclei<'get>(&'get self)->&'get Nuclei<'configs>
    {
        //&self.nuclei.into_inner()
        unimplemented!()
    }

    pub fn node(&'configs self) -> Arc<Node<'configs>>
    {
    //    self.node.swap()
        unimplemented!()
    }

    fn init(&'configs self, node: &'configs Node<'configs>) ->Result<(),Error> {
        self.nuclei.init(self);
        Ok(())
    }

    fn cache_core(&mut self)->Result<(),Error>
    {
        self.configs.cache_core()?;
        Ok(())
    }

    pub fn configs<'get>(&'get self) -> &'get Configs<'configs> {
        &self.configs
    }

    pub fn router<'get>(&'get self)->&'get (dyn Router+'configs)
    {
        return self
    }

    pub fn destroy(& mut self)
    {
//        self.router = Option::None;
    }
}

impl <'configs> Router for Local<'configs>
{
    fn send(&self, message: Arc<Message>) {
        unimplemented!()
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
    fn parse(&self, artifact: &Artifact, str: &str) -> Result<Module, mechtron_core::error::Error> {
        let result = Module::new(&self.wasm_store, str);
        match result {
            Ok(module) => Ok(module),
            Err(e) => Err("wasm compile error".into()),
        }
    }
}
