use std::sync::{Arc, RwLock};

use wasmer::{Module, Store};

use mechtron_common::configs::{Configs, Keeper};
use crate::membrane::WasmMembrane;
use std::collections::HashMap;
use mechtron_common::artifact::{Artifact, ArtifactCache};
use crate::error::Error;

pub struct Cache
{
    pub configs: Arc<Configs>,
    pub wasms: Wasms
}

impl Cache
{
    pub fn new( configs: Arc<Configs>,
                store: Arc<Store>,
                modules: Keeper<Module>)->Self
    {
        let wasms = Wasms::new(store,modules,configs.clone());
        Cache{
            configs: configs,
            wasms: wasms
        }
    }


}

pub struct Wasms
{
    store: Arc<Store>,
    modules: Keeper<Module>,
    membranes: Membranes
}

impl Wasms
{
    pub fn new( store: Arc<Store>, modules: Keeper<Module>, configs: Arc<Configs>)->Self
    {
        Wasms{
            store: store,
            modules:  modules,
            membranes: Membranes::new(configs)
        }
    }
    pub fn cache(&self, artifact : &Artifact)->Result<(),Error>
    {
        if self.membranes.has(artifact)?
        {
            return Ok(());
        }

        self.modules.cache(artifact);

        let module = self.modules.get(artifact).unwrap();
        self.membranes.create( module,artifact )?;
        Ok(())
    }

    pub fn get_membrane( &self, artifact:&Artifact )->Result<Arc<WasmMembrane>,Error>
    {
        Ok(self.membranes.get(artifact)?)
    }
}


struct Membranes
{
    configs: Arc<Configs>,
    store: RwLock<HashMap<Artifact,Arc<WasmMembrane>>>
}

impl Membranes
{
    pub fn new( configs: Arc<Configs>)->Self
    {
        Membranes{
            configs: configs,
            store: RwLock::new(HashMap::new())
        }
    }
    pub fn has( &self, artifact:&Artifact ) -> Result<bool,Error>
    {
        let store = self.store.read()?;
        Ok(store.contains_key(artifact))
    }

    fn create( &self, module: Arc<Module>, artifact: &Artifact )->Result<(),Error>
    {
        let mut store = self.store.write()?;
        if store.contains_key(artifact )
        {
            return Ok(());
        }
        let membrane = WasmMembrane::new(module, self.configs.clone() )?;
        membrane.init()?;

        store.insert( artifact.clone(), membrane);

        Ok(())
    }

    fn get( &self, artifact: &Artifact )->Result<Arc<WasmMembrane>,Error>
    {
        let store = self.store.read()?;
        match store.get(artifact)
        {
            None => Err(format!("could not find artifact {:?}",artifact).into()),
            Some(membrane) => Ok(membrane.clone())
        }
    }
}
