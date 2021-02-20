use std::sync::{Arc, Mutex, MutexGuard};

use no_proto::buffer::NP_Buffer;
use no_proto::error::NP_Error;
use no_proto::memory::NP_Memory_Owned;

use mechtron_core::artifact::Artifact;
use mechtron_core::buffers;
use mechtron_core::configs::{
    Configs, CreateMessageConfig, MessageConfig, SimConfig, TronConfig,
};
use mechtron_core::core::*;
use mechtron_core::id::{Id, NucleusKey, Revision, StateKey, TronKey};
use mechtron_core::message::{Message, MessageBuilder, MessageKind, Payload, PayloadBuilder, DeliveryTarget};
use mechtron_core::state::{ReadOnlyState, State};

use crate::node::Node;
use crate::nucleus::{Nucleus, TronContext, NeutronContext};
use mechtron_core::buffers::{Buffer, Path};
use crate::error::Error;
use std::ops::DerefMut;
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use mechtron_core::util::PongPayloadBuilder;

pub trait Tron {
    fn create(
        &self,
        info: TronInfo,
        context: &dyn TronContext,
        state: Arc<Mutex<State>>,
        create: &Message,
    ) -> Result<Option<Vec<MessageBuilder>>, Error>;

    fn update(
        &self,
        phase: &str,
    ) -> Result<
        fn(
            info: TronInfo,
            context: &dyn TronContext,
            state: Arc<Mutex<State>>,
        ) -> Result<Option<Vec<MessageBuilder>>, Error>,
        Error,
    >;

    fn port(
        &self,
        port: &str,
    ) -> Result<
        fn(
            info: TronInfo,
            context: &dyn TronContext,
            state: Arc<Mutex<State>>,
            messages: &Vec<&Message>,
        ) -> Result<Option<Vec<MessageBuilder>>, Error>,
        Error,
    >;

    fn extra(
        &self,
        port: &str,
    ) -> Result<
        fn(
            info: TronInfo,
            context: &dyn TronContext,
            state: &ReadOnlyState,
            message: &Message,
        ) -> Result<Option<Vec<MessageBuilder>>, Error>,
        Error,
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
        context: &TronInfo,
        state: &State,
        message: &Message,
    ) -> Result<Option<Vec<MessageBuilder>>, Error>,
}

#[derive(Clone)]
pub struct TronInfo {
    pub key: TronKey,
    pub config: Arc<TronConfig>,
}

impl TronInfo {
    pub fn new(
        key: TronKey,
        tron_config: Arc<TronConfig>,
    ) -> Self {
        TronInfo {
            key: key,
            config: tron_config,
        }
    }


}



pub struct TronShell {
    pub tron: Box<dyn Tron>,
}

impl TronShell {
    pub fn new(tron: Box<dyn Tron>) -> Self {
        TronShell { tron: tron }
    }

    fn warn<E:Debug>( &self, error: E )
    {
       println!("WARN: TronShell got unexpected error {:?}",error);
    }

    fn panic<E:Debug>( &self, error: E )
    {
        println!("PANIC: TronShell got unexpected error {:?}",error);
    }

    fn reject( &self,message: &Message, info: TronInfo, context: &TronContext )->Message
    {
        // we unrwrap it because if REJECT message isn't available, then nothing should work
        message.reject(mechtron_core::message::From {
            tron: info.key.clone(),
            cycle: context.revision().cycle.clone(),
            timestamp: context.timestamp()
        }, format!("port '{}' does not exist on this mechtron", message.to.port).as_str(), context.seq(), context.configs()).unwrap()
    }

    fn ok( &self, message: &Message, info: TronInfo, ok: bool, context: &TronContext )->Message
    {
        // we unrwrap it because if REJECT message isn't available, then nothing should work
        message.ok(mechtron_core::message::From {
            tron: info.key.clone(),
            cycle: context.revision().cycle,
            timestamp: context.timestamp()
        }, ok, context.seq(), context.configs()).unwrap()
    }

    fn respond( &self, message: &Message, info: TronInfo, payloads: Vec<Payload>,context: &TronContext )->Message
    {
        message.respond(mechtron_core::message::From {
            tron: info.key.clone(),
            cycle: context.revision().cycle,
            timestamp: context.timestamp()
        }, payloads, context.seq())
    }

    fn from(&self, info: TronInfo, context: &dyn TronContext) -> mechtron_core::message::From {
        mechtron_core::message::From {
            tron: info.key.clone(),
            cycle: context.revision().cycle.clone(),
            timestamp: context.timestamp()
        }
    }

    pub fn create(
        &self,
        info: TronInfo,
        context: &dyn TronContext,
        state:  Arc<Mutex<State>>,
        create: &Message,
    ) -> Option<Vec<Message>> {

        let mut builders = self.tron.create(info.clone(), context, state, create);
        let rtn = self.handle_builders(info.clone(), builders);
        if rtn.is_empty()  {
            Option::None
        }
        else {
            Option::Some(rtn)
        }
    }


    pub fn extra(
        &mut self,
        info: TronInfo,
        context: &dyn TronContext,
        state: &ReadOnlyState,
        message: &Message,
    ) -> Option<Vec<Message>>
    {
        match message.to.target
        {
            DeliveryTarget::Shell => {
                match message.to.port.as_str(){
                    "ping" => {
                        return Option::Some(
                            vec!(self.respond(message, info,
                                              vec!(PongPayloadBuilder::new(context.configs()).unwrap())
                                              ,context))
                        )
                    }
                    _ => {
                        return Option::None
                    }
                };
            }
            DeliveryTarget::Kernel => {
                match info.config.message.extra.contains_key(&message.to.port)
                {
                    true => {
                        let func = self.tron.extra(&message.to.port);
                        match func{
                            Ok(func) => {
                                let builders = func( info.clone(),context,state,message );
                                let rtn = self.handle_builders(info.clone(), builders );
                                if rtn.is_empty()
                                {
                                    return Option::None;
                                }
                                else {
                                    return Option::Some(rtn);
                                }
                            }
                            Err(e) => {
                                self.panic(e);
                                return Option::None
                            }
                        }
                    }
                    false => {
                        let rtn = message.reject(mechtron_core::message::From {
                            tron: info.key.clone(),
                            cycle: context.revision().cycle,
                            timestamp: context.timestamp()
                        }, format!("port '{}' does not exist on this mechtron", message.to.port).as_str(), context.seq(), context.configs());
                        let rtn = match rtn{
                            Ok(message) => {
                                let rtn = vec!(message);
                                return Option::Some(rtn)
                            }
                            Err(e) => {
                                self.warn(e)
                            }
                        };

                    }
                }



            }
        }
        Option::None
    }

    pub fn incomming(
        &mut self,
        info: TronInfo,
        context: &dyn TronContext,
        state: Arc<Mutex<State>>,
        messages: Vec<&Message>,
    ) -> Option<Vec<Message>> {
        let mut hash = HashMap::new();
        for message in messages
        {
            if !hash.contains_key(&message.to.port )
            {
                hash.insert(message.to.port.clone(), vec!() );
            }
            let messages = hash.get_mut(&message.to.port ).unwrap();
            messages.push(message);
        }
        let mut ports = vec!();
        for port in hash.keys()
        {
            ports.push(port);
        }

        let mut rtn = vec!();

        ports.sort();
        for port in ports
        {
            let messages = hash.get(port).unwrap();
            let func = self.tron.port(port);
            match func
            {
                Ok(func) => {
                    let builders = func( info.clone(),context,state.clone(),messages );
                    rtn.append( &mut self.handle_builders(info.clone(),builders ))
                }
                Err(e) => {
                    self.panic(e);
                    return Option::None
                }
            }

        }

        if rtn.is_empty()
        {
            Option::None
        }
        else {
            Option::Some(rtn)
        }
    }

    pub fn handle_builders(
        &self,
        info : TronInfo,
        builders: Result<Option<Vec<MessageBuilder>>,Error>,
    ) -> Vec<Message> {
        /*            match builders {
                       None => Ok(Option::None),
                       Some(builders) =>
                           {
                               let mut rtn = vec!();
                               for mut builder in builders {
                                   builder.from = Option::Some(self.from(info.clone()));
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
    fn add_tron(&self, state: &mut MutexGuard<State>, key: &TronKey, kind: u8) -> Result<(), Error> {
        let index = state.data.get_length(&path!("trons"))?;
        let path = Path::new(path!["trons", index.to_string()]);
        key.append(&path.push(path!["id"]), &mut state.meta);
        state.data.set(&path.plus("kind"), kind)?;

        Ok(())
    }

    fn set_tron_name(
        &self,
        state: &mut MutexGuard<State>,
        name: &str,
        key: &TronKey,
    ) -> Result<(), Error> {
        key.append(&Path::new(path!["tron_names"]), &mut state.meta);
        Ok(())
    }
}

impl Neutron {
    fn init() -> Result<Box<Tron>, Error> {
        Ok(Box::new(Neutron {}))
    }

    pub fn valid_neutron_id(id: Id) -> bool {
        return id.id == 0;
    }

    pub fn create_tron(
        &self,
        info: TronInfo,
        context: &mut dyn NeutronContext,
        state: Arc<Mutex<State>>,
        create: &Message,
    ) -> Result<(), Error> {
        let mut neutron_state = state.lock()?;

        let tron_seq_id = neutron_state.data.get::<i64>(&path!["tron_seq_id"] )?;
        let mut tron_seq = neutron_state.data.get::<i64>(&path!["tron_seq"] )?;
        tron_seq = tron_seq+1;
        neutron_state.data.set( &path!["tron_seq"], tron_seq );

        let tron_key = TronKey::new(info.key.nucleus.clone(), Id::new(tron_seq_id,tron_seq));
        let interface = NeutronStateInterface {};
        interface.add_tron(& mut neutron_state, &tron_key, 0)?;

        let create_meta = &create.payloads[0].buffer;
        if create_meta.is_set::<String>(&path![&"lookup_name"])?
        {
            let name = create_meta.get::<String>(&path![&"lookup_name"])?;
            interface.set_tron_name(& mut neutron_state, name.as_str(), &tron_key);
        }

        let tron_config = create_meta.get::<String>(&path![&"artifact"])?;
        let tron_config = Artifact::from(&tron_config)?;
        let tron_config = context.configs().trons.get(&tron_config)?;


        let mut tron_state = Arc::new(Mutex::new(State::new(context.configs(), tron_config.state.artifact.clone())?));

        {
            let mut tron_state = tron_state.lock()?;

            tron_state.meta.set(&path![&"artifact"], tron_config.source.to());
            tron_state.meta.set(&path![&"creation_timestamp"], context.timestamp());
            tron_state.meta.set(&path![&"creation_cycle"], context.revision().cycle);
        }


        context.create(tron_key,tron_config.source.clone(), tron_state, create );

        Ok(())
    }
}

impl Tron for Neutron {
    fn create(
        &self,
        info: TronInfo,
        context: &dyn TronContext,
        state: Arc<Mutex<State>>,
        create: &Message,
    ) -> Result<Option<Vec<MessageBuilder>>, Error> {

        let mut state = state.lock()?;

        let interface = NeutronStateInterface {};

        //neutron adds itself to the tron manifest
        interface.add_tron(&mut state, &info.key, 0)?;
        interface.set_tron_name(&mut state, "neutron", &info.key)?;

        if create.payloads[1].buffer.is_set::<String>(&path![&"nucleus_lookup_name"])?
        {
            // then we need to pass a message to the simtron to add a lookup name for this nucleus
            let mut builder = MessageBuilder::new();
            builder.to_tron_lookup_name = Option::Some("simtron".to_string());
            builder.to_nucleus_lookup_name= Option::Some("simulation".to_string());
            builder.to_phase = Option::Some(0);
            builder.kind = Option::Some(MessageKind::Update);

            let factory = context.configs().schemas.get(&CORE_SCHEMA_NUCLEUS_LOOKUP_NAME_MESSAGE)?;
            let buffer = factory.new_buffer(Option::None);
            let mut buffer = Buffer::new(buffer);
            let nucleus_lookup_name: String = create.payloads[1].buffer.get(&path!["nucleus_lookup_name"])?;
            buffer.set( &path!["name"],nucleus_lookup_name );
            info.key.nucleus.append(&Path::just("id"), &mut buffer )?;
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

    fn update(&self, phase: &str) -> Result<fn(TronInfo, &dyn TronContext, Arc<Mutex<State>>) -> Result<Option<Vec<MessageBuilder>>, Error>, Error> {
        unimplemented!()
    }

    fn port(&self, port: &str) -> Result<fn(TronInfo, &dyn TronContext, Arc<Mutex<State>>, &Vec<&Message>) -> Result<Option<Vec<MessageBuilder>>, Error>, Error> {
        unimplemented!()
    }

    fn extra(&self, port: &str) -> Result<fn(TronInfo, &dyn TronContext, &ReadOnlyState, &Message) -> Result<Option<Vec<MessageBuilder>>, Error>, Error> {
        unimplemented!()
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
    ) -> Result<Self, Error> {

        let meta_factory = configs.schemas.get(&CORE_SCHEMA_META_CREATE)?;
        let mut meta = Buffer::new(meta_factory.new_buffer(Option::None));
        meta.set(&path![&"artifact"], tron_config.source.to())?;
        let (constructor_artifact, constructor) =
            CreatePayloadsBuilder::constructor(configs, tron_config)?;
        Ok(CreatePayloadsBuilder {
            meta: meta,
            constructor_artifact: constructor_artifact,
            constructor: constructor,
        })
    }
    pub fn set_sim_id(&mut self, sim_id: &Id) -> Result<(), Error> {
        sim_id.append(&Path::just("sim_id"), &mut self.constructor)?;
        Ok(())
    }

    pub fn set_lookup_name(&mut self, lookup_name: &str) -> Result<(), Error> {
        self.meta.set(&path![&"lookup_name"], lookup_name)?;
        Ok(())
    }

    pub fn set_sim_config(&mut self, sim_config: &SimConfig) -> Result<(), Error> {
        self.constructor
            .set(&path!["sim_config_artifact"], sim_config.source.to())?;
        Ok(())
    }

    fn constructor(
        configs: &Configs,
        tron_config: &TronConfig,
    ) -> Result<(Artifact, Buffer), Error> {
            let constructor_artifact = tron_config.message.create.artifact.clone();
            let factory = configs.schemas.get(&constructor_artifact)?;
            let constructor = factory.new_buffer(Option::None);
            let constructor = Buffer::new(constructor);

            Ok((constructor_artifact, constructor))
    }

    pub fn payloads<'configs>(configs: &'configs Configs, builder: CreatePayloadsBuilder) -> Vec<Payload> {
        let meta_artifact = CORE_SCHEMA_META_CREATE.clone();
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


pub fn init_tron(config: &TronConfig) -> Result<Box<dyn Tron>, Error> {

    let rtn: Box<Tron> = match config.kind.as_str() {
        "neutron" => Neutron::init()? as Box<Tron>,
        _ => return Err(format!("we don't have a tron of kind {}", config.kind).into()),
    };

    Ok(rtn)
}
