use mechtron_common::configs::SimConfig;
use std::sync::Arc;
use std::error::Error;
use crate::nucleus::Nucleus;
use mechtron_common::artifact::ArtifactCacher;
use crate::tron::CreatePayloadsBuilder;
use mechtron_common::buffers::set;
use mechtron_common::message::{Message, To};
use crate::app::SYS;

struct SimulationBootstrap
{
}

impl SimulationBootstrap{

    pub fn launch(sim_config:Arc<SimConfig>)->Result<(),Box<dyn Error>> {

        sim_config.cache(&mut SYS.local.configs)?;

        let sim_id = SYS.net.id_seq.next();

        let nucleus_id = Nucleus::new(sim_id, Option::Some("simulation".to_string()))?;

        let simtron_config = SYS.local.configs.core_tron_config("trons/sim")?;
        let mut sim_create_payload_builder = CreatePayloadsBuilder::new(&SYS.local.configs, &simtron_config)?;
        sim_create_payload_builder.set_lookup_name("simtron");
        set(&mut sim_create_payload_builder.constructor, &[&"sim_config_artifact"], sim_config.source.to())?;


        let message = Message::multi_payload(&mut SYS.net.id_seq,
                                             MesssageKind::Create,
                                             mechtron_common::message::From { tron: context.id.clone(), cycle: 0, timestamp },
                                             To::basic(
                                                 neutron_key.clone(),
                                                 "create".to_string(),
                                             ),
                                             CreatePayloadsBuilder::payloads(&SYS.local.configs, sim_create_payload_builder));


        SYS.router.send(message);

        Ok(())
    }
}