use core::cell::RefCell;
use core::default::Default;
use core::fmt::Debug;
use core::option::Option;
use core::option::Option::{None, Some};
use core::result::Result;
use core::result::Result::{Err, Ok};
use std::collections::HashMap;
use std::sync::{Arc, MutexGuard, Mutex};

use mechtron_common::buffers::ReadOnlyBuffer;
use mechtron_common::id::{Id, MechtronKey};
use mechtron_common::message::{Cycle, DeliveryMoment, MechtronLayer, Message, MessageBuilder, MessageKind, Payload};
use mechtron_common::state::{ReadOnlyState, ReadOnlyStateMeta, State, StateMeta};
use mechtron_common::util::PongPayloadBuilder;

use crate::error::Error;
use crate::nucleus::{MechtronShellContext, TronInfo};
use mechtron_common::api::CreateApiCallCreateNucleus;
use mechtron_common::artifact::Artifact;
use mechtron_common::configs::{PanicEscalation, Configs};
use crate::membrane::MechtronMembrane;
use mechtron_common::mechtron::Context;
use std::cmp::Ordering;

pub struct MechtronShell<'a> {
    pub info: TronInfo,
    pub outbound: RefCell<Vec<Message>>,
    pub panic: RefCell<Option<String>>,
    pub kernel: MutexGuard<'a,MechtronMembrane>
}


impl <'a> MechtronShell<'a> {
    pub fn new(kernel: MutexGuard<'a,MechtronMembrane>, key: MechtronKey, configs: &Configs ) -> Result<Self,Error> {

        let (config, bind) = {
            let config = kernel.get_mechtron_config();
            let bind = configs.binds.get(&config.bind.artifact)?;
            (config, bind)
        };

        let info = TronInfo {
            key: key.clone(),
            config: config.clone(),
            bind: bind,
        };

        Ok(MechtronShell {
            info: info,
            kernel: kernel,
            outbound: RefCell::new(vec!()),
            panic: RefCell::new(Option::None),
        })
    }



    fn warn<E: Debug>(&self, error: E)
    {
        println!("WARN: TronShell got unexpected error {:?}", error);
    }

    fn panic<E: Debug>(&self, error: E)
    {
        if !self.is_panic()
        {
            let message = format!("PANIC: TronShell got unexpected error {:?}", error);
            println!("{}",&message);
            self.panic.replace(Option::Some(message));
        }
    }

    pub fn check( &self, context: &dyn MechtronShellContext )
    {
        if self.is_panic() && (self.info.bind.panic_escalation == PanicEscalation::Nucleus || self.info.bind.panic_escalation == PanicEscalation::Simulation )
        {
            context.panic( self.panic.borrow().as_ref().unwrap().clone() );
        }
    }

    pub fn is_panic(&self) -> bool
    {
        self.panic.borrow().is_some()
    }

    pub fn is_tainted(&self)->Result<bool,Error>
    {
        self.kernel.is_tainted()
    }

    fn reject(&mut self, message: &Message, reason: &str, context: &dyn MechtronShellContext, layer: MechtronLayer)
    {
        println!("{}", reason);
        // we unrwrap it because if REJECT message isn't available, then nothing should work
        let message = message.reject(self.from(context, layer), reason, context.seq(), context.configs()).unwrap();
        self.send(message)
    }

    fn ok(&mut self, message: &Message, ok: bool, context: &dyn MechtronShellContext, layer: MechtronLayer)
    {
        // we unrwrap it because if REJECT message isn't available, then nothing should work
        let message = message.ok(self.from(context, layer), ok, context.seq(), context.configs()).unwrap();
        self.send(message)
    }

    fn respond(&mut self, message: &Message, payloads: Vec<Payload>, context: &dyn MechtronShellContext, layer: MechtronLayer)
    {
        let message = message.respond(self.from(context, layer), payloads, context.seq());
        self.send(message);
    }

    fn from(&self, context: &dyn MechtronShellContext, layer: MechtronLayer) -> mechtron_common::message::From {
        mechtron_common::message::From {
            tron: self.info.key.clone(),
            cycle: context.revision().cycle.clone(),
            timestamp: context.timestamp(),
            layer: layer,
        }
    }

    fn send(&mut self, message: Message)
    {
        self.outbound.borrow_mut().push(message);
    }

    pub fn flush(&self) -> Vec<Message>
    {
        self.outbound.replace(Default::default())
    }

    pub fn create(
        &mut self,
        create: &Message,
        context: &dyn MechtronShellContext
    ) {
        match self.create_result(create, context)
        {
            Ok(_) => {}
            Err(error) => {
                self.panic(error.clone());
                self.kernel.set_taint(format!("CREATE {:?}",error).as_str());
                self.check(context);
            }
        }
    }

    fn create_result(
        &mut self,
        create: &Message,
        context: &dyn MechtronShellContext,
    ) -> Result<(), Error> {
        if self.kernel.is_tainted()?
        {
            return Err("mechtron state is tainted".into());
        }
        let kernel_context = Context::new( self.info.key.clone(), context.revision().cycle, context.phase() );
        let builders = self.kernel.create(&kernel_context,create)?;
unimplemented!();
        self.handle(Option::Some(builders), context)?;

        Ok(())
    }

    pub fn messages(
        &mut self,
        messages: Vec<Arc<Message>>,
        context: &dyn MechtronShellContext
    ) {
        match self.messages_result(messages, context)
        {
            Ok(_) => {}
            Err(error) => {
                self.panic(error.clone());
                self.kernel.set_taint(format!("MESSAGES: {:?}", error).as_str());
            }
        }
    }

    pub fn messages_result(
        &mut self,
        messages: Vec<Arc<Message>>,
        context: &dyn MechtronShellContext
    ) -> Result<(), Error> {
        if self.kernel.is_tainted()?
        {
            return Err("mechtron state is tainted".into());
        }

        let (_,messages) = Self::split( messages );
        let messages = Self::sort( messages );
        let kernel_context = Context::new( self.info.key.clone(), context.revision().cycle, context.phase() );

        for port in messages.keys()
        {
            for message in messages.get(port).unwrap()
            {
                self.kernel.message(&kernel_context, &message);
            }
        }

        Ok(())
    }

    pub fn extra(
        &mut self,
        message: &Message,
        context: &dyn MechtronShellContext
    )
    {
        match self.extra_result(message, context)
        {
            Ok(_) => {}
            Err(err) => {
                self.warn(err);
            }
        }
    }

    fn extra_result(
        &mut self,
        message: &Message,
        context: &dyn MechtronShellContext
    ) -> Result<(), Error>
    {
        /*
        if self.kernel.get_state().is_tainted()?
        {
            return Err("mechtron state is tainted".into());
        }
        println!("entered EXTRA");
        match message.to.layer
        {
            MechtronLayer::Shell => {
                match message.to.port.as_str() {
                    "ping" => {
                        println!("PING!!!");
                        self.respond(message, vec!(PongPayloadBuilder::new(context.configs()).unwrap()), context, MechtronLayer::Shell);
                    }
                    "pong" => {
                        println!("PONG!!!");
                    }

                    _ => {
                        self.reject(message, format!("TronShell has no extra port: {}", message.to.port.clone()).as_str(), context, MechtronLayer::Shell);
                    }
                };
            }
            MechtronLayer::Kernel => {
                let bind = context.configs().binds.get(&self.info.config.bind.artifact).unwrap();
                match bind.message.extra.contains_key(&message.to.port)
                {
                    true => {
                        let func = self.tron.extra(&message.to.port);
                        match func {
                            Ok(func) => {
                                let builders = func(self.info.clone(), context, &state, message)?;
                                self.handle(builders, context);
                            }
                            Err(e) => {
                                self.warn(e);
                            }
                        }
                    }
                    false => {
                        self.reject(message, format!("extra cyclic port '{}' does not exist on this mechtron", message.to.port).as_str(), context, MechtronLayer::Shell);
                    }
                }
            }
        }
        Ok(())

         */
        Ok(())
    }



    fn handle(
        &self,
        builders: Option<Vec<MessageBuilder>>,
        context: &dyn MechtronShellContext,
    ) -> Result<(), Error> {
unimplemented!();
        match builders {
            None => Ok(()),
            Some(builders) => {
                for mut builder in builders
                {
                    builder.from = Option::Some(self.from(context, MechtronLayer::Kernel));

                    if builder.to_nucleus_lookup_name.is_some()
                    {
                        let nucleus_id = context.lookup_nucleus(&builder.to_nucleus_lookup_name.unwrap())?;
                        builder.to_nucleus_id = Option::Some(nucleus_id);
                        builder.to_nucleus_lookup_name = Option::None;
                    }

                    if builder.to_tron_lookup_name.is_some()
                    {
                        let nucleus_id = builder.to_nucleus_id;
                        match nucleus_id {
                            None => {
                                // do nothing. builder.build() will panic for us
                            }
                            Some(nucleus_id) => {
                                let tron_key = context.lookup_mechtron(&nucleus_id, &builder.to_tron_lookup_name.unwrap().as_str())?;
                                builder.to_tron_id = Option::Some(tron_key.mechtron);
                                builder.to_tron_lookup_name = Option::None;
                            }
                        }
                    }

                    if builder.kind.as_ref().is_some() && builder.kind.as_ref().unwrap().clone() == MessageKind::Api
                    {
                        // handle API message
                        self.handle_api_call(builder, context)?;
                    } else {
                        let message = builder.build(context.seq().clone())?;
                        self.outbound.borrow_mut().push(message);
                    }
                }

                Ok(())
            }
        }
    }


    fn handle_api_call(
        &self,
        mut builder: MessageBuilder,
        context: &dyn MechtronShellContext,
    ) -> Result<(), Error>
    {
        builder.to_cycle_kind = Option::Some(Cycle::Present);
        builder.to_nucleus_id = Option::Some(self.info.key.nucleus.clone());
        builder.to_tron_id = Option::Some(self.info.key.mechtron.clone());
        builder.to_delivery = Option::Some(DeliveryMoment::Phasic);
        builder.to_port = Option::Some("api".to_string());
        builder.to_layer = Option::Some(MechtronLayer::Shell);
        builder.to_phase = Option::Some("default".to_string());
        builder.from = Option::Some(self.from(context, MechtronLayer::Kernel));
        let message = builder.build(context.seq().clone())?;
        let bind = context.configs().binds.get(&self.info.config.bind.artifact).unwrap();

        let api = message.payloads[0].buffer.get::<String>(&path!["api"])?;

unimplemented!();
        match api.as_str() {
            "neutron_api" => {
                // need some test to make sure this is actually a neutron
                if !bind.kind.eq("Neutron")
                {
                    self.panic(format!("attempt for non Neutron to access neutron_api {}", bind.kind));
                } else {
                    let call = message.payloads[0].buffer.get::<String>(&path!["call"])?;
                    match call.as_str() {
                        "create_mechtron" => {
                            // now get the state of the mechtronmessage.payloads
                            let new_mechtron_state = State::new_from_meta(context.configs(), message.payloads[1].buffer.copy_to_buffer())?;
                            let new_mechtron_state = new_mechtron_state.read_only()?;

                            // very wasteful to be cloning the bytes here...
                            let create_message = message.payloads[2].buffer.read_bytes().to_vec();
                            let create_message = Message::from_bytes(create_message, context.configs())?;
                            context.neutron_api_create(new_mechtron_state, create_message);
                        }
                        _ => { return Err(format!("we don't have an api {} call {}", api, call).into()); }
                    }
                }
            },
            "create_api" => {
                let call = message.payloads[0].buffer.get::<String>(&path!["call"])?;
                match call.as_str(){
                    "create_nucleus" => {
                        let nucleus_config = CreateApiCallCreateNucleus::nucleus_config_artifact( &message.payloads )?;
                        let nucleus_config = Artifact::from(&nucleus_config)?;
                        context.configs().cache( &nucleus_config )?;
                        let nucleus_config = context.configs().nucleus.get(&nucleus_config)?;
                        context.create_api_create_nucleus(nucleus_config);
                    },
                    _ => { return Err(format!("we don't have an api {} call {}", api, call).into()); }
                }
            },
            _ => { return Err(format!("we don't have an api {}", api).into()); }
        }

        Ok(())
    }

    fn split(messages: Vec<Arc<Message>>) ->(Vec<Arc<Message>>, Vec<Arc<Message>>)
    {
        let mut shell = vec!();
        let mut kernel = vec!();

        for message in messages
        {
            match message.to.layer
            {
                MechtronLayer::Shell => {shell.push(message)}
                MechtronLayer::Kernel => {kernel.push(message)}
            }
        }

        (shell,kernel)
    }

    fn sort(messages: Vec<Arc<Message>>) ->HashMap<String,Vec<Arc<Message>>>
    {
        let mut rtn = HashMap::new();

        for message in messages
        {
            let port = message.to.port.clone();
            if !rtn.contains_key(&port)
            {
                rtn.insert( port.clone(), vec!() );
            }
            let messages = rtn.get_mut(&port).unwrap();
            messages.push(message);
        }

        for (key,mut messages) in rtn.iter_mut()
        {
            messages.sort_by( |a,b| {
                if a.id.seq_id == b.id.seq_id
                {
                    if a.id.id < b.id.id
                    {
                        Ordering::Less
                    }
                    else
                    {
                        Ordering::Greater
                    }
                }
                else if a.id.seq_id < b.id.seq_id
                {
                    Ordering::Less
                }
                else{
                    Ordering::Greater
                }
            });
        }

        rtn
    }
}


