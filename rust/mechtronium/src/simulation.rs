use crate::node::{Node, NucleusContext};
use crate::nucleus::Nucleus;
use crate::tron::CreatePayloadsBuilder;
use mechtron_core::configs::SimConfig;
use mechtron_core::core::*;
use mechtron_core::id::{Id, TronKey};
use mechtron_core::message::{Message, MessageKind, To};
use std::error::Error;
use std::sync::Arc;
use std::time::SystemTime;

pub struct SimulationBootstrap {}

fn timestamp() -> Result<u64, Box<dyn Error>> {
    let since_the_epoch = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;
    let timestamp =
        since_the_epoch.as_secs() * 1000 + since_the_epoch.subsec_nanos() as u64 / 1_000_000;

    return Ok(timestamp);
}

impl SimulationBootstrap {
    pub fn bootstrap(sys: Arc<Node>, sim_config: Arc<SimConfig>) -> Result<(), Box<dyn Error>> {
        //        sim_config.cache(&mut sys.local.configs)?;

        /*
        let sim_id = sys.net.id_seq.next();

        let nucleus_id = Nucleus::new(
            sim_id,
            Option::Some("simulation".to_string()),
            NucleusContext::new(sys.clone()),
        )?;
        let neutron_key = TronKey {
            nucleus: nucleus_id.clone(),
            tron: Id {
                seq_id: nucleus_id.seq_id.clone(),
                id: 0,
            },
        };

        let simtron_config = sys
            .local
            .configs
            .tron_config_keeper
            .get(&CORE_SIMTRON_CONFIG)?;
        let mut sim_create_payload_builder =
            CreatePayloadsBuilder::new(&sys.local.configs, &simtron_config)?;
        sim_create_payload_builder.set_lookup_name("simtron");
        sim_create_payload_builder.set_sim_config(&sim_config);

        let message = Message::multi_payload(
            &mut sys.net.id_seq,
            MessageKind::Create,
            mechtron_core::message::From {
                tron: neutron_key.clone(),
                cycle: 0,
                timestamp: timestamp()?,
            },
            To::basic(neutron_key.clone(), "create".to_string()),
            CreatePayloadsBuilder::payloads(&sys.local.configs, sim_create_payload_builder),
        );

        sys.router.send(message);

        Ok(())
         */
        unimplemented!()
    }
}
