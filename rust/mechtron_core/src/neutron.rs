use std::sync::{Arc, MutexGuard};

use mechtron::mechtron::{Mechtron, MessageHandler, Response};
use mechtron::CONFIGS;
use mechtron_common::core::*;
use mechtron_common::api::{CreateApiCallCreateNucleus, NeutronApiCallCreateMechtron};
use mechtron_common::artifact::Artifact;
use mechtron_common::buffers::{Buffer, Path};
use mechtron_common::error::Error;
use mechtron_common::id::{Id, MechtronKey};
use mechtron_common::message::{Cycle, MechtronLayer, Message, MessageBuilder, MessageKind, Payload};
use mechtron_common::state::{NeutronStateInterface, ReadOnlyState, State, StateMeta};
use mechtron_common::mechtron::Context;
use std::rc::Rc;
use std::cell::{Cell, RefCell};
use mechtron::membrane::log;
use mechtron_common::configs::MechtronConfig;

pub struct Neutron {
    context: Context,
    state: Rc<RefCell<Option<Box<State>>>>
}


impl Neutron {

    pub fn new(context:Context,state:Rc<RefCell<Option<Box<State>>>>)->Self
    {
        Neutron{
            context: context,
            state:state
        }
    }


    pub fn valid_neutron_id(id: Id) -> bool {
        return id.id == 0;
    }


    fn inbound__create_mechtron(
        context: &Context,
        neutron_state: &mut State,
        create_message: Message,
    ) -> Result<Response, Error> {

log("neutron","creating a new mechtron...");
        // a simple helper interface for working with neutron state
        let neutron_state_interface = NeutronStateInterface {};

        // grab the new mechtron create meta
        let new_mechtron_create_meta = &create_message.payloads[0].buffer;

log("neutron","where is starhelix");
        // and derive the new mechtron config
        let new_mechtron_config = new_mechtron_create_meta.get::<String>(&path![&"config"])?;
log("neutron","uno");
        let new_mechtron_config = Artifact::from(&new_mechtron_config)?;
log("neutron",format!("duso for {}",new_mechtron_config.to()).as_str());
// must find a better way to do this!
// all caching should be done before the simulation starts
CONFIGS.cache(&new_mechtron_config);
        let new_mechtron_config = CONFIGS.mechtrons.get(&new_mechtron_config);
        match &new_mechtron_config
        {
            Ok(_) => {
                log("neutron", "Was okay?");
            }
            Err(err) => {
                log("neutron", format!("{:?}",err).as_str());
            }
        }
log("neutron","configs......");
        let new_mechtron_config = new_mechtron_config.unwrap();


        // increment the neutron's mechtron_index
        let mut mechtron_index = neutron_state.buffers.get("data").unwrap().get::<i64>(&path!["mechtron_index"] )?;
        mechtron_index = mechtron_index +1;
        neutron_state_interface.set_mechtron_index(neutron_state, mechtron_index);

log("neutron","state interface......");
        // create the new mechtron id and key
        let new_mechtron_id= Id::new(context.key.nucleus.id,mechtron_index);
        let new_mechtron_key = MechtronKey::new(context.key.nucleus.clone(), new_mechtron_id );

        // add the new mechtron to the neutron/nucleus manifest
        neutron_state_interface.add_mechtron(& mut *neutron_state, &new_mechtron_key, new_mechtron_config.source.to() )?;

        // if the new mechtron has a lookup name, add it
        if new_mechtron_create_meta.is_set::<String>(&path![&"lookup_name"])?
        {
            let name = new_mechtron_create_meta.get::<String>(&path![&"lookup_name"])?;
            neutron_state_interface.set_mechtron_name(& mut *neutron_state, name.as_str(), &new_mechtron_key);
        }
log("neutron","moving right along....");

        // prepare an api call to the MechtronShell to create this new mechtron
        let mut call = NeutronApiCallCreateMechtron::new(&CONFIGS, new_mechtron_config.clone(), &create_message )?;

log("neutron","there's gotta be a bug here somewhere....");
        // set some additional meta information about the new mechtron
        {
            new_mechtron_id.append(&Path::new(path!("id")), &mut call.state.meta)?;
            call.state.set_mechtron_config(new_mechtron_config);
            call.state.meta.set(&path![&"creation_cycle"], context.cycle)?;
//            call.state.meta.set(&path![&"creation_timestamp"], context.timestamp() as i64)?;
            call.state.meta.set(&path![&"taint"], false)?;
        }

log("debug","Vlad");
        let mut builder = MessageBuilder::new();
        builder.kind = Option::Some(MessageKind::Api);
        builder.to_layer = Option::Some(MechtronLayer::Shell);
        builder.to_nucleus_id=Option::Some(context.key.nucleus.clone());
        builder.to_tron_id=Option::Some(context.key.mechtron.clone());
        builder.to_cycle_kind=Option::Some(Cycle::Present);
        builder.to_port = Option::Some("api".to_string());
        builder.payloads.replace(Option::Some(NeutronApiCallCreateMechtron::payloads(call,&CONFIGS)?));

log("debug","OK NEUTRON CREATE FINISHED");
        Ok(Response::Messages(vec!(builder)))
    }
}

impl Mechtron for Neutron {
    fn create(&mut self, create_message: &Message) -> Result<Response, Error> {

        let interface = NeutronStateInterface {};


        let mut state = self.state.borrow_mut();
        let state = state.as_mut().unwrap();
        let config = state.config.clone();


        //neutron adds itself to the tron manifest
        interface.add_mechtron(state, &self.context.key, config.source.to())?;
        interface.set_mechtron_name(state, "neutron", &self.context.key)?;
        interface.set_mechtron_index(state, 0);


        if create_message.payloads[1].buffer.is_set::<String>(&path![&"nucleus_lookup_name"])?
        {
            // then we need to pass a message to the simtron to add a lookup name for this nucleus
            let mut builder = MessageBuilder::new();
            builder.to_tron_lookup_name = Option::Some("simtron".to_string());
            builder.to_nucleus_lookup_name = Option::Some("simulation".to_string());
            builder.to_phase = Option::Some("default".to_string());
            builder.kind = Option::Some(MessageKind::Update);

            let factory = CONFIGS.schemas.get(&CORE_SCHEMA_NUCLEUS_LOOKUP_NAME_MESSAGE)?;
            let buffer = factory.new_buffer(Option::None);
            let mut buffer = Buffer::new(buffer);
            let nucleus_lookup_name: String = create_message.payloads[1].buffer.get(&path!["nucleus_lookup_name"])?;
            buffer.set(&path!["name"], nucleus_lookup_name);
            self.context.key.nucleus.append(&Path::just("id"), &mut buffer)?;
            let payload = Payload {
                buffer: buffer.read_only(),
                schema: CORE_SCHEMA_NUCLEUS_LOOKUP_NAME_MESSAGE.clone()
            };
            let payloads = vec!(payload);
            builder.payloads.replace(Option::Some(payloads));

            Ok(Response::Messages(vec!(builder)))
        } else {
            Ok(Response::None)
        }
    }

    fn message(&mut self, port: &str) -> Result<MessageHandler, Error> {
        Ok(match port{
            "create" => MessageHandler::Handler(Neutron::inbound__create_mechtron),
            _ => MessageHandler::None
        })
    }


    fn update(&mut self) -> Result<Response, Error> {
        Ok(Response::None)
    }


    fn extra(&self, port: &str) -> Result<MessageHandler, Error> {
        Ok(MessageHandler::None)
    }

}


