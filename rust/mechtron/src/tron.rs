use std::error::Error;
use std::sync::Arc;

use no_proto::buffer::NP_Buffer;
use no_proto::error::NP_Error;
use no_proto::memory::NP_Memory_Owned;

use mechtron_common::artifact::Artifact;
use mechtron_common::buffers;
use mechtron_common::configs::{Configs, CreateMessageConfig, MessagesConfig, TronConfig, SimConfig};
use mechtron_common::state::{State, ReadOnlyState};
use mechtron_common::id::{StateKey, Id, NucleusKey, Revision, TronKey};
use mechtron_common::message::{Message, MessageBuilder, Payload, MessageKind, PayloadBuilder};
use mechtron_common::core::*;

use crate::app::{Local, SYS};
use mechtron_common::buffers::{Buffer, Path};

pub trait Tron
{
    fn init() -> Result<Box<Self>, Box<dyn Error>> where Self: Sized;

    fn create(&self,
              context: &TronContext,
              state: &mut State,
              create: &Message) -> Result<(Option<Vec<MessageBuilder>>), Box<dyn Error>>;

    fn update(&self, phase: &str) -> Result<fn(context: &TronContext, state: &State) -> Result<Option<Vec<MessageBuilder>>, Box<dyn Error>>, Box<dyn Error>>;

    fn port(&self, port: &str) -> Result<fn(context: &TronContext, state: &State, message: &Message) -> Result<Option<Vec<MessageBuilder>>, Box<dyn Error>>, Box<dyn Error>>;

    fn update_phases(&self) -> Phases;
}

pub enum Phases
{
    All,
    Some(Vec<String>),
    None,
}

pub struct MessagePort
{
    pub receive: fn(context: &TronContext, state: &State, message: &Message) -> Result<Option<Vec<MessageBuilder>>, Box<dyn Error>>
}

pub struct TronContext
{
    pub sim_id: Id,
    pub key: TronKey,
    pub revision: Revision,
    pub tron_config: Arc<TronConfig>,
    pub timestamp: i64,
}

impl TronContext {
    pub fn configs(&self) -> &mut Configs
    {
        return &mut SYS.local.configs;
    }

    pub fn get_state(&self, key: &StateKey) -> Result<ReadOnlyState, Box<dyn Error>>
    {
        let nucleus = SYS.local.nuclei.get(&self.sim_id)?;
        if key.revision.cycle >= self.revision.cycle
        {
            return Err(format!("tron {:?} attempted to read the state of tron {:?} in a present or future cycle, which is not allowed", self.key, key.state_id).into());
        }
        let state = nucleus.state.read_only(key)?;
        Ok(state)
    }

    fn lookup_nucleus(&self, context: &TronContext, name: &str) -> Result<Id, Box<dyn Error>>
    {
        let neutron_key = TronKey { nucleus: context.key.nucleus_id.clone(), tron: Id::new(context.key.nucleus_id.seq_id, 0) };
        let state_key = StateKey { tron: neutron_key, revision: Revision { cycle: context.revision.cycle - 1 } };
        let neutron_state = context.get_state(&state_key)?;

        let simulation_nucleus_id = Id::new(neutron_state.data.get::<i64>(&path![&"simulation_nucleus_id", &"seq_id"])?.unwrap(),
                                            neutron_state.data.get::<i64>(&path![&"simulation_nucleus_id", &"id"])?.unwrap());

        let simtron_key = TronKey {
            nucleus: simulation_nucleus_id.clone(),
            tron: Id::new(simulation_nucleus_id.seq_id, 1),
        };

        let state_key = StateKey { tron: simtron_key, revision: Revision { cycle: context.revision.cycle - 1 } };
        let simtron_state = context.get_state(&state_key)?;

        let nucleus_id = Id::new(simtron_state.data.get::<i64>(&path![&"nucleus_names", name, &"seq_id"])?.unwrap(),
                                 simtron_state.data.get::<i64>(&path![&"nucleus_names", name, &"id"])?.unwrap());


        Ok(nucleus_id)
    }

    fn lookup_tron(&self, context: &TronContext, nucleus_id: &Id, name: &str) -> Result<TronKey, Box<dyn Error>>
    {
        let neutron_key = TronKey { nucleus: nucleus_id.clone(), tron: Id::new(nucleus_id.seq_id, 0) };
        let state_key = StateKey { tron: neutron_key, revision: Revision { cycle: context.revision.cycle - 1 } };
        let neutron_state = context.get_state(&state_key)?;
        let tron_id = Id::new(neutron_state.data.get::<i64>(&path![&"tron_names", name, &"seq_id"])?.unwrap(),
                              neutron_state.data.get::<i64>(&path![&"tron_names", name, &"id"])?.unwrap());

        let tron_key = TronKey {
            nucleus: nucleus_id.clone(),
            tron: tron_id,
        };

        Ok(tron_key)
    }
}

pub struct TronShell
{
    pub tron: Box<dyn Tron>
}

impl TronShell
{
    pub fn new(tron: Box<dyn Tron>) -> Self {
        TronShell {
            tron: tron
        }
    }

    fn from(&self, context: &TronContext) -> mechtron_common::message::From
    {
        mechtron_common::message::From {
            tron: context.key.clone(),
            cycle: context.revision.cycle.clone(),
            timestamp: context.timestamp.clone(),
        }
    }


    fn builders_to_messages(&self, context: &TronContext, builders: Option<Vec<MessageBuilder>>) -> Result<Option<Vec<Message>>, Box<dyn Error>>
    {
        if builders.is_none()
        {
            return Ok(Option::None);
        }

        let mut builders = builders.unwrap();

        let messages = builders.iter().map(|builder: &mut MessageBuilder| {
            builder.from = Option::Some(self.from(context));

            if builder.to_nucleus_lookup_name.is_some()
            {
                builder.to_nucleus_id = Option::Some(self.lookup_nucleus(context, builder.to_nucleus_lookup_name.unwrap().as_str())?);
            }

            if builder.to_tron_lookup_name.is_some()
            {
                builder.to_tron_id = Option::Some(self.lookup_tron(context, &builder.to_nucleus_id.unwrap(), builder.to_tron_lookup_name.unwrap().as_str())?.tron_id);
            }


            builder.build(&mut SYS.net.id_seq)
        }).collect();

        return Ok(Option::Some(messages));
    }

    pub fn create(&self, context: &TronContext,
                  state: &mut State,
                  create: &Message) -> Result<Option<Vec<Message>>, Box<dyn Error>> {
        let mut builders = self.tron.create(context, state, create)?;

        return self.handle_builders(context, builders)
    }


    pub fn receive(&mut self, context: &TronContext, state: &mut State, message: &Message) -> Result<Option<Vec<Message>>, Box<dyn Error>> {

            let func = self.tron.port(&"blah")?;
            let builders = func(context, state, message)?;

            return self.handle_builders(context,builders)
        }

        pub fn handle_builders(&self, context: &TronContext, builders: Option<Vec<MessageBuilder>>) -> Result<Option<Vec<Message>>, Box<dyn Error>>
        {
            match builders {
                None => Ok(Option::None),
                Some(builders) =>
                    {
                        let mut rtn = vec!();
                        for mut builder in builders {
                            builder.from = Option::Some(self.from(context));
                            rtn.push(builder.build(&mut SYS.net.id_seq)?);
                        }
                        Ok(Option::Some(rtn))
                    }
            }
        }
    }



    pub struct Neutron
    {}

    pub struct NeutronStateInterface
    {}

    impl NeutronStateInterface
    {
        fn add_tron(&self, state: &mut State, key: &TronKey, kind: u8) -> Result<(), Box<dyn Error>>
        {
            let index = state.data.get_length(&[&"trons"])?.unwrap();
            let path = Path::new( path!["trons", index.to_string()] );
            key.append(&path.push(path!["id"]), &mut state.meta);
            state.data.set(&path.plus("kind"), kind)?;

            Ok(())
        }

        fn set_tron_name(&self, state: &mut State, name: &str, key: &TronKey ) -> Result<(), Box<dyn Error>>
        {
            key.append(&Path::new(path!["tron_names"]), &mut state.meta );
            Ok(())
        }
    }

    impl Neutron {
        pub fn valid_neutron_id(id: Id) -> bool {
            return id.id == 0;
        }

        pub fn create_tron(&self, context: &TronContext, state: &mut State, create: &Message) -> Result<(TronKey, State), Box<dyn Error>>
        {
            let tron_key = TronKey::new(context.key.nucleus_id, SYS.net.id_seq.next());
            let interface = NeutronStateInterface {};
            interface.add_tron(state, &tron_key, 0)?;

            let create_meta = &create.payloads[0].buffer;
            if create_meta.get(&path![&"lookup_name"]).unwrap().is_some()
            {
                let name = create_meta.get::<String>(&path![&"lookup_name"]).unwrap().unwrap();
                interface.set_tron_name(state, name.as_str(), name);
            }

            let tron_config = create_meta.get::<String>(&path![&"artifact"]).unwrap().unwrap();
            let tron_config = Artifact::from(&tron_config)?;
            let tron_config = context.configs().tron_config_keeper.get(&tron_config)?;

            let tron_state_artifact = match tron_config.messages
            {
                None => context.configs().core_artifact("schema/empty"),
                Some(_) => match tron_config.messages.unwrap().create {
                    None => context.configs().core_artifact("schema/empty"),
                    Some(_) => tron_config.messages.unwrap().create.unwrap().artifact
                }
            }?;

            let mut tron_state = State::new(context.configs(), tron_state_artifact.clone())?;

            tron_state.meta.set(&path![&"artifact"], tron_config.source.to());
            tron_state.meta.set(&path![&"creation_timestamp"], context.timestamp);
            tron_state.meta.set(&path![&"creation_cycle"], context.revision.cycle);

            let tron = init_tron(&tron_config)?;
            let tron = TronShell::new(tron);
            let tron_context = TronContext {
                sim_id: context.sim_id.clone(),
                key: tron_key.clone(),
                revision: context.revision.clone(),
                tron_config: tron_config.clone(),
                timestamp: context.timestamp,
            };

            tron.create(&tron_context, &mut tron_state, &create )?;

            Ok((tron_key, tron_state))
        }
    }

    impl Tron for Neutron {
        fn init() -> Result<Box<Self>, Box<dyn Error>> where Self: Sized {
            Ok(Box::new(Neutron {}))
        }

        fn create(&self, context: &TronContext, state: &mut State, create: &Message) -> Result<Option<Vec<MessageBuilder>>, Box<dyn Error>> {

            let interface = NeutronStateInterface {};

            //neutron adds itself to the tron manifest
            interface.add_tron(state, &context.key, 0)?;
            interface.set_tron_name(state, "neutron", &context.key)?;

            if create.payloads[1].buffer.is_set(&path![&"nucleus_lookup_name"])?
            {
                // then we need to pass a message to the simtron to add a lookup name for this nucleus
                let mut builder = MessageBuilder::new();
                builder.to_tron_lookup_name = Option::Some("simtron".to_string());
                builder.to_nucleus_lookup_name= Option::Some("simulation".to_string());
                builder.to_phase = Option::Some(0);
                builder.kind = Option::Some(MessageKind::Update);

                let factory = context.configs().buffer_factory_keeper.get(&CORE_SCHEMA_NUCLEUS_LOOKUP_NAME_MESSAGE)?;
                let buffer = factory.new_buffer(Option::None);
                let mut buffer = Buffer::new(buffer);
                buffer.set( &path!["name"], create.payloads[1].get(&path!["nucleus_lookup_name"])? );
                context.key.nucleus.append( &Path::just("id"), &mut buffer )?;
                let payload = PayloadBuilder{
                    buffer: buffer,
                    artifact: CORE_SCHEMA_NUCLEUS_LOOKUP_NAME_MESSAGE.clone()
                };
                let payloads = vec!(payload);
                builder.payloads = Option::Some( payloads );

                Ok(Option::Some(vec!(builder)))
            }
            else{
                Ok(Option::None)
            }

        }

        fn update(&self, phase: &str) -> Result<fn(&TronContext, &State) -> Result<Option<Vec<MessageBuilder>>, Box<dyn Error>>, Box<dyn Error>> {
            Err("does not have an update for phase".into())
        }

        fn port(&self, port: &str) -> Result<fn(&TronContext, &mut State, &Message) -> Result<Option<Vec<MessageBuilder>>, Box<dyn Error>>, Box<dyn Error>> {
            Err("no ports availabe from this tron".into())
        }

        fn update_phases(&self) -> Phases {
            Phases::None
        }
    }

    pub struct CreatePayloadsBuilder
    {
        pub constructor_artifact: Artifact,
        pub meta: Buffer,
        pub constructor: Buffer
    }

    impl CreatePayloadsBuilder
    {
        pub fn new(configs: &Configs, tron_config: &TronConfig) -> Result<Self, Box<dyn Error>>
        {
            let meta_factory = configs.core_buffer_factory("schema/create/meta")?;
            let mut meta = Buffer::new(meta_factory.new_buffer(Option::None));
            meta.set(&path![&"artifact"], tron_config.source.to())?;
            let (constructor_artifact, constructor) = CreatePayloadsBuilder::constructor(configs, tron_config)?;
            Ok(CreatePayloadsBuilder {
                meta: meta,
                constructor_artifact: constructor_artifact,
                constructor: constructor,
            })
        }
        pub fn set_sim_id(&mut self, sim_id: &Id)->Result<(),Box<dyn Error>>
        {
            sim_id.append( &Path::just("sim_id"), &mut self.constructor )?;
            Ok(())
        }

        pub fn set_lookup_name(&mut self, lookup_name: &str)->Result<(),Box<dyn Error>>
        {
            self.meta.set( &path![&"lookup_name"], lookup_name)?;
            Ok(())
        }

        pub fn set_sim_config( &mut self, sim_config: &SimConfig )->Result<(),Box<dyn Error>>
        {
            self.constructor.set( &path!["sim_config_artifact"], sim_config.source.to() )?;
            Ok(())
        }

        fn constructor(configs: &Configs, tron_config: &TronConfig) -> Result<(Artifact, Buffer ), Box<dyn Error>>
        {
            if tron_config.messages.is_some() && tron_config.messages.unwrap().create.is_some() {
                let constructor_artifact = tron_config.messages.unwrap().create.unwrap().artifact.clone();
                let factory = configs.buffer_factory_keeper.get(&constructor_artifact)?;
                let constructor = factory.new_buffer(Option::None);
                let constructor = Buffer::new(constructor);

                Ok((constructor_artifact, constructor))
            } else {
                let constructor_artifact = configs.core_artifact("schema/empty")?.clone();
                let factory = configs.core_buffer_factory("schema/empty")?;
                let constructor = factory.new_buffer(Option::None);
                Ok((constructor_artifact, constructor))
            }
        }

        pub fn payloads(configs: &Configs, builder: CreatePayloadsBuilder) -> Vec<Payload>
        {
            let meta_artifact = configs.core_artifact("schema/create/meta")?;
            vec![
                Payload {
                    artifact: meta_artifact,
                    buffer: Buffer::read_only(builder.meta),
                },
                Payload {
                    artifact: builder.constructor_artifact,
                    buffer: Buffer::read_only(builder.constructor),
                }
            ]
        }
    }

    struct SimTron{

    }

    impl Tron for SimTron{
        fn init() -> Result<Box<Self>, Box<dyn Error>> where Self: Sized {
            unimplemented!()
        }

        fn create(&self, context: &TronContext, state: &mut State, create: &Message) -> Result<Option<Vec<MessageBuilder>>, Box<dyn Error>> {
            unimplemented!()
        }

        fn update(&self, phase: &str) -> Result<fn(&TronContext, &State) -> Result<Option<Vec<MessageBuilder>>, Box<dyn Error>>, Box<dyn Error>> {
            unimplemented!()
        }

        fn port(&self, port: &str) -> Result<fn(&TronContext, &State, &Message) -> Result<Option<Vec<MessageBuilder>>, Box<dyn Error>>, Box<dyn Error>> {
            unimplemented!()
        }

        fn update_phases(&self) -> Phases {
            unimplemented!()
        }
    }

    pub fn init_tron(config: &TronConfig) -> Result<Box<dyn Tron>, Box<dyn Error>>
    {
        let rtn =Box::new(match config.kind.as_str() {
            "neutron" => Neutron::init(),
            "sim" => SimTron::init(),
            _ => return Err(format!("we don't have a tron of kind {}", config.kind).into())
        });

        Ok(rtn)
    }
