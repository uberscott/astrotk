use crate::artifact::Artifact;
use crate::buffers::{Buffer, BufferFactories, ReadOnlyBuffer, Path};
use crate::configs::{Configs, Keeper, MechtronConfig, BindConfig};
use crate::core::*;
use crate::id::{StateKey, Id, MechtronKey};
use crate::message::Payload;
use no_proto::buffer::{NP_Buffer, NP_Finished_Buffer};
use no_proto::error::NP_Error;
use no_proto::memory::{NP_Memory_Owned, NP_Memory_Ref};
use no_proto::NP_Factory;
use std::rc::Rc;
use std::sync::Arc;
use crate::error::Error;
use no_proto::pointer::bytes::NP_Bytes;
use std::collections::HashMap;

#[derive(Clone)]
pub struct State {
    pub config: Arc<MechtronConfig>,
    pub meta: Buffer,
    pub buffers: HashMap<String,Buffer>,
    pub ext: HashMap<String,Buffer>,
    pub scratch: HashMap<String,Buffer>
}

impl State {
    pub fn new_buffers(configs: &Configs, config: Arc<MechtronConfig>) -> Result<HashMap<String,Buffer>, Error> {
        let bind = configs.binds.get( &config.bind.artifact )?;

        let mut buffers = HashMap::new();
        for buffer_config in &bind.state.buffers
        {
            let buffer= Buffer::new(
                configs
                    .schemas
                    .get(&buffer_config.artifact)?
                    .new_buffer(Option::None),
            );
            buffers.insert( buffer_config.name.clone() , buffer );
        }


        Ok(buffers)
    }

    pub fn new(configs: &Configs, config: Arc<MechtronConfig>) -> Result<Self, Error> {

        let mut meta = Buffer::new(
            configs
                .schemas
                .get(&CORE_SCHEMA_META_STATE)?
                .new_buffer(Option::None),
        );

        meta.set(&path!["config"], config.source.to());

        let mut buffers = State::new_buffers(configs,config.clone())?;

        Ok(State {
            meta: meta,
            buffers: buffers,
            scratch: HashMap::new(),
            ext: HashMap::new(),
            config: config.clone()
        })
    }


    pub fn new_from_meta(
        configs: &Configs,
        meta: Buffer
    ) -> Result<Self,Error> {

        let config = configs.mechtrons.get(&Artifact::from(meta.get( &path!["config"] )? )? )?;
        let mut buffers = State::new_buffers(configs,config.clone())?;

        Ok(State {
            meta: meta,
            buffers: buffers,
            ext: HashMap::new(),
            scratch: HashMap::new(),
            config:config
        })
    }


    pub fn from(
        configs: &Configs,
        meta: Buffer,
        buffers: HashMap<String,Buffer>,
    ) -> Result<Self,Error> {

        let source = meta.get::<String>(&path!["mechtron_config"])?;
        let source = Artifact::from(source.as_str() )?;
        let config = configs.mechtrons.get( &source )?;

        Ok(State {
            meta: meta,
            buffers: buffers,
            ext: HashMap::new(),
            scratch: HashMap::new(),
            config:config
        })
    }

    pub fn to_bytes(&self, configs: &Configs ) ->Result<Vec<u8>,Error>
    {
        let mut buffer = {
            let size = self.meta.len() +  128;
            let factory = configs.schemas.get(&CORE_SCHEMA_STATE)?;
            let buffer = factory.new_buffer(Option::Some(size));
            let mut buffer = Buffer::new(buffer);
            buffer
        };

        buffer.set( &path!["config"], self.config.source.to() );

        buffer.set( &path!["meta"], self.meta.read_bytes() )?;

        let path = Path::new( path!["buffers"]);
        for key in self.buffers.keys()
        {
            buffer.set(&path.with(path![key]), self.buffers.get(key).unwrap().read_bytes())?;
        }

        Ok(Buffer::bytes(buffer))
    }

    pub fn from_bytes(bytes: Vec<u8>, configs: &Configs ) ->Result<Self,Error>
    {
        let buffer = {
            let factory = configs.schemas.get(&CORE_SCHEMA_STATE)?;
            let buffer = factory.open_buffer(bytes);
            let buffer = Buffer::new(buffer);
            buffer
        };

        let (config, bind) = {
            let config = buffer.get::<String>(&path!["config"])?;
            let config = Artifact::from(&config)?;
            let config = configs.mechtrons.get(&config)?;
            let bind = configs.binds.get(&config.bind.artifact)?;
            (config,bind)
        };

        let meta = {
            let factory = configs.schemas.get(&CORE_SCHEMA_META_STATE)?;
            let meta = buffer.get::<Vec<u8>>(&path!["meta"])?;
            let meta = factory.open_buffer(meta);
            let meta = Buffer::new(meta);
            meta
        };

        let buffers = {
            let mut buffers = HashMap::new();
            let path = Path::new(path!["buffers"]);
            for key in &buffer.get_keys(&path!["buffers"])?.unwrap()
            {
                let buffer_config = bind.state.get_buffer(key.clone());
                if buffer_config.is_some()
                {
                    let buffer_config = buffer_config.unwrap();
                    let factory = configs.schemas.get(&buffer_config.artifact)?;
                    let buffer = buffer.get::<Vec<u8>>(&path.with(path![key.clone()]))?;
                    let buffer = factory.open_buffer(buffer);
                    let buffer = Buffer::new(buffer);
                    buffers.insert(key.clone(), buffer);
                }
                else {
                    println!("bad buffer config {}",key.clone());
                }
            }
            buffers
        };

        Ok(State{
            meta: meta,
            buffers: buffers,
            config: config,
            scratch:HashMap::new(),
            ext: HashMap::new()
        })
    }

    pub fn is_tainted(&self) -> Result<bool, Error> {
        Ok(self.meta.get(&path!["taint"])?)
    }

    pub fn read_only(&self) -> Result<ReadOnlyState, Error> {
        let mut buffers: HashMap<String,ReadOnlyBuffer> = HashMap::new();
        for key in self.buffers.keys()
        {
            buffers.insert(key.clone(), self.buffers.get(key).unwrap().read_only());
        }
        Ok(ReadOnlyState {
            config: self.config.clone(),
            meta: self.meta.read_only(),
            buffers: buffers
        })
    }

    pub fn compact(&mut self) -> Result<(), Error> {
        self.meta.compact()?;
//        self.data.compact()?;

        Ok(())
    }
}

impl StateMeta for State {

    fn set_mechtron_config(&mut self, config: Arc<MechtronConfig>) -> Result<(), Error> {
        Ok(self.meta.set(&path!["mechtron_config"], config.source.to())?)
    }

    fn set_creation_timestamp(&mut self, value: i64) -> Result<(), Error> {
        Ok(self.meta.set(&path!["creation_timestamp"], value)?)
    }

    fn set_creation_cycle(&mut self, value: i64) -> Result<(), Error> {
        Ok(self.meta.set(&path!["creation_cycle"], value)?)
    }

    fn set_taint( &mut self, taint: bool )
    {
        self.meta.set(&path!["taint"], taint );
    }


}

impl ReadOnlyStateMeta for State {
    fn get_mechtron_id(&self) -> Result<Id, Error> {
      Ok(Id::from_buffer( &Path::new(path!["id"] ), &self.meta )?)
    }

    fn get_mechtron_config(&self) -> Arc<MechtronConfig>{
        self.config.clone()
    }

    fn get_creation_timestamp(&self) -> Result<i64, Error> {
        Ok(self.meta.get(&path!["creation_timestamp"])?)
    }

    fn get_creation_cycle(&self) -> Result<i64, Error> {
        Ok(self.meta.get(&path!["creation_cycle"])?)
    }

    fn is_tainted(&self) -> Result<bool, Error> {
        Ok(self.meta.get(&path!["taint"])?)
    }
}

#[derive(Clone)]
pub struct ReadOnlyState {
    pub config: Arc<MechtronConfig>,
    pub meta: ReadOnlyBuffer,
    pub buffers: HashMap<String,ReadOnlyBuffer>,
}

impl ReadOnlyState {
    pub fn copy(&self) -> State {

        let mut buffers = HashMap::new();
        for key in self.buffers.keys()
        {
            buffers.insert( key.clone(), self.buffers.get(key).unwrap().copy_to_buffer() );
        }

        State {
            config: self.config.clone(),
            meta: self.meta.copy_to_buffer(),
            buffers : buffers,
            ext: HashMap::new(),
            scratch: HashMap::new()
        }
    }

    pub fn convert_to_payloads(
        configs: &Configs,
        state: ReadOnlyState,
    ) -> Result<Vec<Payload>, Error> {

        let bind = configs.binds.get(&state.config.bind.artifact)?;
        let rtn: Vec<Payload> = vec![
            Payload {
                buffer: state.meta,
                schema: CORE_SCHEMA_META_STATE.clone(),
            }
            // need Payloads::convert ...
        ];

        return Ok(rtn);
    }

    pub fn to_bytes(&self, configs: &Configs ) ->Result<Vec<u8>,Error>
    {
        let mut buffer = {
            let factory = configs.schemas.get(&CORE_SCHEMA_STATE)?;
            let buffer = factory.new_buffer(Option::None );
            let mut buffer = Buffer::new(buffer);
            buffer
        };

        buffer.set( &path!["config"], self.config.source.to() );

        buffer.set( &path!["meta"], self.meta.read_bytes() )?;

        let path = Path::new( path!["buffers"]);
        for key in self.buffers.keys()
        {
            buffer.set(&path.with(path![key]), self.buffers.get(key).unwrap().read_bytes())?;
        }

        Ok(Buffer::bytes(buffer))
    }
}

impl ReadOnlyStateMeta for ReadOnlyState {

    fn get_mechtron_id(&self) -> Result<Id, Error> {
        Ok(Id::from( &Path::new(path!["id"] ), &self.meta )?)
    }

    fn get_mechtron_config(&self) -> Arc<MechtronConfig> {
        self.config.clone()
    }

    fn get_creation_timestamp(&self) -> Result<i64, Error> {
        Ok(self.meta.get(&path!["creation_timestamp"])?)
    }

    fn get_creation_cycle(&self) -> Result<i64, Error> {
        Ok(self.meta.get(&path!["creation_cycle"])?)
    }

    fn is_tainted(&self) -> Result<bool, Error> {
        Ok(self.meta.get(&path!["taint"])?)
    }

}

pub trait ReadOnlyStateMeta {
    fn get_mechtron_id(&self) -> Result<Id, Error>;
    fn get_mechtron_config(&self) -> Arc<MechtronConfig>;
    fn get_creation_timestamp(&self) -> Result<i64, Error>;
    fn get_creation_cycle(&self) -> Result<i64, Error>;
    fn is_tainted(&self) -> Result<bool, Error>;
}

pub trait StateMeta: ReadOnlyStateMeta {
    fn set_mechtron_config(&mut self, config: Arc<MechtronConfig>) -> Result<(), Error>;
    fn set_creation_timestamp(&mut self, value: i64) -> Result<(), Error>;
    fn set_creation_cycle(&mut self, value: i64) -> Result<(), Error>;
    fn set_taint( &mut self, taint: bool );
}


pub struct NeutronStateInterface {}

impl NeutronStateInterface {
    pub fn add_mechtron(&self, state: &mut State, key: &MechtronKey, kind: String) -> Result<(), Error> {
        println!("ADD MECHTRON...{}",kind);
        let index = {
            match state.buffers.get("data").unwrap().get_length(&path!("mechtrons"))
            {
                Ok(length) => {length}
                Err(_) => {0}
            }
        };

        let path = Path::new(path!["mechtrons", index.to_string()]);
        key.mechtron.append(&path.push(path!["id"]), &mut state.buffers.get_mut("data").unwrap())?;
        state.buffers.get_mut("data").unwrap().set(&path.with(path!["kind"]), kind)?;
        println!("MECHTRON ADDED...x");

        Ok(())
    }

    pub fn set_mechtron_name(
        &self,
        state: &mut State,
        name: &str,
        key: &MechtronKey,
    ) -> Result<(), Error> {
        key.append(&Path::new(path!["mechtron_names"]), &mut state.meta);
        Ok(())
    }


    pub fn set_mechtron_index
    (
        &self,
        state: &mut State,
        value: i64,
    ) -> Result<(), Error> {
        state.buffers.get_mut("data").unwrap().set( &path!["mechtron_index"], value );
        Ok(())
    }

    pub fn set_mechtron_seq_id(
        &self,
        state: &mut State,
        value: i64,
    ) -> Result<(), Error> {
        state.buffers.get_mut("data").unwrap().set( &path!["mechtron_seq_id"], value );
        Ok(())
    }
}



#[cfg(test)]
mod tests {
use crate::core::*;
use crate::state::{State, StateMeta};
use crate::configs::*;

    use std::sync::{Arc, RwLock};

    use no_proto::buffer::NP_Buffer;
    use no_proto::memory::NP_Memory_Ref;
    use no_proto::NP_Factory;

    use crate::artifact::{Artifact, ArtifactBundle, ArtifactRepository, ArtifactCache};
    use crate::buffers::{Buffer, BufferFactories, Path};
    use crate::error::Error;
    use crate::id::{Id, IdSeq, MechtronKey};
    use crate::core::*;
    use crate::message::tests::*;
    use std::collections::{HashSet, HashMap};
    use std::fs::File;
    use std::io::Read;
    use crate::configs::Configs;
    use crate::message::{To, Message, MessageKind, Cycle, DeliveryMoment, MechtronLayer, Payload, MessageBuilder};

    lazy_static!
    {
        static ref CONFIGS: Configs<'static>  = Configs::new(Arc::new(TestArtifactRepository::new("../../repo")));
    }

    #[test]
  pub fn test()
  {
      let config = CONFIGS.mechtrons.get(&CORE_MECHTRON_NEUTRON).unwrap();
      let mut state = State::new(&CONFIGS, config ).unwrap();
      state.set_taint(true);
      state.set_creation_timestamp(1);
      state.set_creation_cycle(2);

      let bytes = state.to_bytes(&CONFIGS ).unwrap();

      let new_state = State::from_bytes(bytes,&CONFIGS).unwrap();

      assert_eq!( state.is_tainted().unwrap(), new_state.is_tainted().unwrap() );
  }


}