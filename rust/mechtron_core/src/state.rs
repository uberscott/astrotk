use crate::artifact::Artifact;
use crate::buffers::{Buffer, BufferFactories, ReadOnlyBuffer, Path};
use crate::configs::{Configs, Keeper, MechtronConfig, BindConfig};
use crate::core::*;
use crate::id::{StateKey, Id};
use crate::message::Payload;
use no_proto::buffer::{NP_Buffer, NP_Finished_Buffer};
use no_proto::error::NP_Error;
use no_proto::memory::{NP_Memory_Owned, NP_Memory_Ref};
use no_proto::NP_Factory;
use std::rc::Rc;
use std::sync::Arc;
use crate::error::Error;

#[derive(Clone)]
pub struct State {
    pub meta: Buffer,
    pub data: Buffer,
    pub config: Arc<MechtronConfig>
}

impl State {

    pub fn new<'configs>(configs: &Configs<'configs>, config: Arc<MechtronConfig>) -> Result<Self, Error> {

        let bind = configs.binds.get( &config.bind.artifact )?;

        let mut meta = Buffer::new(
            configs
                .schemas
                .get(&CORE_SCHEMA_META_STATE)?
                .new_buffer(Option::None),
        );
        let data = Buffer::new(
            configs
                .schemas
                .get(&bind.state.artifact)?
                .new_buffer(Option::None),
        );
        meta.set(&path!["mechtron_config_artifact"], config.source.to());
        Ok(State {
            meta: meta,
            data: data,
            config: config
        })
    }


    pub fn new_from_meta(
        configs: &Configs,
        meta: Buffer
    ) -> Result<Self,Error> {

let taint= meta.get::<bool>(&path!["taint"])?;
println!("TAINT {} ",taint );
        let source = meta.get::<String>(&path!["mechtron_config"])?;
        let source = Artifact::from(source.as_str() )?;
        let config = configs.mechtrons.get( &source )?;
        let bind = configs.binds.get(&config.bind.artifact )?;
        let data_factory = configs.schemas.get( &bind.state.artifact )?;
        let buffer = data_factory.new_buffer(Option::None);
        let data = Buffer::new(buffer);

        Ok(State {
            meta: meta,
            data: data,
            config:config
        })
    }


    pub fn from<'configs>(
        configs: &Configs<'configs>,
        meta: Buffer,
        data: Buffer,
    ) -> Result<Self,Error> {

        let source = meta.get::<String>(&path!["mechtron_config"])?;
        let source = Artifact::from(source.as_str() )?;
        let config = configs.mechtrons.get( &source )?;

        Ok(State {
            meta: meta,
            data: data,
            config:config
        })


    }

    fn is_tainted(&self) -> Result<bool, Error> {
        Ok(self.meta.get(&path!["taint"])?)
    }

    pub fn read_only(&self) -> Result<ReadOnlyState, Error> {
        Ok(ReadOnlyState {
            config: self.config.clone(),
            meta: self.meta.read_only(),
            data: self.data.read_only(),
        })
    }

    pub fn compact(&mut self) -> Result<(), Error> {
        self.meta.compact()?;
        self.data.compact()?;

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
    pub data: ReadOnlyBuffer,
}

impl ReadOnlyState {
    pub fn copy(&self) -> State {
        State {
            config: self.config.clone(),
            meta: self.meta.copy_to_buffer(),
            data: self.data.copy_to_buffer(),
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
            },
            Payload {
                buffer: state.data,
                schema: bind.state.artifact.clone(),
            },
        ];

        return Ok(rtn);
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
