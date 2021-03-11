use std::borrow::BorrowMut;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::collections::hash_map::RandomState;
use std::fmt::Debug;
use std::ops::DerefMut;
use std::rc::Rc;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, Weak};

use no_proto::buffer::NP_Buffer;
use no_proto::collection::list::NP_List;
use no_proto::collection::struc::NP_Struct;
use no_proto::error::NP_Error;
use no_proto::memory::NP_Memory_Owned;

use mechtron_common::api::{CreateApiCallCreateNucleus, NeutronApiCallCreateMechtron};
use mechtron_common::artifact::Artifact;
use mechtron_common::buffers::{Buffer, Path};
use mechtron_common::configs::{BindConfig, Configs, MechtronConfig};
use mechtron_common::id::{Id, MechtronKey};
use mechtron_common::message::{Cycle, MechtronLayer, Message, MessageBuilder, MessageKind, Payload};
use mechtron_common::state::{NeutronStateInterface, ReadOnlyState, State};
use mechtron_core::api::{CreateApiCallCreateNucleus, NeutronApiCallCreateMechtron};
use mechtron_core::artifact::Artifact;
use mechtron_core::buffers;
use mechtron_core::buffers::{Buffer, Path};
use mechtron_core::configs::{BindConfig, Configs, CreateMessageConfig, MechtronConfig, MessageConfig, SimConfig};
use mechtron_core::core::*;
use mechtron_core::id::{Id, MechtronKey, NucleusKey, Revision, StateKey};
use mechtron_core::mechtron::MechtronContext;
use mechtron_core::message::{Cycle, DeliveryMoment, MechtronLayer, Message, MessageBuilder, MessageKind, Payload};
use mechtron_core::message::MessageKind::Create;
use mechtron_core::state::{NeutronStateInterface, ReadOnlyState, ReadOnlyStateMeta, State, StateMeta};
use mechtron_core::util::PongPayloadBuilder;

use crate::error::Error;
use crate::node::Node;
use crate::nucleus::{MechtronShellContext, Nucleus};

pub enum Phases {
    All,
    Some(Vec<String>),
    None,
}

pub struct MessagePort {
    pub receive: fn(
        context: &TronInfo,
        state: &State,
        message: &Message,
    ) -> Result<Option<Vec<MessageBuilder>>, Error>,
}

#[derive(Clone)]
pub struct TronInfo {
    pub key: MechtronKey,
    pub config: Arc<MechtronConfig>,
    pub bind: Arc<BindConfig>,
}

impl TronInfo {
    pub fn new(
        key: MechtronKey,
        tron_config: Arc<MechtronConfig>,
        bind: Arc<BindConfig>,
    ) -> Self {
        TronInfo {
            key: key,
            config: tron_config,
            bind: bind
        }
    }
}


pub enum TronShellState<'readonly>
{
    Mutable(MutexGuard<'readonly,State>),
    ReadOnly(Arc<ReadOnlyState>)
}

pub struct CreatePayloadsBuilder {
    pub constructor_artifact: Artifact,
    pub meta: Buffer,
    pub constructor: Buffer,
}

impl CreatePayloadsBuilder {
    pub fn new (
        configs: &Configs,
        config: Arc<MechtronConfig>,
    ) -> Result<Self, Error> {

        let meta_factory = configs.schemas.get(&CORE_SCHEMA_META_CREATE)?;
        let mut meta = Buffer::new(meta_factory.new_buffer(Option::None));
        meta.set(&path![&"config"], config.source.to().clone())?;
        let (constructor_artifact, constructor) =
            CreatePayloadsBuilder::constructor(configs, config.clone())?;
        Ok(CreatePayloadsBuilder {
            meta: meta,
            constructor_artifact: constructor_artifact,
            constructor: constructor,
        })
    }

    pub fn set_lookup_name(&mut self, lookup_name: &str) -> Result<(), Error> {
        self.meta.set(&path![&"lookup_name"], lookup_name)?;
        Ok(())
    }

    pub fn set_config(&mut self, config: &MechtronConfig) -> Result<(), Error> {
        self.constructor
            .set(&path!["config"], config.source.to())?;
        Ok(())
    }

    fn constructor(
        configs: &Configs,
        config: Arc<MechtronConfig>,
    ) -> Result<(Artifact, Buffer), Error> {
            let bind = configs.binds.get( &config.bind.artifact )?;
            let constructor_artifact = bind.message.create.artifact.clone();
            let factory = configs.schemas.get(&constructor_artifact)?;
            let constructor = factory.new_buffer(Option::None);
            let constructor = Buffer::new(constructor);

            Ok((constructor_artifact, constructor))
    }

    pub fn payloads<'configs>(configs: &'configs Configs, builder: CreatePayloadsBuilder) -> Vec<Payload> {
        let meta_artifact = CORE_SCHEMA_META_CREATE.clone();
        vec![
            Payload {
                schema: meta_artifact,
                buffer: builder.meta.read_only(),
            },
            Payload {
                schema: builder.constructor_artifact,
                buffer: builder.constructor.read_only(),
            },
        ]
    }

    pub fn payloads_builders( builder: CreatePayloadsBuilder) -> Vec<Payload> {
        let meta_artifact = CORE_SCHEMA_META_CREATE.clone();
        vec![
            Payload{
                schema: meta_artifact,
                buffer: builder.meta.read_only(),
            },
            Payload{
                schema: builder.constructor_artifact,
                buffer: builder.constructor.read_only(),
            },
        ]
    }
}





