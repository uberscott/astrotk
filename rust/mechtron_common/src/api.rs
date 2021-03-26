use crate::artifact::Artifact;
use crate::buffers::Buffer;
use crate::configs::{Configs, BindConfig, MechtronConfig, NucleusConfig, NucleusConfigRef};
use crate::core::*;
use crate::error::Error;
use crate::message::{Message, Payload};
use crate::state::{State, StateMeta};
use std::sync::Arc;
use crate::logger::log;

pub struct NeutronApiCallCreateMechtron<'message>
{
   pub meta: Buffer,
   pub state: State,
   pub create_message: &'message Message
}

impl <'message> NeutronApiCallCreateMechtron<'message> {
   pub fn new(configs: &Configs, config: Arc<MechtronConfig>, create_message: &'message Message ) -> Result<Self, Error> {
      let mut meta = Buffer::new(
         configs
             .schemas
             .get(&CORE_SCHEMA_META_API)?
             .new_buffer(Option::None),
      );

log( "NeutronAPiCallCreateMechtron entry?");
      meta.set(&path!["api"], "neutron_api")?;
      meta.set(&path!["call"], "create_mechtron")?;
log( "NeutronAPiCallCreateMechtron whatta going on?");
       let mut state = State::new(configs, config)?;
log( "NeutronAPiCallCreateMechtron new state is ready!??");

      Ok(NeutronApiCallCreateMechtron {
         meta: meta,
         state: state,
         create_message: create_message
      })
   }

   pub fn payloads(call: NeutronApiCallCreateMechtron, configs: &Configs) -> Result<Vec<Payload>, Error>
   {

log( "NeutronAPiCallCreateMechtron payload time!");
      Ok(vec![Payload{
                 buffer: call.meta.read_only().to_bytes(),
                 schema: CORE_SCHEMA_META_API.clone(),
              },
              Payload{
                 buffer: call.state.meta.read_only().to_bytes(),
                 schema: CORE_SCHEMA_META_STATE.clone(),
              },
               Message::to_payload(call.create_message,configs)?

      ])
   }

}

pub struct CreateApiCallCreateNucleus
{
    nucleus_config: NucleusConfigRef
}

impl CreateApiCallCreateNucleus{

    pub fn new(nucleus_config: NucleusConfigRef ) -> Self
    {
        CreateApiCallCreateNucleus{
            nucleus_config: nucleus_config
        }
    }

    pub fn payloads(call: CreateApiCallCreateNucleus, configs: &Configs) -> Result<Vec<Payload>, Error>
    {
        let mut meta = Buffer::new(
            configs
                .schemas
                .get(&CORE_SCHEMA_META_API)?
                .new_buffer(Option::None),
        );

        meta.set(&path!["api"], "create_api")?;
        meta.set(&path!["call"], "create_nucleus")?;

        let factory = configs.schemas.get( &CORE_SCHEMA_ARTIFACT )?;
        let mut buffer = factory.new_buffer(Option::None);
        let mut buffer = Buffer::new(buffer);
        buffer.set(&path![], call.nucleus_config.artifact.to() );
        Ok(vec![Payload{
            buffer: meta.read_only().to_bytes(),
            schema: CORE_SCHEMA_META_API.clone(),
        },
            Payload{
            buffer: buffer.read_only().to_bytes(),
            schema: CORE_SCHEMA_ARTIFACT.clone(),
        },
        ])
    }

    pub fn nucleus_config_artifact( payloads: &Vec<Payload>, configs: &Configs  ) -> Result<String,Error>
    {
        Ok(payloads[1].read_only(configs)?.get::<String>( &path![] )?)
    }


}
