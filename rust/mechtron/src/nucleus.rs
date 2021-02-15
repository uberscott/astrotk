use std::error::Error;
use std::sync::Arc;

use no_proto::memory::NP_Memory_Owned;

use mechtron_common::core::*;
use mechtron_common::artifact::{Artifact, ArtifactCacher};
use mechtron_common::configs::{Configs, SimConfig, TronConfig};
use mechtron_common::state::{State, ReadOnlyState,ReadOnlyStateMeta};
use mechtron_common::id::{StateKey, Id};
use mechtron_common::id::Revision;
use mechtron_common::id::TronKey;
use mechtron_common::message::{Cycle, Message, MessageKind, Payload, To, DeliveryMoment};

use crate::app::SYS;
use crate::tron::{TronContext, CreatePayloadsBuilder, init_tron, Neutron, Tron, TronShell};
use crate::state::{NucleusStateStructure, StateIntake, NucleusPhasicStateStructure};
use crate::message::{NucleusMessagingStructure, MessageRouter, IntakeContext, NucleusPhasicMessagingStructure, OutboundMessaging};
use std::time::SystemTime;

pub struct Nucleus
{
    id : Id,
    sim_id : Id,
    state: NucleusStateStructure,
    messaging: NucleusMessagingStructure,
    head: Revision
}

fn timestamp()->i64
{
    let since_the_epoch = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH );
    let timestamp = since_the_epoch.as_secs() * 1000 +
        since_the_epoch.subsec_nanos() as u64 / 1_000_000;

    return timestamp;
}

impl Nucleus
{
    pub fn new(sim_id:Id, lookup_name: Option<String>)->Result<Id,Box<dyn Error>>
    {
        let mut nucleus = Nucleus {
            id: SYS.net.id_seq.next(),
            sim_id: sim_id,
            state: NucleusStateStructure::new(),
            messaging: NucleusMessagingStructure::new(),
            head: Revision { cycle: 0 }
        };

        let id = nucleus.bootstrap(lookup_name)?;

        SYS.local.nuclei.add(nucleus);

        return Ok(id)
    }


    fn bootstrap(&mut self, lookup_name: Option<String>) -> Result<Id, Box<dyn Error>>
    {
        let timestamp = timestamp();

        let neutron_config = SYS.local.configs.core_tron_config("tron/neutron")?;
        let neutron_key = TronKey::new( self.id.clone(), Id::new(self.id.seq_id, 0));

        let context = TronContext {
            sim_id: self.sim_id.clone(),
            key: neutron_key.clone(),
            revision: self.head.clone(),
            tron_config: neutron_config.clone(),
            timestamp: timestamp.clone(),
        };

        // first we create a neutron for the simulation nucleus
        let mut neutron_create_payload_builder = CreatePayloadsBuilder::new(&SYS.local.configs, &neutron_config)?;
        neutron_create_payload_builder.set_sim_id( &self.sim_id );
        if lookup_name.is_some()
        {
            neutron_create_payload_builder.set_lookup_name(&lookup_name.unwrap());
        }

        let create = Message::multi_payload(&mut SYS.net.id_seq,
                                            MessageKind::Create,
                                            mechtron_common::message::From { tron: context.key.clone(), cycle: 0, timestamp },
                                            To {
                                                tron: context.key.clone(),
                                                port: "create".to_string(),
                                                cycle: Cycle::Next,
                                                phase: 0,
                                                delivery: DeliveryMoment::Cyclic,
                                            },
                                            CreatePayloadsBuilder::payloads(&SYS.local.configs, neutron_create_payload_builder));


        let neutron_state_artifact = neutron_config.state.unwrap().artifact;
        let mut state = State::new(&SYS.local.configs, neutron_state_artifact.clone() )?;

        let neutron_config = SYS.local.configs.tron_config_keeper.get( &CORE_NEUTRON_CONFIG );
        let neutron = TronShell::new( init_tron(&neutron_config)? );
        neutron.create(&context,&mut state,&create);

        let state_key = StateKey {
            tron: context.key,
            revision: context.revision.clone(),
        };

        self.state.intake(state, state_key)?;

        Ok(context.key.nucleus.clone())
    }

    pub fn intake( &mut self, message: Message )
    {
       self.messaging.intake(message, self );
    }

    pub fn revise( &mut self, from: Revision, to: Revision )->Result<(),Box<dyn Error>>
    {
        if from.cycle != to.cycle - 1
        {
            return Err("cycles must be sequential. 'from' revision cycle must be exactly 1 less than 'to' revision cycle".into());
        }

        let context = RevisionContext{
            revision: to.clone(),
            timestamp: timestamp(),
            phase: "dunno".to_string()
        };

        let mut nucleus_cycle = NucleusCycle::init(self.id.clone(), context);

        for message in self.messaging.query(&to.cycle)?
        {
            nucleus_cycle.intake_message(message)?;
        }

        for (key,state) in self.state.query(&context.revision)?
        {
            nucleus_cycle.intake_state(key,state)?;
        }

        nucleus_cycle.step()?;

        let (states, messages) = nucleus_cycle.commit();

        self.head = to;

        for (key, state) in states
        {
            self.state.intake(state,key)?;
        }

        for message in messages{
            SYS.router.send(message);
        }

        Ok(())
    }
}

impl IntakeContext for Nucleus
{
    fn head(&self) -> i64 {
        self.head.cycle
    }

    fn phase(&self) -> u8 {
        -1
    }
}

struct NucleusCycle
{
    id: Id,
    state:  NucleusPhasicStateStructure,
    messaging: NucleusPhasicMessagingStructure,
    outbound: OutboundMessaging,
    context: RevisionContext,
    phase: u8
}

impl NucleusCycle
{
    fn init(id: Id, context: RevisionContext, ) -> Self
    {
        NucleusCycle {
            id: id,
            state: NucleusPhasicStateStructure::new(),
            messaging: NucleusPhasicMessagingStructure::new(),
            outbound: OutboundMessaging::new(),
            context: context,
            phase: 0
        }
    }

    fn intake_message(&mut self, message: Arc<Message>) -> Result<(), Box<dyn Error>>
    {
        self.messaging.intake(message)?;
        Ok(())
    }

    fn intake_state(&mut self, key: TronKey, state: &ReadOnlyState) -> Result<(), Box<dyn Error>>
    {
        self.state.intake(key, state)?;
        Ok(())
    }

    fn step(&mut self)
    {
        let phase: u8 = 0;
        match self.messaging.remove(&phase)?
        {
            None => {}
            Some(messages) => {
                for message in messages
                {
                    self.process(message.as_ref())?;
                }
            }
        }
    }

    fn commit(&mut self)->(Vec<(StateKey,State)>,Vec<Message>)
    {
        (self.state.drain(self.context.revision()),self.outbound.drain())
    }

    fn tron( &mut self, key: &TronKey )->Result<(Arc<TronConfig>, &mut State, TronShell, TronContext),Box<dyn Error>>
    {
        let mut state  = self.state.get(key)?;
        let artifact = state.get_artifact()?;
        let tron_config = SYS.local.configs.tron_config_keeper.get(&artifact)?;
        let tron_shell = TronShell::new(init_tron(&tron_config)? )?;

        let context = TronContext {
            sim_id: self.sim_id.clone(),
            key: key.clone(),
            revision: Revision { cycle: self.context.cycle },
            tron_config: tron_config.clone(),
            timestamp: self.context.timestamp.clone(),
        };


        Ok( (tron_config, state, tron_shell,context))
    }


    fn process(&mut self, message: &Message) -> Result<(), Box<dyn Error>>
    {
        match message.kind {
            MessageKind::Create => self.process_create(message),
            MessageKind::Update=> self.process_update(message),
            _ => Err("not implemented yet".into())
        }
    }

    fn process_create(&mut self, message: &Message) -> Result<(), Box<dyn Error>>
    {
        // ensure this is addressed to a neutron
        if !Neutron::valid_neutron_id(message.to.tron.tron_id.clone())
        {
            Err(format!("not a valid neutron id: {}", message.to.tron.tron_id.id.clone()).into())
        }

        let neutron_state_key = StateKey { tron: message.to.tron.clone(), revision: Revision { cycle: self.context.revision.cycle } };

        let mut neutron_state = self.state.get(&neutron_state_key.tron_id)?;

        let neutron_config = SYS.local.configs.tron_config_keeper.get(&CORE_NEUTRON_CONFIG)?;

        let context = TronContext {
            sim_id: self.sim_id.clone(),
            key: message.to.tron.clone(),
            revision: Revision { cycle: self.context.cycle },
            tron_config: neutron_config,
            timestamp: self.context.timestamp.clone(),
        };

        let neutron = Neutron{};
        neutron.create_tron(&context, neutron_state , message);

        Ok(())
    }

    fn process_update(&mut self, message: &Message )->Result<(),Box<dyn Error>>
    {
        let state_key = StateKey { tron: message.to.tron.clone(), revision: Revision { cycle: self.context.revision.cycle } };
        let (config,state,tron,context ) = self.tron(&message.to.tron )?;
    }
}

impl MessageRouter for NucleusCycle{
    fn send(&mut self, message: Message) -> Result<(), Box<dyn Error>> {

        if message.to.tron.nucleus_id != self.id
        {
            self.outbound.push(message);
            return Ok(());
        }

        match message.to.cycle
        {
            Cycle::Exact(cycle) => {
                if self.context.revision.cycle != cycle
                {
                    self.outbound.push(message);
                    return Ok(());
                }
            }
            Cycle::Present => {}
            Cycle::Next => {
                self.outbound.push(message);
                return Ok(());
            }
        }

        match &message.to.inter_delivery_type
        {
            DeliveryMoment::Cyclic => {
                self.outbound.push(message);
                return Ok(());
            }
            DeliveryMoment::Phasic => {
                if message.to.phase >= self.phase
                {
                    self.outbound.push(message);
                    return Ok(());
                }
                else {
                    self.intake(message);
                    return Ok(());
                }
            }
        }
    }
}





struct RevisionContext
{
    revision: Revision,
    timestamp: i64,
    phase: String,
}

impl RevisionContext
{
    pub fn configs(&self) -> &Configs
    {
        &SYS.local.configs
    }

    pub fn revision(&self) -> &Revision {
        &self.revision
    }

    pub fn timestamp(&self) -> i64
    {
        self.timestamp
    }

    pub fn phase(&self) -> &str {
        self.phase.as_str()
    }
}