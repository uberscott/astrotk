use crate::artifact::Artifact;
use crate::buffers::Buffer;
use crate::configs::{Configs, BindConfig};
use crate::core::*;
use crate::error::Error;
use crate::message::PayloadBuilder;
use crate::state::{State, StateMeta};
use std::sync::Arc;

pub struct NeutronApiCallCreateMechtron
{
   pub meta: Buffer,
   pub state: State,
}

impl NeutronApiCallCreateMechtron {
   pub fn new<'configs>(configs: &'configs Configs, bind: Arc<BindConfig> ) -> Result<Self, Error> {
      let mut meta = Buffer::new(
         configs
             .schemas
             .get(&CORE_SCHEMA_META_API)?
             .new_buffer(Option::None),
      );

println!("HIYO");
      meta.set(&path!["api"], "neutron_api");
      meta.set(&path!["call"], "create_mechtron");

       let mut state = State::new(configs, bind)?;
println!("AND... BLAH");

      Ok(NeutronApiCallCreateMechtron {
         meta: meta,
         state: state,
      })
   }

   pub fn payloads(call: NeutronApiCallCreateMechtron) -> Result<Vec<PayloadBuilder>, Error>
   {
      Ok(vec![PayloadBuilder {
                 buffer: call.meta,
                 artifact: CORE_SCHEMA_META_API.clone(),
              },
              PayloadBuilder {
                 buffer: call.state.meta,
                 artifact: CORE_SCHEMA_META_STATE.clone(),
              },
              PayloadBuilder {
                 buffer: call.state.data,
                 artifact: call.state.artifact
              }])
   }
}
