use mechtron_common::configs::SimConfig;
use std::sync::Arc;
use std::error::Error;
use crate::nucleus::Nucleus;
use mechtron_common::artifact::ArtifactCacher;
use crate::tron::CreatePayloadsBuilder;
use mechtron_common::message::{Message, To, MessageKind};
use crate::app::SYS;
use crate::message::MessageRouter;
use mechtron_common::id::{Id, TronKey};
use std::time::SystemTime;

pub struct SimulationBootstrap
{
}

fn timestamp()->i64
{
    let since_the_epoch = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH );
    let timestamp = since_the_epoch.as_secs() * 1000 +
        since_the_epoch.subsec_nanos() as u64 / 1_000_000;

    return timestamp;
}

impl SimulationBootstrap{

    pub fn launch(sim_config:Arc<SimConfig>)->Result<(),Box<dyn Error>> {

        sim_config.cache(&mut SYS.local.configs)?;

        let sim_id = SYS.net.id_seq.next();

        let nucleus_id = Nucleus::new(sim_id, Option::Some("simulation".to_string()))?;
        let neutron_key = TronKey{ nucleus: nucleus_id.clone(), tron: Id { seq_id: nucleus_id.seq_id.clone(), id: 0 }};

        let simtron_config = SYS.local.configs.core_tron_config("trons/sim")?;
        let mut sim_create_payload_builder = CreatePayloadsBuilder::new(&SYS.local.configs, &simtron_config)?;
        sim_create_payload_builder.set_lookup_name("simtron");
        sim_create_payload_builder.set_sim_config(&sim_config);

        let message = Message::multi_payload(&mut SYS.net.id_seq,
                                             MessageKind::Create,
                                             mechtron_common::message::From { tron: neutron_key.clone(), cycle: 0, timestamp: timestamp() },
                                             To::basic(
                                                 neutron_key.clone(),
                                                 "create".to_string(),
                                             ),
                                             CreatePayloadsBuilder::payloads(&SYS.local.configs, sim_create_payload_builder));


        SYS.router.send(message);

        Ok(())
    }
}