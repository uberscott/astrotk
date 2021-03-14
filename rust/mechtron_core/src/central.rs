use core::option::Option;
use core::option::Option::{None, Some};
use core::result::Result;
use core::result::Result::{Err, Ok};
use mechtron::CONFIGS;
use mechtron_common::api::{CreateApiCallCreateNucleus};
use mechtron_common::artifact::Artifact;
use mechtron_common::error::Error;
use mechtron_common::message::{Cycle, MechtronLayer, Message, MessageBuilder, MessageKind, Payload, To};
use mechtron_common::state::{ReadOnlyState, State};
use mechtron_common::buffers::{Path};
use mechtron_common::core::*;
use mechtron::mechtron::{Mechtron, Response,  MessageHandler};
use std::sync::MutexGuard;
use mechtron_common::mechtron::Context;
use std::rc::Rc;
use std::cell::{Cell, RefCell};
use mechtron_common::logger::log;
use mechtron_common::buffers::Buffer;
use mechtron_common::configs::Configs;
use mechtron_common::id::Id;

pub struct CentralMechtron {
    context: Context,
    state: Rc<RefCell<Option<Box<State>>>>
}

impl CentralMechtron {

    pub fn new(context:Context, state: Rc<RefCell<Option<Box<State>>>>)->Self
    {
        CentralMechtron {
            context: context,
            state: state
        }
    }
}

impl CentralMechtron
{
    fn inbound__set_nucleus_to_node(
        context: &Context,
        state: &mut State,
        message: Message,
    ) -> Result<Response, Error> {

        Ok(Response::None)
    }

    fn inbound__get_node_for_nucleus(
        context: &Context,
        state: &mut State,
        message: Message,
    ) -> Result<Response, Error> {


        let mut builder = context.message_builder();
        builder.api_call();

        let nucleus_id = Id::from(&Path::new(path![]), &message.payloads[0].buffer )?;

        let mut call = TypeIdValueApiRequest::new(&CONFIGS)?;
        call.set_type("nucleus_to_node".to_string());
        call.set_id(nucleus_id);

        let mut builder = MessageBuilder::new();
        builder.kind = Option::Some(MessageKind::Api);
        builder.to_layer = Option::Some(MechtronLayer::Shell);
        builder.to_nucleus_id=Option::Some(context.key.nucleus.clone());
        builder.to_tron_id=Option::Some(context.key.mechtron.clone());
        builder.to_cycle_kind=Option::Some(Cycle::Present);
        builder.to_port = Option::Some("api".to_string());
        builder.payloads.replace(Option::Some(TypeIdValueApiRequest::payloads(call, &CONFIGS)?));
        builder.callback = message.callback.clone();

        Ok(Response::None)
    }
}

impl Mechtron for CentralMechtron {



   fn create(&mut self, create_message: &Message) -> Result<Response, Error>
   {
       Ok(Response::None)
   }

    fn update(&mut self) -> Result<Response, Error> {
        Ok(Response::None)
    }

    fn message(&mut self, port: &str) -> Result<MessageHandler, Error> {
        Ok(MessageHandler::None)
    }

    fn extra(&self, port: &str) -> Result<MessageHandler, Error> {
        Ok(MessageHandler::None)
    }

}


pub struct TypeIdValueApiRequest
{
    pub meta: Buffer,
    pub type_id: Buffer
}

impl TypeIdValueApiRequest {
    pub fn new(configs: &Configs) -> Result<Self, Error> {
        let mut meta = Buffer::new(
            configs
                .schemas
                .get(&CORE_SCHEMA_META_API)?
                .new_buffer(Option::None),
        );

        let type_id = Buffer::new(
            configs
                .schemas
                .get(&CORE_SCHEMA_TYPE_ID)?
                .new_buffer(Option::None),
        );

        meta.set(&path!["api"], "type_id_value_store")?;
        meta.set(&path!["call"], "get")?;

        Ok(TypeIdValueApiRequest {
            meta: meta,
            type_id: type_id,
        })
    }

    pub fn set_id( &mut self, id: Id )->Result<(),Error>
    {
        let path = Path::new(path![]);
        id.append( &path, &mut self.type_id )
    }

    pub fn set_type( &mut self, kind: String )->Result<(),Error>
    {
        self.type_id.set(&path!["type"], kind)
    }

    pub fn payloads(call: TypeIdValueApiRequest, configs: &Configs) -> Result<Vec<Payload>, Error>
    {
        Ok(vec![Payload{
            buffer: call.meta.read_only(),
            schema: CORE_SCHEMA_META_API.clone(),
        },
                Payload{
                    buffer: call.type_id.read_only(),
                    schema: CORE_SCHEMA_TYPE_ID.clone(),
                },
        ])
    }

}









pub struct SetTypeIdValueApiRequest
{
    pub meta: Buffer,
    pub type_id: Buffer
}

impl SetTypeIdValueApiRequest {
    pub fn new(configs: &Configs) -> Result<Self, Error> {
        let mut meta = Buffer::new(
            configs
                .schemas
                .get(&CORE_SCHEMA_META_API)?
                .new_buffer(Option::None),
        );

        let type_id = Buffer::new(
            configs
                .schemas
                .get(&CORE_SCHEMA_TYPE_ID_VALUE)?
                .new_buffer(Option::None),
        );

        meta.set(&path!["api"], "type_id_value_store")?;
        meta.set(&path!["call"], "get")?;

        Ok(SetTypeIdValueApiRequest {
            meta: meta,
            type_id: type_id,
        })
    }

    pub fn set_id( &mut self, id: Id )->Result<(),Error>
    {
        let path = Path::new(path!["key"]);
        id.append( &path, &mut self.type_id )
    }

    pub fn set_type( &mut self, kind: String )->Result<(),Error>
    {
        self.type_id.set(&path!["key,","type"], kind)
    }

    pub fn set_value( &mut self, id: Id )->Result<(),Error>
    {
        let path = Path::new(path!["value"]);
        id.append( &path, &mut self.type_id )
    }


    pub fn payloads(call: SetTypeIdValueApiRequest, configs: &Configs) -> Result<Vec<Payload>, Error>
    {
        Ok(vec![Payload{
            buffer: call.meta.read_only(),
            schema: CORE_SCHEMA_META_API.clone(),
        },
                Payload{
                    buffer: call.type_id.read_only(),
                    schema: CORE_SCHEMA_TYPE_ID.clone(),
                },
        ])
    }

}