use crate::artifact::Artifact;
use crate::buffers::{Buffer, BufferFactories, ReadOnlyBuffer};
use crate::configs::{Configs, Keeper, MechtronConfig, BindConfig};
use crate::core::*;
use crate::id::StateKey;
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
    pub artifact: Artifact
}

impl State {
    pub fn new_empty<'configs>(configs: &'configs Configs) -> Result<Self, Error> {
        let mut meta = Buffer::new(
            configs
                .schemas
                .get(&CORE_SCHEMA_META_STATE)?
                .new_buffer(Option::None),
        );
        let data = Buffer::new(
            configs
                .schemas
                .get(&CORE_SCHEMA_EMPTY)?
                .new_buffer(Option::None),
        );
        meta.set(&path!["artifact"], CORE_SCHEMA_EMPTY.to());
        Ok(State {
            meta: meta,
            data: data,
            artifact: CORE_SCHEMA_EMPTY.clone()
        })
    }
    pub fn new<'configs>(configs: &'configs Configs, bind: Arc<BindConfig>) -> Result<Self, Error> {

        let artifact = bind.state.artifact.clone();
        let mut meta = Buffer::new(
            configs
                .schemas
                .get(&CORE_SCHEMA_META_STATE)?
                .new_buffer(Option::None),
        );
        let data = Buffer::new(
            configs
                .schemas
                .get(&artifact)?
                .new_buffer(Option::None),
        );
        meta.set(&path!["artifact"], artifact.to());
        Ok(State {
            meta: meta,
            data: data,
            artifact: artifact
        })
    }

    pub fn from(
        artifact: Artifact,
        meta: NP_Buffer<NP_Memory_Owned>,
        data: NP_Buffer<NP_Memory_Owned>,
    ) -> Self {
        State {
            meta: Buffer::new(meta),
            data: Buffer::new(data),
            artifact: artifact
        }
    }


    pub fn read_only(&self) -> Result<ReadOnlyState, Error> {
        Ok(ReadOnlyState {
            artifact: self.artifact.clone(),
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
    fn set_artifact(&mut self, artifact: &Artifact) -> Result<(), Error> {
        Ok(self.meta.set(&path!["artifact"], artifact.to())?)
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
    fn get_artifact(&self) -> Result<Artifact, Error> {
        Ok(Artifact::from(self.meta.get(&path!["artifact"])?)?)
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
    pub artifact: Artifact,
    pub meta: ReadOnlyBuffer,
    pub data: ReadOnlyBuffer,
}

impl ReadOnlyState {
    pub fn copy(&self) -> State {
        State {
            artifact: self.artifact.clone(),
            meta: self.meta.copy_to_buffer(),
            data: self.data.copy_to_buffer(),
        }
    }

    pub fn convert_to_payloads(
        configs: &Configs,
        state: ReadOnlyState,
    ) -> Result<Vec<Payload>, Error> {
        let artifact = state.get_artifact()?;
        let rtn: Vec<Payload> = vec![
            Payload {
                buffer: state.meta,
                artifact: CORE_SCHEMA_META_STATE.clone(),
            },
            Payload {
                buffer: state.data,
                artifact: artifact,
            },
        ];

        return Ok(rtn);
    }
}

impl ReadOnlyStateMeta for ReadOnlyState {
    fn get_artifact(&self) -> Result<Artifact, Error> {
        Ok(Artifact::from(self.meta.get(&path!["artifact"])?)?)
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
    fn get_artifact(&self) -> Result<Artifact, Error>;
    fn get_creation_timestamp(&self) -> Result<i64, Error>;
    fn get_creation_cycle(&self) -> Result<i64, Error>;
    fn is_tainted(&self) -> Result<bool, Error>;
}

pub trait StateMeta: ReadOnlyStateMeta {
    fn set_artifact(&mut self, artifact: &Artifact) -> Result<(), Error>;
    fn set_creation_timestamp(&mut self, value: i64) -> Result<(), Error>;
    fn set_creation_cycle(&mut self, value: i64) -> Result<(), Error>;
    fn set_taint( &mut self, taint: bool );
}
