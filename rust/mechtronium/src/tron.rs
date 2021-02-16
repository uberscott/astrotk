use std::error::Error;
use std::sync::Arc;

use no_proto::buffer::NP_Buffer;
use no_proto::error::NP_Error;
use no_proto::memory::NP_Memory_Owned;

use mechtron_core::artifact::Artifact;
use mechtron_core::buffers;
use mechtron_core::configs::{
    Configs, CreateMessageConfig, MessagesConfig, SimConfig, TronConfig,
};
use mechtron_core::core::*;
use mechtron_core::id::{Id, NucleusKey, Revision, StateKey, TronKey};
use mechtron_core::message::{Message, MessageBuilder, MessageKind, Payload, PayloadBuilder};
use mechtron_core::state::{ReadOnlyState, State};

use crate::mechtronium::Mechtronium;
use crate::nucleus::Nucleus;
use mechtron_core::buffers::{Buffer, Path};
use std::rc::Rc;

pub trait Tron {
    fn create(
        &self,
        context: TronContext,
        state: &mut State,
        create: &Message,
    ) -> Result<(Option<Vec<MessageBuilder>>), Box<dyn Error>>;

    fn update(
        &self,
        phase: &str,
    ) -> Result<
        fn(
            context: TronContext,
            state: &State,
        ) -> Result<Option<Vec<MessageBuilder>>, Box<dyn Error>>,
        Box<dyn Error>,
    >;

    fn port(
        &self,
        port: &str,
    ) -> Result<
        fn(
            context: TronContext,
            state: &State,
            message: &Message,
        ) -> Result<Option<Vec<MessageBuilder>>, Box<dyn Error>>,
        Box<dyn Error>,
    >;

    fn update_phases(&self) -> Phases;
}

pub enum Phases {
    All,
    Some(Vec<String>),
    None,
}

pub struct MessagePort {
    pub receive: fn(
        context: &TronContext,
        state: &State,
        message: &Message,
    ) -> Result<Option<Vec<MessageBuilder>>, Box<dyn Error>>,
}

#[derive(Clone)]
pub struct TronContext {
    //    pub sim_id: Id,
    pub key: TronKey,
    pub revision: Revision,
    pub tron_config: Arc<TronConfig>,
    pub timestamp: u64,
}

impl TronContext {
    pub fn new(
        key: TronKey,
        revision: Revision,
        tron_config: Arc<TronConfig>,
        timestamp: u64,
    ) -> Self {
        TronContext {
            //           sim_id: sim_id,
            key: key,
            revision: revision,
            tron_config: tron_config,
            timestamp: timestamp,
        }
    }

    pub fn configs(&self) -> &Configs {
        unimplemented!()
    }

    pub fn get_state(&self, key: &StateKey) -> Result<Arc<ReadOnlyState>, Box<dyn Error>> {
        /*
        let sys = self.sys()?;
        if key.revision.cycle >= self.revision.cycle {
            return Err(format!("tron {:?} attempted to read the state of tron {:?} in a present or future cycle, which is not allowed", self.key, key).into());
        }
        let state = &self.nucleus.state;
        let state = state.read_only(key)?;
        Ok(state)
         */
        unimplemented!()
    }

    fn lookup_nucleus(&self, context: &TronContext, name: &str) -> Result<Id, Box<dyn Error>> {
        let neutron_key = TronKey {
            nucleus: context.key.nucleus.clone(),
            tron: Id::new(context.key.nucleus.seq_id, 0),
        };
        let state_key = StateKey {
            tron: neutron_key,
            revision: Revision {
                cycle: context.revision.cycle - 1,
            },
        };
        let neutron_state = context.get_state(&state_key)?;

        let simulation_nucleus_id = Id::new(
            neutron_state
                .data
                .get::<i64>(&path![&"simulation_nucleus_id", &"seq_id"])?,
            neutron_state
                .data
                .get::<i64>(&path![&"simulation_nucleus_id", &"id"])?,
        );

        let simtron_key = TronKey {
            nucleus: simulation_nucleus_id.clone(),
            tron: Id::new(simulation_nucleus_id.seq_id, 1),
        };

        let state_key = StateKey {
            tron: simtron_key,
            revision: Revision {
                cycle: context.revision.cycle - 1,
            },
        };
        let simtron_state = context.get_state(&state_key)?;

        let nucleus_id = Id::new(
            simtron_state
                .data
                .get::<i64>(&path![&"nucleus_names", name, &"seq_id"])?,
            simtron_state
                .data
                .get::<i64>(&path![&"nucleus_names", name, &"id"])?,
        );

        Ok(nucleus_id)
    }

    fn lookup_tron(
        &self,
        context: &TronContext,
        nucleus_id: &Id,
        name: &str,
    ) -> Result<TronKey, Box<dyn Error>> {
        let neutron_key = TronKey {
            nucleus: nucleus_id.clone(),
            tron: Id::new(nucleus_id.seq_id, 0),
        };
        let state_key = StateKey {
            tron: neutron_key,
            revision: Revision {
                cycle: context.revision.cycle - 1,
            },
        };
        let neutron_state = context.get_state(&state_key)?;
        let tron_id = Id::new(
            neutron_state
                .data
                .get::<i64>(&path![&"tron_names", name, &"seq_id"])?,
            neutron_state
                .data
                .get::<i64>(&path![&"tron_names", name, &"id"])?,
        );

        let tron_key = TronKey {
            nucleus: nucleus_id.clone(),
            tron: tron_id,
        };

        Ok(tron_key)
    }

}

pub struct TronShell {
    pub tron: Box<dyn Tron>,
}

impl TronShell {
    pub fn new(tron: Box<dyn Tron>) -> Self {
        TronShell { tron: tron }
    }

    fn from(&self, context: TronContext) -> mechtron_core::message::From {
        mechtron_core::message::From {
            tron: context.key.clone(),
            cycle: context.revision.cycle.clone(),
            timestamp: context.timestamp.clone(),
        }
    }

    pub fn create(
        &self,
        context: TronContext,
        state: &mut State,
        create: &Message,
    ) -> Result<Option<Vec<Message>>, Box<dyn Error>> {
        let mut builders = self.tron.create(context.clone(), state, create)?;

        return self.handle_builders(context.clone(), builders);
    }

    pub fn receive(
        &mut self,
        context: TronContext,
        state: &mut State,
        message: &Message,
    ) -> Result<Option<Vec<Message>>, Box<dyn Error>> {
        let func = self.tron.port(&"blah")?;
        let builders = func(context.clone(), state, message)?;

        return self.handle_builders(context, builders);
    }

    pub fn handle_builders(
        &self,
        context: TronContext,
        builders: Option<Vec<MessageBuilder>>,
    ) -> Result<Option<Vec<Message>>, Box<dyn Error>> {
        /*            match builders {
                       None => Ok(Option::None),
                       Some(builders) =>
                           {
                               let mut rtn = vec!();
                               for mut builder in builders {
                                   builder.from = Option::Some(self.from(context.clone()));
                                   rtn.push(builder.build(&mut context.sys()?.net.id_seq)?);
                               }
                               Ok(Option::Some(rtn))
                           }

                   }
        */
        unimplemented!()
    }
}

pub struct Neutron {}

pub struct NeutronStateInterface {}

impl NeutronStateInterface {
    fn add_tron(&self, state: &mut State, key: &TronKey, kind: u8) -> Result<(), Box<dyn Error>> {
        let index = state.data.get_length(&path!("trons"))?;
        let path = Path::new(path!["trons", index.to_string()]);
        key.append(&path.push(path!["id"]), &mut state.meta);
        state.data.set(&path.plus("kind"), kind)?;

        Ok(())
    }

    fn set_tron_name(
        &self,
        state: &mut State,
        name: &str,
        key: &TronKey,
    ) -> Result<(), Box<dyn Error>> {
        key.append(&Path::new(path!["tron_names"]), &mut state.meta);
        Ok(())
    }
}

impl Neutron {
    fn init() -> Result<Box<Tron>, Box<dyn Error>> {
        Ok(Box::new(Neutron {}))
    }

    pub fn valid_neutron_id(id: Id) -> bool {
        return id.id == 0;
    }

    pub fn create_tron(
        &self,
        context: TronContext,
        state: Rc<State>,
        create: &Message,
    ) -> Result<(TronKey, State), Box<dyn Error>> {
        /*
                    let tron_key = TronKey::new(context.key.nucleus.clone(), context.sys()?.net.id_seq.next());
                    let interface = NeutronStateInterface {};
                    interface.add_tron(state, &tron_key, 0)?;

                    let create_meta = &create.payloads[0].buffer;
                    if create_meta.is_set::<String>(&path![&"lookup_name"])?
                    {
                        let name = create_meta.get::<String>(&path![&"lookup_name"])?;
                        interface.set_tron_name(state, name.as_str(), &tron_key);
                    }

                    let tron_config = create_meta.get::<String>(&path![&"artifact"])?;
                    let tron_config = Artifact::from(&tron_config)?;
                    let tron_config = context.configs().tron_config_keeper.get(&tron_config)?;

                    let tron_state_artifact = match tron_config.messages
                    {
                        None => CORE_SCHEMA_EMPTY.clone(),
                        Some(_) => match tron_config.messages.unwrap().create {
                            None => CORE_SCHEMA_EMPTY.clone(),
                            Some(_) => tron_config.messages.unwrap().create.unwrap().artifact
                        }
                    };

                    let mut tron_state = State::new(context.configs(), tron_state_artifact.clone())?;

                    tron_state.meta.set(&path![&"artifact"], tron_config.source.to());
                    tron_state.meta.set(&path![&"creation_timestamp"], context.timestamp);
                    tron_state.meta.set(&path![&"creation_cycle"], context.revision.cycle);

                    let tron = init_tron(&tron_config)?;
                    let tron = TronShell::new(tron);
                    let tron_context = TronContext {
        nucleus: context.nucleus,
                        key: tron_key.clone(),
                        revision: context.revision.clone(),
                        tron_config: tron_config.clone(),
                        timestamp: context.timestamp,
                        sys: context.sys.clone()
                    };

                    tron.create(tron_context.clone(), &mut tron_state, &create )?;

                    Ok((tron_key, tron_state))

                     */
        unimplemented!()
    }
}

impl Tron for Neutron {
    fn create(
        &self,
        context: TronContext,
        state: &mut State,
        create: &Message,
    ) -> Result<Option<Vec<MessageBuilder>>, Box<dyn Error>> {
        /*

        let interface = NeutronStateInterface {};

        //neutron adds itself to the tron manifest
        interface.add_tron(state, &context.key, 0)?;
        interface.set_tron_name(state, "neutron", &context.key)?;

        if create.payloads[1].buffer.is_set::<String>(&path![&"nucleus_lookup_name"])?
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
            let nucleus_lookup_name: String = create.payloads[1].buffer.get(&path!["nucleus_lookup_name"])?;
            buffer.set( &path!["name"],nucleus_lookup_name );
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
          */
        unimplemented!()
    }
    fn update(
        &self,
        phase: &str,
    ) -> Result<
        fn(
            context: TronContext,
            state: &State,
        ) -> Result<Option<Vec<MessageBuilder>>, Box<dyn Error>>,
        Box<dyn Error>,
    > {
        Err("does not have an update for phase".into())
    }

    fn port(
        &self,
        port: &str,
    ) -> Result<
        fn(
            context: TronContext,
            state: &State,
            message: &Message,
        ) -> Result<Option<Vec<MessageBuilder>>, Box<dyn Error>>,
        Box<dyn Error>,
    > {
        Err("no ports availabe from this tron".into())
    }

    fn update_phases(&self) -> Phases {
        Phases::None
    }
}

pub struct CreatePayloadsBuilder {
    pub constructor_artifact: Artifact,
    pub meta: Buffer,
    pub constructor: Buffer,
}

impl CreatePayloadsBuilder {
    pub fn new<'configs> (
        configs: &'configs Configs,
        tron_config: &TronConfig,
    ) -> Result<Self, Box<dyn Error>> {

        /*
        let meta_factory = configs.buffer_factory_keeper.get(&CORE_CREATE_META)?;
        let mut meta = Buffer::new(meta_factory.new_buffer(Option::None));
        meta.set(&path![&"artifact"], tron_config.source.to())?;
        let (constructor_artifact, constructor) =
            CreatePayloadsBuilder::constructor(configs, tron_config)?;
        Ok(CreatePayloadsBuilder {
            meta: meta,
            constructor_artifact: constructor_artifact,
            constructor: constructor,
        })
         */
        unimplemented!()
    }
    pub fn set_sim_id(&mut self, sim_id: &Id) -> Result<(), Box<dyn Error>> {
        sim_id.append(&Path::just("sim_id"), &mut self.constructor)?;
        Ok(())
    }

    pub fn set_lookup_name(&mut self, lookup_name: &str) -> Result<(), Box<dyn Error>> {
        self.meta.set(&path![&"lookup_name"], lookup_name)?;
        Ok(())
    }

    pub fn set_sim_config(&mut self, sim_config: &SimConfig) -> Result<(), Box<dyn Error>> {
        self.constructor
            .set(&path!["sim_config_artifact"], sim_config.source.to())?;
        Ok(())
    }

    fn constructor(
        configs: &'static Configs,
        tron_config: &TronConfig,
    ) -> Result<(Artifact, Buffer), Box<dyn Error>> {
        /*
        if (&tron_config.messages).is_some() && (&tron_config.messages).unwrap().create.is_some() {
            let constructor_artifact = tron_config.messages.unwrap().create.unwrap().artifact.clone();
            let factory = configs.buffer_factory_keeper.get(&constructor_artifact)?;
            let constructor = factory.new_buffer(Option::None);
            let constructor = Buffer::new(constructor);

            Ok((constructor_artifact, constructor))
        } else {
            let constructor_artifact = CORE_SCHEMA_EMPTY.clone();
            let factory = configs.buffer_factory_keeper.get(&CORE_SCHEMA_EMPTY)?;
            let constructor = factory.new_buffer(Option::None);
            let constructor = Buffer::new(constructor);
            Ok((constructor_artifact, constructor))
        }
         */
        unimplemented!()
    }

    pub fn payloads<'configs>(configs: &'configs Configs, builder: CreatePayloadsBuilder) -> Vec<Payload> {
        let meta_artifact = CORE_CREATE_META.clone();
        vec![
            Payload {
                artifact: meta_artifact,
                buffer: builder.meta.read_only(),
            },
            Payload {
                artifact: builder.constructor_artifact,
                buffer: builder.constructor.read_only(),
            },
        ]
    }
}

struct SimTron {}

impl SimTron {
    pub fn init() -> Result<Box<dyn Tron>, Box<dyn Error>> {
        unimplemented!()
    }
}

impl Tron for SimTron {
    fn create(
        &self,
        context: TronContext,
        state: &mut State,
        create: &Message,
    ) -> Result<Option<Vec<MessageBuilder>>, Box<dyn Error>> {
        unimplemented!()
    }

    fn update(
        &self,
        phase: &str,
    ) -> Result<
        fn(TronContext, &State) -> Result<Option<Vec<MessageBuilder>>, Box<dyn Error>>,
        Box<dyn Error>,
    > {
        unimplemented!()
    }

    fn port(
        &self,
        port: &str,
    ) -> Result<
        fn(TronContext, &State, &Message) -> Result<Option<Vec<MessageBuilder>>, Box<dyn Error>>,
        Box<dyn Error>,
    > {
        unimplemented!()
    }

    fn update_phases(&self) -> Phases {
        unimplemented!()
    }
}

pub fn init_tron(config: &TronConfig) -> Result<Box<dyn Tron>, Box<dyn Error>> {
    let rtn: Box<Tron> = match config.kind.as_str() {
        "neutron" => Neutron::init()? as Box<Tron>,
        "sim" => SimTron::init()? as Box<Tron>,
        _ => return Err(format!("we don't have a tron of kind {}", config.kind).into()),
    };

    Ok(rtn)
}
