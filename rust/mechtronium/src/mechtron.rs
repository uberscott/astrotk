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

use mechtron_core::artifact::Artifact;
use mechtron_core::buffers;
use mechtron_core::buffers::{Buffer, Path};
use mechtron_core::configs::{
    BindConfig, Configs, CreateMessageConfig, MessageConfig, SimConfig,
};
use mechtron_core::core::*;
use mechtron_core::id::{Id, NucleusKey, Revision, StateKey, TronKey};
use mechtron_core::mechtron::MechtronContext;
use mechtron_core::message::{Message, MessageBuilder, MessageKind, Payload, PayloadBuilder, TronLayer};
use mechtron_core::state::{ReadOnlyState, ReadOnlyStateMeta, State, StateMeta};
use mechtron_core::util::PongPayloadBuilder;

use crate::error::Error;
use crate::node::Node;
use crate::nucleus::{MechtronShellContext, NeutronContext, Nucleus};

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
    pub key: TronKey,
    pub config: Arc<BindConfig>,
}

impl TronInfo {
    pub fn new(
        key: TronKey,
        tron_config: Arc<BindConfig>,
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


pub struct MechtronShell {
    pub tron: Box<dyn MechtronKernel>,
    pub info: TronInfo,
    pub outbound: RefCell<Vec<Message>>,
    pub panic: bool
}


impl MechtronShell {
    pub fn new(tron: Box<dyn MechtronKernel>, info: TronInfo  ) -> Self {
        MechtronShell { tron: tron,
                    info: info,
                    outbound: RefCell::new(vec!()),
                    panic: false}
    }

    fn warn<E:Debug>( &self, error: E )
    {
       println!("WARN: TronShell got unexpected error {:?}",error);
    }

    fn panic<E:Debug>( &self, error: E )
    {
        println!("PANIC: TronShell got unexpected error {:?}",error);
    }

    fn reject(&mut self, message: &Message, reason: &str, context: &dyn MechtronShellContext, layer: TronLayer )
    {
        // we unrwrap it because if REJECT message isn't available, then nothing should work
        let message = message.reject(self.from(context,layer), reason, context.seq(), context.configs()).unwrap();
        self.send(message)
    }

    fn ok(&mut self, message: &Message, ok: bool, context: &dyn MechtronShellContext, layer: TronLayer   )
    {
        // we unrwrap it because if REJECT message isn't available, then nothing should work
        let message = message.ok(self.from(context,layer), ok, context.seq(), context.configs()).unwrap();
        self.send(message)
    }

    fn respond(&mut self, message: &Message, payloads: Vec<Payload>, context: &dyn MechtronShellContext, layer: TronLayer   )
    {
        let message = message.respond(self.from(context,layer), payloads, context.seq());
        self.send(message);
    }

    fn from(&self, context: &dyn MechtronShellContext, layer: TronLayer  ) -> mechtron_core::message::From {
        mechtron_core::message::From {
            tron: self.info.key.clone(),
            cycle: context.revision().cycle.clone(),
            timestamp: context.timestamp(),
            layer: layer
        }
    }

    fn send(&mut self, message: Message )
    {
        self.outbound.borrow_mut().push(message);
    }

    pub fn flush(&self)->Vec<Message>
    {
        self.outbound.replace(Default::default())
    }

    pub fn taint(
        &mut self,
        state: &mut MutexGuard<State>
    ) ->Result<(),Error>  {

        state.set_taint(true);
        Ok(())
    }

    pub fn is_tainted(
        &self,
        state: TronShellState
    ) ->Result<bool,Error>  {

        match state{
            TronShellState::Mutable(state) => {
                Ok(state.is_tainted()?)
            }
            TronShellState::ReadOnly(state) => {
                Ok(state.is_tainted()?)
            }
        }
    }

    pub fn create(
        &mut self,
        create: &Message,
        context: &dyn MechtronShellContext,
        state: &mut State
    ) ->Result<(),Error>  {

        let mut state = state;
        let mut builders = self.tron.create(self.info.clone(), context, state, create);
        self.handle(builders,context);
        Ok(())
    }
    pub fn extra(
        &mut self,
        message: &Message,
        context: &dyn MechtronShellContext,
        state: Arc<ReadOnlyState>
    )
    {


        println!("entered EXTRA");
        match message.to.layer
        {
            TronLayer::Shell => {
                match message.to.port.as_str(){
                   "ping" => {
println!("PING!!!");
                        self.respond(message, vec!(PongPayloadBuilder::new(context.configs()).unwrap()),context,TronLayer::Shell);
                    }
                   "pong" => {
                        println!("PONG!!!");
                    }

                    _ => {
                       self.reject(message, format!("TronShell has no extra port: {}", message.to.port.clone()).as_str(),context, TronLayer::Shell );
                    }
                };
            }
            TronLayer::Kernel => {
                match self.info.config.message.extra.contains_key(&message.to.port)
                {
                    true => {
                        let func = self.tron.extra(&message.to.port);
                        match func{
                            Ok(func) => {
                                let builders = func( self.info.clone(),context, &state,message);
                                self.handle(builders,context);
                            }
                            Err(e) => {
                                self.panic(e);
                            }
                        }
                    }
                    false => {
                        self.reject( message, format!("extra cyclic port '{}' does not exist on this mechtron", message.to.port).as_str(),context,TronLayer::Shell);
                    }
                }
            }
        }
    }


    pub fn inbound(
        &mut self,
        messages: Vec<&Message>,
        context: &dyn MechtronShellContext,
        state: &mut MutexGuard<State>
    ) {
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

        ports.sort();
        for port in ports
        {
            let messages = hash.get(port).unwrap();
            match self.info.config.message.inbound.contains_key( port )
            {
                true => {
                    let func = self.tron.port(port);
                    match func
                    {
                        Ok(func) => {
                            self.handle(func( self.info.clone(),context,state,messages ),context);
                        }
                        Err(e) => {
                            self.panic(e);
                        }
                    }

                }
                false => {
                    for message in messages{
                        self.reject(message, format!("mechtron {} does not have an inbound port {}", self.info.config.source.to(), port).as_str(),context, TronLayer::Shell );
                    }
                }
            }

        }


    }

    pub fn handle(
        &self,
        builders: Result<Option<Vec<MessageBuilder>>,Error>,
        context: &dyn MechtronShellContext
    )  {
                    match builders {
                        Ok(builders) => {
                            match builders{
                                None => {}
                                Some(builders) => {
                                    for mut builder in builders
                                    {
                                        builder.from = Option::Some( self.from(context, TronLayer::Kernel ) );

                                        if builder.to_nucleus_lookup_name.is_some()
                                        {
                                            let nucleus_id = context.lookup_nucleus(&builder.to_nucleus_lookup_name.unwrap() );
                                            match nucleus_id{
                                                Ok(nucleus_id) => {
                                                    builder.to_nucleus_id = Option::Some(nucleus_id);
                                                    builder.to_nucleus_lookup_name = Option::None;
                                                }
                                                Err(e) => {
                                                    self.panic(e);
                                                    return;
                                                }
                                            }
                                        }

                                        if builder.to_tron_lookup_name.is_some()
                                        {

                                            let nucleus_id =builder.to_nucleus_id;
                                            match nucleus_id{
                                                None => {
                                                    // do nothing. builder.build() will panic for us
                                                }
                                                Some(nucleus_id) => {
                                                    let tron_key = context.lookup_tron(&nucleus_id,&builder.to_tron_lookup_name.unwrap().as_str() );
                                                    match tron_key{
                                                        Ok(tron_key) => {
                                                            builder.to_tron_id = Option::Some(tron_key.tron);
                                                            builder.to_tron_lookup_name = Option::None;
                                                        }
                                                        Err(e) => {
                                                            self.panic(e);
                                                            return;
                                                        }
                                                    }
                                                }
                                            }
                                        }

                                        let message = builder.build(context.seq().clone() );
                                        match message {
                                            Ok(message) => {
                                                self.outbound.borrow_mut().push(message);
                                            }
                                            Err(e) => {
                                                self.panic(e);
                                                return;
                                            }
                                        }
                                    }
                                }
                            }

                        }
                        Err(e) => {
                           self.panic(e)
                        }
                    }
    }
}

pub struct Neutron {}

pub struct NeutronStateInterface {}

impl NeutronStateInterface {
    fn add_tron(&self, state: &mut State, key: &TronKey, kind: String) -> Result<(), Error> {
        println!("ADD TRON...");
        let index = {
            if state.data.is_set::<i64>(&path!("tron","0","id"))?
            {
                state.data.get_length(&path!("tron"))?
            } else {
                0
            }
        };

        let path = Path::new(path!["tron", index.to_string()]);
        key.append(&path.push(path!["id"]), &mut state.meta);
        state.data.set(&path.plus("kind"), kind)?;
        println!("TRON ADDED...");

        Ok(())
    }

    fn set_tron_name(
        &self,
        state: &mut State,
        name: &str,
        key: &TronKey,
    ) -> Result<(), Error> {
        key.append(&Path::new(path!["tron_names"]), &mut state.meta);
        Ok(())
    }
}

impl Neutron {
    fn init() -> Result<Box<MechtronKernel>, Error> {
        Ok(Box::new(Neutron {}))
    }

    pub fn valid_neutron_id(id: Id) -> bool {
        return id.id == 0;
    }

    pub fn create_mechtron(
        &self,
        info: TronInfo,
        context: &mut dyn NeutronContext,
        state: Arc<Mutex<State>>,
        create: &Message,
    ) -> Result<(), Error> {
        let mut neutron_state = state.lock()?;
        let create_meta = &create.payloads[0].buffer;
        let tron_config = create_meta.get::<String>(&path![&"artifact"])?;
        let tron_config = Artifact::from(&tron_config)?;
        let tron_config = context.configs().binds.get(&tron_config)?;


        let tron_seq_id = neutron_state.data.get::<i64>(&path!["tron_seq_id"] )?;
        let mut tron_seq = neutron_state.data.get::<i64>(&path!["tron_seq"] )?;
        tron_seq = tron_seq+1;
        neutron_state.data.set( &path!["tron_seq"], tron_seq );

        let tron_key = TronKey::new(info.key.nucleus.clone(), Id::new(tron_seq_id,tron_seq));
        let interface = NeutronStateInterface {};
        interface.add_tron(& mut neutron_state, &tron_key, tron_config.source.to() )?;


        if create_meta.is_set::<String>(&path![&"lookup_name"])?
        {
            let name = create_meta.get::<String>(&path![&"lookup_name"])?;
            interface.set_tron_name(& mut neutron_state, name.as_str(), &tron_key);
        }

        let mut tron_state = State::new(context.configs(), tron_config.state.artifact.clone())?;

        {
            tron_state.meta.set(&path![&"artifact"], tron_config.source.to());
            tron_state.meta.set(&path![&"creation_timestamp"], context.timestamp());
            tron_state.meta.set(&path![&"creation_cycle"], context.revision().cycle);
        }

        context.create(tron_key,tron_config.source.clone(), tron_state, create );

        Ok(())
    }

    fn extra_create_simtron(info: TronInfo, context: &dyn MechtronShellContext, state: &ReadOnlyState, message: &Message) -> Result<Option<Vec<MessageBuilder>>, Error>
    {
        Ok(Option::None)
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
        interface.add_tron(state, &info.key, info.config.source.to() )?;
        interface.set_tron_name(state, "neutron", &info.key)?;

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

    fn update(&self, phase: &str) -> Result<fn(TronInfo, &dyn MechtronShellContext, &mut MutexGuard<State>) -> Result<Option<Vec<MessageBuilder>>, Error>, Error> {
        unimplemented!()
    }

    fn port(&self, port: &str) -> Result<fn(TronInfo, &dyn MechtronShellContext, &mut MutexGuard<State>, &Vec<&Message>) -> Result<Option<Vec<MessageBuilder>>, Error>, Error> {
        unimplemented!()
    }

    fn extra(&self, port: &str) -> Result<fn(TronInfo, &dyn MechtronShellContext, &ReadOnlyState, &Message) -> Result<Option<Vec<MessageBuilder>>, Error>, Error> {
        match port{
            "create_simtron" => {
                Ok(Neutron::extra_create_simtron)
            }
            port => Err(format!("").into())
        }
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
        bind: &BindConfig,
    ) -> Result<Self, Error> {

        let meta_factory = configs.schemas.get(&CORE_SCHEMA_META_CREATE)?;
        let mut meta = Buffer::new(meta_factory.new_buffer(Option::None));
        meta.set(&path![&"artifact"], bind.source.to())?;
        let (constructor_artifact, constructor) =
            CreatePayloadsBuilder::constructor(configs, bind)?;
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
        tron_config: &BindConfig,
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


pub fn init_tron(config: &BindConfig) -> Result<Box<dyn MechtronKernel>, Error> {

    let rtn: Box<MechtronKernel> = match config.kind.as_str() {
        "neutron" => Neutron::init()? as Box<MechtronKernel>,
        _ => return Err(format!("we don't have a tron of kind {}", config.kind).into()),
    };

    Ok(rtn)
}


