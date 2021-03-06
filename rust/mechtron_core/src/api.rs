use crate::artifact::Artifact;
use crate::buffers::Buffer;
use crate::configs::{Configs, BindConfig, MechtronConfig};
use crate::core::*;
use crate::error::Error;
use crate::message::{Message, Payload};
use crate::state::{State, StateMeta};
use std::sync::Arc;

pub struct NeutronApiCallCreateMechtron
{
   pub meta: Buffer,
   pub state: State,
   pub create_message: Message
}

impl NeutronApiCallCreateMechtron {
   pub fn new<'configs>(configs: &'configs Configs, config: Arc<MechtronConfig>, create_message: Arc<Message> ) -> Result<Self, Error> {
      let mut meta = Buffer::new(
         configs
             .schemas
             .get(&CORE_SCHEMA_META_API)?
             .new_buffer(Option::None),
      );

      meta.set(&path!["api"], "neutron_api")?;
      meta.set(&path!["call"], "create_mechtron")?;

       let mut state = State::new(configs, config)?;

      Ok(NeutronApiCallCreateMechtron {
         meta: meta,
         state: state,
         create_message: (*create_message).clone()
      })
   }

   pub fn payloads<'config>(call: NeutronApiCallCreateMechtron, configs: &Configs<'config>) -> Result<Vec<Payload>, Error>
   {
      Ok(vec![Payload{
                 buffer: call.meta.read_only(),
                 schema: CORE_SCHEMA_META_API.clone(),
              },
              Payload{
                 buffer: call.state.data.read_only(),
                 schema: CORE_SCHEMA_META_STATE.clone(),
              },
               Message::to_payload(call.create_message,configs)?

      ])
   }

}
