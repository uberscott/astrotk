use crate::node::{Node, NucleusContext, Local};
use crate::nucleus::Nucleus;
use crate::mechtron::CreatePayloadsBuilder;
use mechtron_core::configs::SimConfig;
use mechtron_core::core::*;
use mechtron_core::id::{Id, TronKey};
use mechtron_core::message::{Message, MessageKind, To, TronLayer, Cycle, DeliveryMoment};
use std::sync::Arc;
use std::time::SystemTime;
use crate::error::Error;
use crate::router::Router;

pub struct SimulationBootstrap {}



impl SimulationBootstrap {
    pub fn bootstrap(local: Arc<Local>, config: Arc<SimConfig>) -> Result<(), Error> {
        let sim_id = local.seq().next();

        let sim_nucleus_config = local.cache().configs.nucleus.get(&CORE_NUCLEUS_SIMULATION)?;
        let sim_nucleus_id = local.create_source_nucleus(sim_id.clone(),sim_nucleus_config.clone(),Option::Some("sim".to_string()))?;

        let simtron_config = local.cache().configs.mechtrons.get( &CORE_MECHTRON_SIMTRON )?;
        let simtron_bind = local.cache().configs.binds.get( &simtron_config.bind.artifact )?;
        {
            let mut builder = CreatePayloadsBuilder::new(&local.cache().configs, &simtron_bind)?;
            builder.set_lookup_name("simtron");
            builder.constructor.set( &path!["sim_config_artifact"], config.source.to() )?;

            let neutron_key = TronKey::new(sim_nucleus_id.clone(),Id::new(sim_nucleus_id.seq_id, 0));

            let message = Message::multi_payload(
                                   local.seq().clone(),
                                   MessageKind::Create,
                                  mechtron_core::message::From{
                                        tron: neutron_key.clone(),
                                        cycle: 0,
                                        timestamp: 0,
                                        layer: TronLayer::Shell
                                    },
                                    To{
                                        tron: neutron_key.clone(),
                                        port: "create_simtron".to_string(),
                                        cycle: Cycle::Present,
                                        phase: "default".to_string(),
                                        delivery: DeliveryMoment::ExtraCyclic,
                                        layer: TronLayer::Kernel
                                    },
                                        CreatePayloadsBuilder::payloads(&local.cache().configs,builder));
           local.send( Arc::new(message));
        }
        Ok(())
    }
}
