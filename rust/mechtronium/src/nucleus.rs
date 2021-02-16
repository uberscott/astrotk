use std::error::Error;
use std::sync::{Arc, RwLock};

use no_proto::memory::NP_Memory_Owned;

use mechtron_core::artifact::{Artifact, ArtifactCacher};
use mechtron_core::configs::{Configs, SimConfig, TronConfig, Keeper};
use mechtron_core::core::*;
use mechtron_core::id::Revision;
use mechtron_core::id::TronKey;
use mechtron_core::id::{Id, StateKey};
use mechtron_core::message::{Cycle, DeliveryMoment, Message, MessageKind, Payload, To};
use mechtron_core::state::{ReadOnlyState, ReadOnlyStateMeta, State};

use crate::mechtronium::{Mechtronium, NucleusContext};
use crate::message::{
    IntakeContext, MessageRouter, NucleusMessagingStructure, NucleusPhasicMessagingStructure,
    OutboundMessaging,
};
use crate::state::{NucleusPhasicStateStructure, NucleusStateStructure, StateIntake};
use crate::tron::{init_tron, CreatePayloadsBuilder, Neutron, Tron, TronContext, TronShell};
use std::borrow::BorrowMut;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::SystemTime;

pub struct Nuclei<'nuclei> {
    nuclei: RwLock<HashMap<Id, Arc<Nucleus<'nuclei>>>>,
    sys: Option<Arc<Mechtronium<'nuclei>>>,
}

impl <'nuclei> Nuclei <'nuclei> {
    pub fn new() -> Self {
        Nuclei {
            nuclei: RwLock::new(HashMap::new()),
            sys: Option::None,
        }
    }

    pub fn init(&mut self, system: Arc<Mechtronium<'nuclei>>) {
        self.sys = Option::Some(system);
    }

    pub fn get<'get>(&'get self, nucleus_id: &Id) -> Result<Arc<Nucleus<'nuclei>>, Box<dyn Error + '_>> {
        let sources = self.nuclei.read()?;
        if !sources.contains_key(nucleus_id) {
            return Err(format!(
                "nucleus id {:?} is not present in the local nuclei",
                nucleus_id
            )
            .into());
        }

        let nucleus = sources.get(nucleus_id).unwrap();
        return Ok(nucleus.clone());
    }

    pub fn add(&self, nucleus: Nucleus<'nuclei>) -> Result<(), Box<dyn Error + '_>> {
        let mut sources = self.nuclei.write()?;
        sources.insert(nucleus.id.clone(), Arc::new(nucleus));
        Ok(())
    }
}

pub struct Nucleus<'nucleus> {
    pub id: Id,
    pub sim_id: Id,
    pub state: NucleusStateStructure,
    pub messaging: NucleusMessagingStructure,
    pub head: Revision,
    pub context: NucleusContext<'nucleus>,
}

fn timestamp() -> Result<u64, Box<dyn Error>> {
    let since_the_epoch = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;
    let timestamp =
        since_the_epoch.as_secs() * 1000 + since_the_epoch.subsec_nanos() as u64 / 1_000_000;

    return Ok(timestamp);
}

impl <'nucleus> Nucleus<'nucleus> {
    pub fn new(
        sim_id: Id,
        lookup_name: Option<String>,
        context: NucleusContext,
    ) -> Result<Id, Box<dyn Error+'_>> {

        /*
        let mut nucleus = Nucleus {
            id: context.sys().net.id_seq.next(),
            sim_id: sim_id,
            state: NucleusStateStructure::new(),
            messaging: NucleusMessagingStructure::new(),
            head: Revision { cycle: 0 },
            context: context.clone(),
        };

        let id = nucleus.bootstrap(lookup_name)?;

        context.sys().local.nuclei.add(nucleus);

        return Ok(id);
         */
        unimplemented!()
    }

    fn context_from(
        &self,
        key: TronKey,
        artifact: &Artifact,
    ) -> TronContext {
        let sys = self.context.sys();
        let local = sys.local();
        let configs = &local.configs;
        let tron_config_keeper = &configs.tron_config_keeper;
        let config = tron_config_keeper.get(&artifact).unwrap();

        let config = config.clone();
        let rtn = self.context_for(key, config);
        rtn
    }

    fn context_for(
        &self,
        key: TronKey,
        config: Arc<TronConfig>,
    ) -> TronContext {
        let sys = self.context.sys();
        TronContext::new(
            key,
            self.head.clone(),
            config.clone(),
            timestamp().unwrap().clone(),
        )
    }

    fn bootstrap(&mut self, lookup_name: Option<String>) -> Result<Id, Box<dyn Error+'_>> {
        /*
        let sys = self.context.sys();
        let configs = sys.configs();

        let timestamp = timestamp()?;

        let neutron_key = TronKey::new(self.id.clone(), Id::new(self.id.seq_id, 0));
        let context = self.context_from(neutron_key, &CORE_NEUTRON_CONFIG);

        // first we create a neutron for the simulation nucleus

        let mut neutron_create_payload_builder =
            CreatePayloadsBuilder::new(configs, &context.tron_config)?;
        neutron_create_payload_builder.set_sim_id(&self.sim_id);

        if lookup_name.is_some() {
            neutron_create_payload_builder.set_lookup_name(&lookup_name.unwrap());
        }

        let create = Message::multi_payload(
            &mut self.context.sys().net.id_seq,
            MessageKind::Create,
            mechtron_core::message::From {
                tron: context.key.clone(),
                cycle: 0,
                timestamp,
            },
            To {
                tron: context.key.clone(),
                port: "create".to_string(),
                cycle: Cycle::Next,
                phase: 0,
                delivery: DeliveryMoment::Cyclic,
            },
            CreatePayloadsBuilder::payloads(configs, neutron_create_payload_builder),
        );

        let neutron_state_artifact = context.tron_config.content.unwrap().artifact;
        let mut state = State::new(configs, neutron_state_artifact.clone())?;

        let neutron_config = configs.tron_config_keeper.get(&CORE_NEUTRON_CONFIG)?;
        let neutron = TronShell::new(init_tron(&context.tron_config)?);
        neutron.create(context.clone(), &mut state, &create);

        let state_key = StateKey {
            tron: context.key,
            revision: context.revision.clone(),
        };

        //self.state.intake(state, state_key)?;

        Ok(context.key.nucleus.clone())
         */
        unimplemented!()
    }

    pub fn intake(&self, message: Message) {
//        self.messaging.intake(message, self);
        unimplemented!()
    }

    pub fn revise(&mut self, from: Revision, to: Revision) -> Result<(), Box<dyn Error + '_>> {
        if from.cycle != to.cycle - 1 {
            return Err("cycles must be sequential. 'from' revision cycle must be exactly 1 less than 'to' revision cycle".into());
        }

        let context = NucleusCycleContext {
            revision: to.clone(),
            timestamp: timestamp()?,
            phase: "dunno".to_string(),
            nucleus: self
        };

        /*
        let mut nucleus_cycle = NucleusCycle::init(self.id.clone(), context, self)?;

        for message in self.messaging.query(&to.cycle)? {
            nucleus_cycle.intake_message(message)?;
        }

        match self.state.query(&context.revision) {
            None => {}
            Some(results) => {
                for (key, state) in results {
                    nucleus_cycle.intake_state(key, state)?;
                }
            }
        }

        nucleus_cycle.step();

        let (states, messages) = nucleus_cycle.commit();

        self.head = to;

        for (key, state) in states {
            self.state.intake(state, key)?;
        }

        for message in messages {
            context.sys()?.router.send(message);
        }

         */

        Ok(())
    }
}

impl <'c> IntakeContext for Nucleus<'c> {
    fn head(&self) -> i64 {
        self.head.cycle
    }

    fn phase(&self) -> u8 {
        0
    }
}

struct NucleusCycle<'cycle,'nucleus> {
    nucleus: &'cycle Nucleus<'nucleus>,
    id: Id,
    state: NucleusPhasicStateStructure,
    messaging: NucleusPhasicMessagingStructure,
    outbound: OutboundMessaging,
    context: NucleusCycleContext<'cycle,'nucleus>,
    phase: u8
}

impl<'cycle,'nucleus> NucleusCycle<'cycle,'nucleus> {
    fn init<'error>(
        id: Id,
        context: NucleusCycleContext<'cycle,'nucleus>,
        nucleus: &'cycle Nucleus<'nucleus>,
    ) -> Result<Self, Box<dyn Error+'error>> {
        Ok(NucleusCycle {
            nucleus: nucleus,
            id: id,
            state: NucleusPhasicStateStructure::new(),
            messaging: NucleusPhasicMessagingStructure::new(),
            outbound: OutboundMessaging::new(),
            context: context,
            phase: 0,
        })
    }

    fn intake_message(&mut self, message: Arc<Message>) -> Result<(), Box<dyn Error>> {
        self.messaging.intake(message)?;
        Ok(())
    }

    fn intake_state(&mut self, key: TronKey, state: &ReadOnlyState) -> Result<(), Box<dyn Error>> {
        self.state.intake(key, state)?;
        Ok(())
    }

    fn step(&mut self) -> Result<(), Box<dyn Error>> {
        let phase: u8 = 0;
        match self.messaging.remove(&phase)? {
            None => {}
            Some(messages) => {
                for message in messages {
                    self.process(message.as_ref())?;
                }
            }
        }
        Ok(())
    }

    fn commit(&mut self) -> (Vec<(StateKey, Rc<State>)>, Vec<Message>) {
        (
            self.state.drain(self.context.revision()),
            self.outbound.drain(),
        )
    }

    fn tron(
        &mut self,
        key: &TronKey,
    ) -> Result<(Arc<TronConfig>, Rc<State>, TronShell, TronContext), Box<dyn Error+'static >> {
        /*
        let mut state = self.state.get(key)?;
        let artifact = state.get_artifact()?;
        let tron_config = self
            .context
            .sys()
            .local
            .configs
            .tron_config_keeper
            .get(&artifact).unwrap();
        let tron_shell = TronShell::new(init_tron(&tron_config)?);

        let context = TronContext::new(
            self.nucleus,
            self.context.sys()?,
            key.clone(),
            Revision {
                cycle: self.context.revision.cycle,
            },
            tron_config.clone(),
            self.context.timestamp.clone(),
        );

        Ok((tron_config, state, tron_shell, context))
         */
        unimplemented!()
    }

    fn process(&mut self, message: &Message) -> Result<(), Box<dyn Error>> {
        match message.kind {
            MessageKind::Create => self.process_create(message),
            MessageKind::Update => self.process_update(message),
            _ => Err("not implemented yet".into()),
        }
    }

    fn process_create(&mut self, message: &Message) -> Result<(), Box<dyn Error>> {
        /*
        // ensure this is addressed to a neutron
        if !Neutron::valid_neutron_id(message.to.tron.tron.clone()) {
            return Err(format!(
                "not a valid neutron id: {:?}",
                message.to.tron.tron.id.clone()
            )
            .into());
        }

        let neutron_state_key = StateKey {
            tron: message.to.tron.clone(),
            revision: Revision {
                cycle: self.context.revision.cycle,
            },
        };

        let mut neutron_state = self.state.get(&neutron_state_key.tron)?;

        let neutron_config = self
            .context
            .sys()?
            .local
            .configs
            .tron_config_keeper
            .get(&CORE_NEUTRON_CONFIG)?;

        let context = TronContext::new(
            self.nucleus,
            self.context.sys()?,
            message.to.tron.clone(),
            Revision {
                cycle: self.context.revision.cycle,
            },
            neutron_config,
            self.context.timestamp.clone(),
        );

        let neutron = Neutron {};
        neutron.create_tron(context, neutron_state, message);

        Ok(())
         */
        unimplemented!()
    }

    fn process_update(&mut self, message: &Message) -> Result<(), Box<dyn Error>> {
        let state_key = StateKey {
            tron: message.to.tron.clone(),
            revision: Revision {
                cycle: self.context.revision.cycle,
            },
        };
        let (config, state, tron, context) = self.tron(&message.to.tron)?;
        Ok(())
    }
}

impl<'cycle,'nucleus> MessageRouter for NucleusCycle<'cycle,'nucleus> {
    fn send(&mut self, message: Message) {
        if !&message.to.tron.nucleus.eq(&self.id) {
            self.outbound.push(message);
            return;
        }

        match message.to.cycle {
            Cycle::Exact(cycle) => {
                if self.context.revision.cycle != cycle {
                    self.outbound.push(message);
                    return;
                }
            }
            Cycle::Present => {}
            Cycle::Next => {
                self.outbound.push(message);
                return;
            }
        }

        match &message.to.delivery {
            DeliveryMoment::Cyclic => {
                self.outbound.push(message);
                return;
            }
            DeliveryMoment::Phasic => {
                if message.to.phase >= self.phase {
                    self.outbound.push(message);
                    return;
                } else {
                    self.intake_message(Arc::new(message));
                    return;
                }
            }
        }
    }
}

struct NucleusCycleContext<'context,'nucleus> {
    revision: Revision,
    timestamp: u64,
    phase: String,
    nucleus: &'context Nucleus<'nucleus>,
}

impl <'context,'nucleus> NucleusCycleContext<'context,'nucleus> {
    pub fn configs<'get>(&'get self) -> &'get Configs<'context> {
        unimplemented!();
    }

    pub fn revision(&self) -> &Revision {
        &self.revision
    }

    pub fn timestamp(&self) -> u64 {
        self.timestamp
    }

    pub fn phase(&self) -> &str {
        self.phase.as_str()
    }

    pub fn sys(&self) -> Arc<Mechtronium<'context>>{
        unimplemented!()
    }
}

