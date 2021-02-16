use crate::artifact::Artifact;
use crate::buffers::{Buffer, BufferFactories, ReadOnlyBuffer};
use crate::configs::{Configs, Keeper};
use crate::core::*;
use crate::id::StateKey;
use crate::message::Payload;
use no_proto::buffer::{NP_Buffer, NP_Finished_Buffer};
use no_proto::error::NP_Error;
use no_proto::memory::{NP_Memory_Owned, NP_Memory_Ref};
use no_proto::NP_Factory;
use std::error::Error;
use std::rc::Rc;
use std::sync::Arc;
use crate::error::MechError;

#[derive(Clone)]
pub struct State {
    pub meta: Buffer,
    pub data: Buffer,
}

impl State {
    pub fn new<'configs>(configs: &'configs Configs, artifact: Artifact) -> Result<Self, Box<dyn Error+'configs>> {
        let meta = Buffer::new(
            configs
                .buffer_factory_keeper
                .get(&CORE_CONTENT_META)?
                .new_buffer(Option::None),
        );
        let data = Buffer::new(
            configs
                .buffer_factory_keeper
                .get(&artifact)?
                .new_buffer(Option::None),
        );
        Ok(State {
            meta: meta,
            data: data,
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
        }
    }

    pub fn read_only(&self) -> Result<ReadOnlyState, Box<dyn Error>> {
        Ok(ReadOnlyState {
            meta: self.meta.read_only(),
            data: self.data.read_only(),
        })
    }

    pub fn compact(&mut self) -> Result<(), Box<dyn Error>> {
        self.meta.compact()?;
        self.data.compact()?;

        Ok(())
    }
}

impl StateMeta for State {
    fn set_artifact(&mut self, artifact: &Artifact) -> Result<(), Box<dyn Error>> {
        Ok(self.meta.set(&path!["artifact"], artifact.to())?)
    }

    fn set_creation_timestamp(&mut self, value: i64) -> Result<(), Box<dyn Error>> {
        Ok(self.meta.set(&path!["creation_timestamp"], value)?)
    }

    fn set_creation_cycle(&mut self, value: i64) -> Result<(), Box<dyn Error>> {
        Ok(self.meta.set(&path!["creation_cycle"], value)?)
    }
}

impl ReadOnlyStateMeta for State {
    fn get_artifact(&self) -> Result<Artifact, Box<dyn Error>> {
        Ok(Artifact::from(self.meta.get(&path!["artifact"])?)?)
    }

    fn get_creation_timestamp(&self) -> Result<i64, Box<dyn Error>> {
        Ok(self.meta.get(&path!["creation_timestamp"])?)
    }

    fn get_creation_cycle(&self) -> Result<i64, Box<dyn Error>> {
        Ok(self.meta.get(&path!["creation_cycle"])?)
    }
}

#[derive(Clone)]
pub struct ReadOnlyState {
    pub meta: ReadOnlyBuffer,
    pub data: ReadOnlyBuffer,
}

impl ReadOnlyState {
    pub fn copy(&self) -> State {
        State {
            meta: self.meta.copy_to_buffer(),
            data: self.data.copy_to_buffer(),
        }
    }

    pub fn convert_to_payloads(
        configs: &Configs,
        state: ReadOnlyState,
    ) -> Result<Vec<Payload>, Box<dyn Error>> {
        let artifact = state.get_artifact()?;
        let rtn: Vec<Payload> = vec![
            Payload {
                buffer: state.meta,
                artifact: CORE_CONTENT_META.clone(),
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
    fn get_artifact(&self) -> Result<Artifact, Box<dyn Error>> {
        Ok(Artifact::from(self.meta.get(&path!["artifact"])?)?)
    }

    fn get_creation_timestamp(&self) -> Result<i64, Box<dyn Error>> {
        Ok(self.meta.get(&path!["creation_timestamp"])?)
    }

    fn get_creation_cycle(&self) -> Result<i64, Box<dyn Error>> {
        Ok(self.meta.get(&path!["creation_cycle"])?)
    }
}

pub trait ReadOnlyStateMeta {
    fn get_artifact(&self) -> Result<Artifact, Box<dyn Error>>;
    fn get_creation_timestamp(&self) -> Result<i64, Box<dyn Error>>;
    fn get_creation_cycle(&self) -> Result<i64, Box<dyn Error>>;
}

pub trait StateMeta: ReadOnlyStateMeta {
    fn set_artifact(&mut self, artifact: &Artifact) -> Result<(), Box<dyn Error>>;
    fn set_creation_timestamp(&mut self, value: i64) -> Result<(), Box<dyn Error>>;
    fn set_creation_cycle(&mut self, value: i64) -> Result<(), Box<dyn Error>>;
}
