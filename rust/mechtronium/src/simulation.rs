use crate::node::{Node, NucleusContext, Local};
use crate::nucleus::Nucleus;
use crate::mechtron::CreatePayloadsBuilder;
use mechtron_common::configs::SimConfig;
use mechtron_common::core::*;
use mechtron_common::id::{Id, MechtronKey};
use mechtron_common::message::{Message, MessageKind, To, MechtronLayer, Cycle, DeliveryMoment};
use std::sync::Arc;
use std::time::SystemTime;
use crate::error::Error;
use crate::router::InternalRouter;

pub struct Simulation {}



impl Simulation {
    pub fn bootstrap(local: Arc<Local>, config: Arc<SimConfig>) -> Result<(), Error> {
        let sim_id = local.seq().next();

        let sim_nucleus_config = local.cache().configs.nucleus.get(&CORE_NUCLEUS_SIMULATION)?;
        let sim_nucleus_id = local.create_source_nucleus(sim_id.clone(),sim_nucleus_config.clone(),Option::Some("sim".to_string()))?;

        let simtron_config = local.cache().configs.mechtrons.get( &CORE_MECHTRON_SIMTRON )?;
        {
            let mut builder = CreatePayloadsBuilder::new(&local.cache().configs, simtron_config.clone())?;
            builder.set_lookup_name("simtron");
            builder.constructor.set( &path!["sim_config_artifact"], config.source.to() )?;

            let neutron_key = MechtronKey::new(sim_nucleus_id.clone(), Id::new(sim_nucleus_id.seq_id, 0));

            let message = Message::multi_payload(
                local.seq().clone(),
                MessageKind::Create,
                mechtron_common::message::From{
                                        tron: neutron_key.clone(),
                                        cycle: 0,
                                        timestamp: 0,
                                        layer: MechtronLayer::Shell
                                    },
                To{
                                        tron: neutron_key.clone(),
                                        port: "create_simtron".to_string(),
                                        cycle: Cycle::Present,
                                        phase: "default".to_string(),
                                        delivery: DeliveryMoment::ExtraCyclic,
                                        layer: MechtronLayer::Kernel
                                    },
                CreatePayloadsBuilder::payloads(&local.cache().configs,builder));
           local.send( Arc::new(message));
        }
        Ok(())
    }
}

