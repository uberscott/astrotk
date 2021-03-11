use std::sync::{Arc, MutexGuard};

use mechtron::mechtron::{ExtraCyclicMessageHandler, Mechtron, MessageHandler, Response, Context};
use mechtron::CONFIGS;
use mechtron_common::core::*;
use mechtron_common::api::{CreateApiCallCreateNucleus, NeutronApiCallCreateMechtron};
use mechtron_common::artifact::Artifact;
use mechtron_common::buffers::{Buffer, Path};
use mechtron_common::error::Error;
use mechtron_common::id::{Id, MechtronKey};
use mechtron_common::message::{Cycle, MechtronLayer, Message, MessageBuilder, MessageKind, Payload};
use mechtron_common::state::{NeutronStateInterface, ReadOnlyState, State};

pub struct Neutron {
    context: Context
}


impl Neutron {

    pub fn new(context:Context)->Self
    {
        Neutron{
            context: context
        }
    }


    pub fn valid_neutron_id(id: Id) -> bool {
        return id.id == 0;
    }

    fn inbound__create_mechtrons(
        context: &Context,
        state: &mut MutexGuard<State>,
        messages: Vec<Message> ) -> Result<Response, Error>
    {
        let mut builders = vec!();
        for message in messages
        {
           let builder = Neutron::inbound__create_mechtron(context, state, &message)?;
           builders.push(builder);
        }

        Ok(Response::Messages(builders))
    }

    fn inbound__create_mechtron(
        context: &Context,
        neutron_state: &mut MutexGuard<State>,
        create_message: &Message,
    ) -> Result<MessageBuilder, Error> {
        // a simple helper interface for working with neutron state
        let neutron_state_interface = NeutronStateInterface {};

        // grab the new mechtron create meta
        let new_mechtron_create_meta = &create_message.payloads[0].buffer;

        // and derive the new mechtron config
        let new_mechtron_config = new_mechtron_create_meta.get::<String>(&path![&"config"])?;
println!("CREATE MECHTRON {}",new_mechtron_config);
        let new_mechtron_config = Artifact::from(&new_mechtron_config)?;
        let new_mechtron_config = CONFIGS.mechtrons.get(&new_mechtron_config)?;

        // increment the neutron's mechtron_index
        let mut mechtron_index = neutron_state.buffers.get("data").unwrap().get::<i64>(&path!["mechtron_index"] )?;
        mechtron_index = mechtron_index +1;
        neutron_state_interface.set_mechtron_index(neutron_state, mechtron_index);

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

        // prepare an api call to the MechtronShell to create this new mechtron
        let mut call = NeutronApiCallCreateMechtron::new(&CONFIGS, new_mechtron_config.clone(), create_message )?;

        // set some additional meta information about the new mechtron
        {
            new_mechtron_id.append(&Path::new(path!("id")), &mut call.state.meta)?;
            call.state.meta.set(&path![&"mechtron_config"], new_mechtron_config.source.to())?;
            call.state.meta.set(&path![&"creation_cycle"], context.cycle)?;
//            call.state.meta.set(&path![&"creation_timestamp"], context.timestamp() as i64)?;
            call.state.meta.set(&path![&"taint"], false)?;
        }

        let mut builder = MessageBuilder::new();
        builder.kind = Option::Some(MessageKind::Api);
        builder.to_layer = Option::Some(MechtronLayer::Shell);
        builder.to_nucleus_id=Option::Some(context.key.nucleus.clone());
        builder.to_tron_id=Option::Some(context.key.mechtron.clone());
        builder.to_cycle_kind=Option::Some(Cycle::Present);
        builder.payloads.replace(Option::Some(NeutronApiCallCreateMechtron::payloads(call,&CONFIGS)?));

        Ok(builder)
    }
}

impl Mechtron for Neutron {
    fn on_create(&self, state: &mut MutexGuard<State>, create_message: &Message) -> Result<Response, Error>
    {
        let interface = NeutronStateInterface {};

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
    fn on_update(&self, state: & mut MutexGuard<State>) -> Result<Response, Error>
    {
        Ok(Response::None)
    }

    fn on_messages(&self, port: &str) -> Result<MessageHandler, Error>
    {
        match port{
            "create" => Ok(MessageHandler::Handler(Neutron::inbound__create_mechtrons)),
            _ => Err(format!("port not available: {}", port).into())
        }
    }

    fn on_extras(&self, port: &str) -> Result<ExtraCyclicMessageHandler, Error>
    {
        Ok(ExtraCyclicMessageHandler::None)
    }
}


