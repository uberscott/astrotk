use std::borrow::BorrowMut;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::ops::DerefMut;
use std::rc::Rc;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, Weak};

use no_proto::buffer::NP_Buffer;
use no_proto::collection::list::NP_List;
use no_proto::collection::struc::NP_Struct;
use no_proto::error::NP_Error;
use no_proto::memory::NP_Memory_Owned;

use mechtron_core::api::NeutronApiCallCreateMechtron;
use mechtron_core::artifact::Artifact;
use mechtron_core::buffers;
use mechtron_core::buffers::{Buffer, Path};
use mechtron_core::configs::{BindConfig, Configs, CreateMessageConfig, MechtronConfig, MessageConfig, SimConfig};
use mechtron_core::core::*;
use mechtron_core::id::{Id, MechtronKey, NucleusKey, Revision, StateKey};
use mechtron_core::mechtron::MechtronContext;
use mechtron_core::message::{Cycle, DeliveryMoment, MechtronLayer, Message, MessageBuilder, MessageKind, Payload };
use mechtron_core::state::{ReadOnlyState, ReadOnlyStateMeta, State, StateMeta};
use mechtron_core::util::PongPayloadBuilder;

use crate::error::Error;
use crate::node::Node;
use crate::nucleus::{MechtronShellContext, Nucleus};

pub trait MechtronKernel {
    fn create(
        &self,
        info: TronInfo,
        context: &dyn MechtronShellContext,
        state: &mut State,
        create: &Message,
    ) -> Result<Option<Vec<MessageBuilder>>, Error>;

    fn update(
        &self,
        phase: &str,
    ) -> Result<
        fn(
            info: TronInfo,
            context: &dyn MechtronShellContext,
            state: &mut MutexGuard<State>,
        ) -> Result<Option<Vec<MessageBuilder>>, Error>,
        Error,
    >;

    fn port(
        &self,
        port: &str,
    ) -> Result<
        fn(
            info: TronInfo,
            context: &dyn MechtronShellContext,
            state: &mut MutexGuard<State>,
            messages: &Vec<Arc<Message>>,
        ) -> Result<Option<Vec<MessageBuilder>>, Error>,
        Error,
    >;

    fn extra(
        &self,
        port: &str,
    ) -> Result<
        fn(
            info: TronInfo,
            context: &dyn MechtronShellContext,
            state: &ReadOnlyState,
            message: &Message,
        ) -> Result<Option<Vec<MessageBuilder>>, Error>,
        Error
    >;


}

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
}

impl TronInfo {
    pub fn new(
        key: MechtronKey,
        tron_config: Arc<MechtronConfig>,
    ) -> Self {
        TronInfo {
            key: key,
            config: tron_config,
        }
    }
}


pub enum TronShellState<'readonly>
{
    Mutable(MutexGuard<'readonly,State>),
    ReadOnly(Arc<ReadOnlyState>)
}

pub struct Neutron {}

pub struct NeutronStateInterface {}

impl NeutronStateInterface {
    fn add_mechtron(&self, state: &mut State, key: &MechtronKey, kind: String) -> Result<(), Error> {
        println!("ADD MECHTRON...{}",kind);
        let index = {
                match state.data.get_length(&path!("mechtrons"))
                {
                    Ok(length) => {length}
                    Err(_) => {0}
                }
        };

        let path = Path::new(path!["mechtrons", index.to_string()]);
        key.mechtron.append(&path.push(path!["id"]), &mut state.data)?;
        state.data.set(&path.with(path!["kind"]), kind)?;
        println!("MECHTRON ADDED...x");

        Ok(())
    }

    fn set_mechtron_name(
        &self,
        state: &mut State,
        name: &str,
        key: &MechtronKey,
    ) -> Result<(), Error> {
        key.append(&Path::new(path!["mechtron_names"]), &mut state.meta);
        Ok(())
    }


    fn set_mechtron_index
    (
        &self,
        state: &mut State,
        value: i64,
    ) -> Result<(), Error> {
        state.data.set( &path!["mechtron_index"], value );
        Ok(())
    }

    fn set_mechtron_seq_id(
        &self,
        state: &mut State,
        value: i64,
    ) -> Result<(), Error> {
        state.data.set( &path!["mechtron_seq_id"], value );
        Ok(())
    }
}

impl Neutron {
    pub fn init() -> Result<Box<MechtronKernel>, Error> {
        Ok(Box::new(Neutron {}))
    }

    pub fn valid_neutron_id(id: Id) -> bool {
        return id.id == 0;
    }


    fn create_mechtrons(
        info: TronInfo,
        context: &dyn MechtronShellContext,
        state: &mut MutexGuard<State>,
        messages: &Vec<Arc<Message>> ) -> Result<Option<Vec<MessageBuilder>>, Error>
    {
        let mut builders = vec!();
        for message in messages
        {
           let builder = Neutron::create_mechtron(info.clone(),context,state,message.clone())?;
           builders.push(builder);
        }

        Ok(Option::Some(builders))
    }

    pub fn create_mechtron(
        info: TronInfo,
        context: &dyn MechtronShellContext,
        neutron_state: &mut MutexGuard<State>,
        create_message: Arc<Message>,
    ) -> Result<MessageBuilder, Error> {
        // a simple helper interface for working with neutron state
        let neutron_state_interface = NeutronStateInterface {};

        // grab the new mechtron create meta
        let new_mechtron_create_meta = &create_message.payloads[0].buffer;

        // and derive the new mechtron config
        let new_mechtron_config = new_mechtron_create_meta.get::<String>(&path![&"config"])?;
println!("CREATE MECHTRON {}",new_mechtron_config);
        let new_mechtron_config = Artifact::from(&new_mechtron_config)?;
        let new_mechtron_config = context.configs().mechtrons.get(&new_mechtron_config)?;

        // increment the neutron's mechtron_index
        let mut mechtron_index = neutron_state.data.get::<i64>(&path!["mechtron_index"] )?;
        mechtron_index = mechtron_index +1;
        neutron_state_interface.set_mechtron_index(neutron_state, mechtron_index);

        // create the new mechtron id and key
        let new_mechtron_id= Id::new(info.key.nucleus.id,mechtron_index);
        let new_mechtron_key = MechtronKey::new(info.key.nucleus.clone(), new_mechtron_id );

        // add the new mechtron to the neutron/nucleus manifest
        neutron_state_interface.add_mechtron(& mut *neutron_state, &new_mechtron_key, new_mechtron_config.source.to() )?;

        // if the new mechtron has a lookup name, add it
        if new_mechtron_create_meta.is_set::<String>(&path![&"lookup_name"])?
        {
            let name = new_mechtron_create_meta.get::<String>(&path![&"lookup_name"])?;
            neutron_state_interface.set_mechtron_name(& mut *neutron_state, name.as_str(), &new_mechtron_key);
        }

        // prepare an api call to the MechtronShell to create this new mechtron
        let mut call = NeutronApiCallCreateMechtron::new(context.configs(), new_mechtron_config.clone(), create_message.clone() )?;

        // set some additional meta information about the new mechtron
        {
            new_mechtron_id.append(&Path::new(path!("id")), &mut call.state.meta)?;
            call.state.meta.set(&path![&"mechtron_config"], new_mechtron_config.source.to())?;
            call.state.meta.set(&path![&"creation_cycle"], context.revision().cycle)?;
            call.state.meta.set(&path![&"creation_timestamp"], context.timestamp() as i64)?;
            call.state.meta.set(&path![&"taint"], false)?;
        }

        let mut builder = MessageBuilder::new();
        builder.kind = Option::Some(MessageKind::Api);
        builder.to_layer = Option::Some(MechtronLayer::Shell);
        builder.to_nucleus_id=Option::Some(info.key.nucleus.clone());
        builder.to_tron_id=Option::Some(info.key.mechtron.clone());
        builder.to_cycle_kind=Option::Some(Cycle::Present);
        println!("Neutron::create_mechtron() replacing payloads");
        builder.payloads.replace(Option::Some(NeutronApiCallCreateMechtron::payloads(call,context.configs())?));

        Ok(builder)
    }
}




impl MechtronKernel for Neutron {
    fn create(
        &self,
        info: TronInfo,
        context: &dyn MechtronShellContext,
        state: &mut State,
        create: &Message,
    ) -> Result<Option<Vec<MessageBuilder>>, Error> {

        let interface = NeutronStateInterface {};

        //neutron adds itself to the tron manifest
        interface.add_mechtron(state, &info.key, info.config.source.to() )?;
        interface.set_mechtron_name(state, "neutron", &info.key)?;
        interface.set_mechtron_index(state, 0 );


        if create.payloads[1].buffer.is_set::<String>(&path![&"nucleus_lookup_name"])?
        {
            // then we need to pass a message to the simtron to add a lookup name for this nucleus
            let mut builder = MessageBuilder::new();
            builder.to_tron_lookup_name = Option::Some("simtron".to_string());
            builder.to_nucleus_lookup_name= Option::Some("simulation".to_string());
            builder.to_phase = Option::Some("default".to_string());
            builder.kind = Option::Some(MessageKind::Update);

            let factory = context.configs().schemas.get(&CORE_SCHEMA_NUCLEUS_LOOKUP_NAME_MESSAGE)?;
            let buffer = factory.new_buffer(Option::None);
            let mut buffer = Buffer::new(buffer);
            let nucleus_lookup_name: String = create.payloads[1].buffer.get(&path!["nucleus_lookup_name"])?;
            buffer.set( &path!["name"],nucleus_lookup_name );
            info.key.nucleus.append(&Path::just("id"), &mut buffer )?;
            let payload = Payload{
                buffer: buffer.read_only(),
                schema: CORE_SCHEMA_NUCLEUS_LOOKUP_NAME_MESSAGE.clone()
            };
            let payloads = vec!(payload);
            builder.payloads.replace(Option::Some( payloads ));

            Ok(Option::Some(vec!(builder)))
        }
        else{
            Ok(Option::None)
        }
    }

    fn update(&self, phase: &str) -> Result<fn(TronInfo, &dyn MechtronShellContext, &mut MutexGuard<State>) -> Result<Option<Vec<MessageBuilder>>, Error>, Error> {
        unimplemented!()
    }

    fn port(&self, port: &str) -> Result<fn(TronInfo, &dyn MechtronShellContext, &mut MutexGuard<State>, &Vec<Arc<Message>>) -> Result<Option<Vec<MessageBuilder>>, Error>, Error> {

        match port{
            "create" => Ok(Neutron::create_mechtrons),
            _ => Err(format!("port not available: {}", port).into())
        }
    }



    fn extra(&self, port: &str) -> Result<fn(TronInfo, &dyn MechtronShellContext, &ReadOnlyState, &Message) -> Result<Option<Vec<MessageBuilder>>, Error>, Error> {
println!("seeking extra: {}",port)        ;
        unimplemented!()
    }


}

pub struct Simtron{}
impl Simtron{
    pub fn init() -> Result<Box<MechtronKernel>, Error> {
        Ok(Box::new(Simtron{}))
    }
}

impl MechtronKernel for Simtron{
    fn create(&self, info: TronInfo, context: &dyn MechtronShellContext, state: &mut State, create: &Message) -> Result<Option<Vec<MessageBuilder>>, Error> {
        println!("Simtron created!");
        Ok(Option::None)
    }

    fn update(&self, phase: &str) -> Result<fn(TronInfo, &dyn MechtronShellContext, &mut MutexGuard<State>) -> Result<Option<Vec<MessageBuilder>>, Error>, Error> {
        unimplemented!()
    }

    fn port(&self, port: &str) -> Result<fn(TronInfo, &dyn MechtronShellContext, &mut MutexGuard<State>, &Vec<Arc<Message>>) -> Result<Option<Vec<MessageBuilder>>, Error>, Error> {
        unimplemented!()
    }

    fn extra(&self, port: &str) -> Result<fn(TronInfo, &dyn MechtronShellContext, &ReadOnlyState, &Message) -> Result<Option<Vec<MessageBuilder>>, Error>, Error> {
        unimplemented!()
    }
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





