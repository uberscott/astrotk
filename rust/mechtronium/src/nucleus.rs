use std::{fmt, io};
use std::borrow::{Borrow, BorrowMut};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter};
use std::rc::Rc;
use std::sync::{Arc, Mutex, PoisonError, RwLock, Weak, MutexGuard};
use std::time::{Instant, SystemTime};

use no_proto::error::NP_Error;
use no_proto::memory::NP_Memory_Owned;

use mechtron_common::artifact::Artifact;
use mechtron_common::configs::{BindConfig, Configs, Keeper, MechtronConfig, NucleusConfig, SimConfig, SimSpark};
use mechtron_common::core::*;
use mechtron_common::id::{Id, IdSeq, StateKey};
use mechtron_common::id::MechtronKey;
use mechtron_common::id::Revision;
use mechtron_common::message::{Cycle, DeliveryMoment, MechtronLayer, Message, MessageBuilder, MessageKind, Payload, To};
use mechtron_common::state::{ReadOnlyState, ReadOnlyStateMeta, State, StateMeta};

use crate::cache::Cache;
use crate::error::Error;
use crate::shell::{MechtronShell};
use crate::star::{Local, Star, WasmStuff};
use crate::nucleus::message::{CycleMessagingContext, CyclicMessagingStructure, OutboundMessaging, PhasicMessagingStructure};
use crate::nucleus::state::{Lookup, MechtronKernels, StateHistory};
use crate::router::{HasNucleus, InternalRouter};
use crate::mechtron::CreatePayloadsBuilder;
use crate::membrane::{WasmMembrane, MechtronMembrane};
use mechtron_common::logger::log;
use crate::network::Route;

pub trait NucleiContainer
{
    fn has_nucleus(&self, id: &Id) -> bool;
}

pub struct Nuclei {
    nuclei: RwLock<HashMap<Id, Arc<Nucleus>>>,
    context: NucleusContext
}

impl Nuclei {
    pub fn new(cache: Arc<Cache>,
               seq: Arc<IdSeq>,
               router: Arc<dyn Route>) -> Arc<Self> {
        let rtn = Arc::new(Nuclei {
            nuclei: RwLock::new(HashMap::new()),
            context: NucleusContext {
                cache: cache,
                seq: seq,
                router: router,
                nuclei: RefCell::new(Option::None),
            },
        });
        rtn.set_arc_ref_to_self(rtn.clone());
        rtn
    }

    pub fn set_arc_ref_to_self(&self, myself: Arc<Nuclei>)
    {
        self.context.nuclei.replace(Option::Some(Arc::downgrade(&myself)));
    }


    pub fn has_nucleus(&self, id: &Id) -> bool {
        let nuclei = self.nuclei.read();
        if nuclei.is_err()
        {
            return false;
        }
        let nuclei = nuclei.unwrap();
        nuclei.contains_key(id)
    }

    pub fn get<'get>(&'get self, nucleus_id: &Id) -> Result<Arc<Nucleus>, Error> {
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

    pub fn create_sim(&self, sim_config: Arc<SimConfig>, sim_id: Id) -> Result<Id, Error> {
        let nucleus = Nucleus::create_sim(sim_config, self.context.clone(), sim_id )?;

        if nucleus.is_panic()
        {
            return Err(nucleus.panic_message().unwrap().into())
        }

        let id = nucleus.info.id.clone();
        self.add(nucleus)?;
        Ok(id)
    }

    pub fn create(&self, sim_id: Id, lookup_name: Option<String>, config: Arc<NucleusConfig> ) -> Result<Id, Error> {
        let id = self.context.seq.next();
        let mut nucleus = Nucleus::new(id, sim_id, self.context.clone(), config.clone())?;

        nucleus.bootstrap(HashMap::new())?;

        if nucleus.is_panic()
        {
            return Err(nucleus.panic_message().unwrap().into())
        }

        self.add(nucleus)?;
        Ok(id)
    }

    pub fn add( &self, nucleus: Nucleus )->Result<(),Error>
    {
        let mut sources = self.nuclei.write()?;
        sources.insert(nucleus.info.id.clone(),Arc::new(nucleus) );
        Ok(())
    }
}




fn timestamp() -> u64 {
    let since_the_epoch = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH);
    if since_the_epoch.is_err()
    {
        return 0;
    }
    let since_the_epoch = since_the_epoch.unwrap();
    let timestamp =
        since_the_epoch.as_secs() * 1000 + since_the_epoch.subsec_nanos() as u64 / 1_000_000;

    return timestamp;
}

#[derive(Clone)]
pub struct NucleusInfo
{
    pub id : Id,
    pub sim_id: Id
}


#[derive(Clone)]
pub struct NucleusContext
{
    pub cache: Arc<Cache>,
    pub seq: Arc<IdSeq>,
    pub router: Arc<dyn Route>,
    pub nuclei: RefCell<Option<Weak<Nuclei>>>
}



impl  NucleusContext
{

    pub fn info_for(
        &self,
        key: MechtronKey,
        artifact: &Artifact,
    ) -> Result<TronInfo, Error> {
        let config = self.cache.configs.mechtrons.get(&artifact)?;
        let bind = self.cache.configs.binds.get(&config.bind.artifact)?;

        Ok(TronInfo {
            key: key,
            config: config,
            bind: bind
        })
    }


    pub fn new_kernel_from_state(&self, state: Arc<ReadOnlyState>) ->Result<Arc<Mutex<MechtronMembrane>>,Error>{
self.cache.wasms.cache(&state.config.wasm.artifact)?;
       let wasm_membrane = self.cache.wasms.get_membrane(&state.config.wasm.artifact)?;
       let kernel = Arc::new( Mutex::new(MechtronMembrane::new( wasm_membrane, state )));
       Ok(kernel)
    }


    pub fn new_kernel(&self, config: &Arc<MechtronConfig>) ->Result<Arc<Mutex<MechtronMembrane>>,Error>{
        let wasm_membrane = self.cache.wasms.get_membrane(&config.wasm.artifact)?;
        let state = State::new(&self.cache.configs, config.clone() )?;
        let state = state.read_only()?;
        let state = Arc::new(state);
        let kernel = Arc::new( Mutex::new(MechtronMembrane::new( wasm_membrane, state )));
        Ok(kernel)
    }

    pub fn get_nuclei(&self) -> Option<Arc<Nuclei>>
    {
        // we unwrap it because if it hasn't been set then that's a panic error,
        // whereas if the reference is gone, that just means the system is probably in the
        // state of shutting down
        let option = &self.nuclei.borrow();
        let option = option.as_ref().unwrap();
        return option.upgrade();
    }
}

pub struct Nucleus {
    info: NucleusInfo,
    state: Arc<StateHistory>,
    messaging: CyclicMessagingStructure,
    head: Revision,
    context: NucleusContext,
    config: Arc<NucleusConfig>,
    panic: RefCell<Option<String>>,
}


impl Nucleus {}

impl Nucleus {
    fn create_sim(
        sim_config: Arc<SimConfig>,
        context: NucleusContext,
        sim_id : Id
    ) -> Result<Self, Error> {
//        let sim_id = context.seq.next();
        let id = sim_id.clone();

        let config = context.cache.configs.nucleus.get(&CORE_NUCLEUS_SIMULATION)?;

        let nucleus = Nucleus::new( id, sim_id, context, config )?;

        let mut bootstrap_meta = HashMap::new();
        bootstrap_meta.insert( "sim_config".to_string(), sim_config.source.to().clone() );

        nucleus.bootstrap(bootstrap_meta.clone())?;

        Ok(nucleus)
    }

    fn new(
        id: Id,
        sim_id: Id,
        context: NucleusContext,
        config: Arc<NucleusConfig>
    ) -> Result<Self, Error> {
        let mut nucleus = Nucleus {
            info: NucleusInfo {
                id: id,
                sim_id: sim_id,
            },
            state: Arc::new(StateHistory::new()),
            messaging: CyclicMessagingStructure::new(),
            head: Revision { cycle: -1 },
            context: context,
            config: config,
            panic: RefCell::new(Option::None),
        };

        Ok(nucleus)
    }

    pub fn valid_neutron_id(id: Id) -> bool {
        return id.id == 0;
    }


    fn neutron_key(&self) -> MechtronKey
    {
        MechtronKey::new(self.info.id.clone(), Id::new(self.info.id.seq_id, 0))
    }

    pub fn is_panic(&self) -> bool
    {
        self.panic.borrow().is_some()
    }

    pub fn panic_message(&self) -> Option<String>
    {
        self.panic.borrow().clone()
    }

    pub fn intake_message(&self, message: Arc<Message>) {
        // check for extra cyclic
        if message.to.delivery == DeliveryMoment::ExtraCyclic
        {
            self.handle_extracyclic(message);
        } else {
            self.messaging.intake(message, CycleMessagingContext { head: self.head.cycle });
        }
    }

    pub fn handle_extracyclic(&self, message: Arc<Message>) {
        let state_key = StateKey {
            tron: message.to.tron,
            revision: Revision {
                cycle: match message.to.cycle {
                    Cycle::Exact(cycle) => cycle,
                    Cycle::Present => self.head.cycle,
                    Cycle::Next => self.head.cycle + 1
                }
            },
        };

        /*
        let kernel = self.kernels.get(state_key.tron.clone())?;
        let shell = MechtronShell::new(kernel.lock()?, state_key.tron.clone(), &self.context.cache.configs );

        let result = self.shell(&state_key);
        if result.is_err()
        {
            let result = message.reject(mechtron_common::message::From { tron: self.neutron_key(), cycle: self.head.cycle, timestamp: timestamp(), layer: MechtronLayer::Shell }, "state_key had no content", self.context.seq.clone(), &self.context.cache.configs);
            if (result.is_err())
            {
                println!("could not access this tron.");
                return;
            }
            println!("sending reject message because could not find .");
            self.context.router.send(Arc::new(result.unwrap()));
            return;
        }
        let mut shell = result.unwrap();

        shell.extra(message.as_ref(),self);

        if !shell.is_panic()
        {
            self.handle_outbound(shell.flush());
        } else {
            println!("experienced shell panic!");
        }
         */
        unimplemented!()
    }



    fn handle_outbound( &self, messages: Vec<Message>)
    {
                for message in messages
                {
println!("Sending message of type: {:?} ", &message.kind );
                    self.context.router.relay(Arc::new(message));
                }
    }


    fn bootstrap(&self, meta: HashMap<String,String> ) -> Result<(), Error> {
        let mut states = vec!();
        let mut messages = vec!();
        {
            let mut nucleus_cycle = NucleusCycle::init(self.info.clone(), self.context.clone(), self.head.clone(), self.state.clone(), self.config.clone())?;

            nucleus_cycle.bootstrap(meta)?;

            if nucleus_cycle.is_panic()
            {
                let panic_message = nucleus_cycle.panic.borrow().clone().unwrap();
                self.panic(panic_message.clone());
                return Err(panic_message.into())
            }

            let (tmp_states, tmp_messages) = nucleus_cycle.commit()?;
            for state in tmp_states
            {
                states.push(state);
            }
            for message in tmp_messages
            {
                messages.push(message);
            }
        }

        for (key, state) in states {
                self.state.intake(state, key)?;
        }

        for message in messages {
            let router = self.context.router.clone();
            router.relay(message);
        }

//        self.head.cycle = self.head.cycle+1;
        Ok(())
    }


    pub fn revise(&mut self, from: Revision, to: Revision) -> Result<(), Error> {
        if from.cycle != to.cycle - 1 {
            return Err("cycles must be sequential. 'from' revision cycle must be exactly 1 less than 'to' revision cycle".into());
        }

        let mut states = vec!();
        let mut messages= vec!();
        {

            let mut nucleus_cycle = NucleusCycle::init(self.info.clone(), self.context.clone(), to.clone(), self.state.clone(), self.config.clone() )?;

            for message in self.messaging.query(to.cycle)? {
                nucleus_cycle.intake_message(message)?;
            }

            match self.state.query(to.clone())? {
                None => {}
                Some(results) => {
                    for (key, state) in results {
                        nucleus_cycle.intake_state(key, state)?;
                    }
                }
            }

            nucleus_cycle.execute();

            let (tmp_states, tmp_messages) = nucleus_cycle.commit()?;
            for state in tmp_states
            {
                states.push(state);
            }

            for message in tmp_messages
            {
                messages.push(message);
            }
        }

        self.head = to;

        for (key, state) in states {
            self.state.intake(state, key)?;
        }

        for message in messages {
            let router = &mut self.context.router;
            router.relay(message);
        }

        Ok(())
    }
}






impl MechtronShellContext for Nucleus
{
    fn configs<'get>(&'get self) -> &'get Configs {
        &self.context.cache.configs
    }

    fn get_state(&self, key: &StateKey) -> Result<Arc<ReadOnlyState>, Error> {
        self.state.get(key)
    }

    fn lookup_nucleus(&self, name: &str) -> Result<Id, Error> {
        let lookup = Lookup::new(self.state.clone(), self.head.clone());
        lookup.lookup_nucleus(self.info.id.clone(), name)
    }
    fn lookup_mechtron(&self, nucleus_id: &Id, name: &str) -> Result<MechtronKey, Error> {
        let lookup = Lookup::new(self.state.clone(), self.head.clone());
        lookup.lookup_tron(nucleus_id, name)
    }

    fn revision(&self) -> &Revision {
        &self.head
    }

    fn timestamp(&self) -> u64 {
        timestamp()
    }

    fn seq(&self) -> Arc<IdSeq> {
        self.context.seq.clone()
    }

    fn neutron_api_create(&self, state: ReadOnlyState, create: Message)  {

/*        let config = self.configs().binds.get(&config)?;
        let tron = init_tron(&config)?;

        let info = TronInfo {
            config: config,
            key: key
        };

        let mut tron = MechtronShell::new(tron, info);
        let result = {
            tron.create(create,self, &mut state)
        };

        match result
        {
            Ok(_) => {
                self.state.add(key, Arc::new(Mutex::new(state)));
                Ok(())
            }
            Err(e) => {
                Result::Err(e.into())
            }
        }

 */
        unimplemented!()
    }

    fn create_api_create_nucleus(&self, nucleus_config: Arc<NucleusConfig>) -> Result<Id,Error>{
        unimplemented!()
    }

    fn panic(&self, error: String) {
        if !self.is_panic()
        {
            self.panic.replace(Option::Some(error));
        }
    }

    fn phase(&self) -> String {
        "default".to_string()
    }
}

struct NucleusData<'cycle>
{
    state: &'cycle StateHistory,
    messaging: &'cycle CyclicMessagingStructure,
}

struct NucleusCycle {
    info: NucleusInfo,
    kernels: MechtronKernels,
    messaging: PhasicMessagingStructure,
    outbound: OutboundMessaging,
    revision: Revision,
    context: NucleusContext,
    config: Arc<NucleusConfig>,
    history: Arc<StateHistory>,
    phase: String,
    panic: RefCell<Option<String>>,
}

impl NucleusCycle {
    fn init(
        info: NucleusInfo,
        context: NucleusContext,
        revision: Revision,
        history: Arc<StateHistory>,
        config: Arc<NucleusConfig>,

    ) -> Result<Self, Error> {
        Ok(NucleusCycle {
            context: context,
            info: info,
            kernels: MechtronKernels::new(),
            messaging: PhasicMessagingStructure::new(),
            outbound: OutboundMessaging::new(),
            revision: revision,
            history: history,
            config: config,
            phase: "default".to_string(),
            panic: RefCell::new(Option::None),
        })
    }

    fn panic(&self, error: Error)
    {
        if !self.is_panic()
        {
            self.panic.replace(Option::Some(format!("nucleus cycle panic! {:?}", error)));
        }
    }

    fn is_panic(&self) -> bool
    {
        self.panic.borrow().is_some()
    }

    fn bootstrap(&mut self, meta: HashMap<String, String>) -> Result<(), Error> {
        let mut seq = self.context.seq.clone();

        let timestamp = timestamp();

        let neutron_key = MechtronKey::new(self.info.id.clone(), Id::new(self.info.id.seq_id, 0));

        let config = self.configs().mechtrons.get(&CORE_MECHTRON_NEUTRON)?;
        let bind = self.context.cache.configs.binds.get(&config.bind.artifact)?;
        let neutron_info = TronInfo {
            config: config.clone(),
            key: neutron_key.clone(),
            bind: bind,
        };

        // first we create a neutron for the simulation nucleus

        let mut neutron_create_payload_builder =
            CreatePayloadsBuilder::new(self.configs(), neutron_info.config.clone())?;

        let create = Message::longform(
            seq,
            MessageKind::Create,
            mechtron_common::message::From {
                tron: neutron_info.key.clone(),
                cycle: -1,
                timestamp,
                layer: MechtronLayer::Shell
            },
            To {
                tron: neutron_info.key.clone(),
                port: "create".to_string(),
                cycle: Cycle::Present,
                phase: "default".to_string(),
                delivery: DeliveryMoment::Phasic,
                layer: MechtronLayer::Kernel,
            },
            Option::None,
            CreatePayloadsBuilder::payloads(self.configs(), neutron_create_payload_builder),
            Option::Some(meta.clone()),
            Option::None
        );

        let mut neutron_state = State::new(self.configs(), neutron_info.config.clone())?;


        neutron_state.set_creation_cycle(-1);
        neutron_state.set_creation_timestamp(0);
        neutron_state.set_taint(false);
        let neutron_config = self.configs().binds.get(&CORE_BIND_NEUTRON)?;

        let neutron_state = neutron_state.read_only()?;
        self.context.cache.wasms.cache(&config.wasm.artifact)?;

        let wasm_membrane = self.context.cache.wasms.get_membrane(&config.wasm.artifact)?;
        let kernel = MechtronMembrane::new( wasm_membrane, Arc::new(neutron_state) );
        // insert the neutron_state into the MechtronKernels
        self.kernels.intake(neutron_info.key.clone(), kernel);


        let neutron_kernel = self.kernels.get( &neutron_info.key )?;
        let mut neutron_shell = MechtronShell::new(neutron_kernel.lock()?, neutron_info.key.clone(), &self.context.cache.configs )?;
        neutron_shell.create(&create, self);

        // now we CREATE each named mechtron in the neutron_config
        {
            let mut messages = vec!();
            for mechtron_config_ref in self.config.mectrons.as_slice()
            {
                let mechtron_config = self.context.cache.configs.mechtrons.get(&mechtron_config_ref.artifact)?;

                let mut create_payloads_builder = CreatePayloadsBuilder::new(self.configs(), mechtron_config.clone())?;
                create_payloads_builder.set_config(mechtron_config.as_ref());
                let message = Message::longform(
                    self.context.seq.clone(),
                    MessageKind::Create,
                    mechtron_common::message::From {
                        tron: neutron_info.key.clone(),
                        cycle: -1,
                        timestamp,
                        layer: MechtronLayer::Shell,
                    },
                    To {
                        tron: neutron_info.key.clone(),
                        port: "create".to_string(),
                        cycle: Cycle::Present,
                        phase: "default".to_string(),
                        delivery: DeliveryMoment::Phasic,
                        layer: MechtronLayer::Kernel,
                    },
                    Option::None,
                    CreatePayloadsBuilder::payloads(self.configs(), create_payloads_builder),
                    Option::Some(meta.clone()),
                    Option::None
                );

                messages.push(Arc::new(message));
            }


            // send all create messages to neutron
            {
                neutron_shell.messages(messages, self);
                {
                    if neutron_shell.is_tainted()?
                    {
                        return Err("bootstrap failed.  neutron is tainted".into());
                    }
                }
            }

        }

        Ok(())
    }

    fn execute(&mut self) -> Result<(), Error> {
        let config = self.config.clone();
        for phase in &config.phases
        {
            let phase = phase.name.clone();
            match &self.messaging.drain(&phase)? {
                None => {}
                Some(messages) => {
                    for message in messages {
                        self.process(message.as_ref())?;
                    }
                }
            }
        }
        Ok(())
    }


    fn commit(&mut self) -> Result<(Vec<(StateKey, Arc<ReadOnlyState>)>, Vec<Arc<Message>>),Error> {
        let states = self.kernels.drain(&self.revision.clone())?;
        let messages = self.messaging.drain_all()?;
        Ok((states,messages))
    }

    fn intake_message(&mut self, message: Arc<Message>) -> Result<(), Error> {
        self.messaging.intake(message)?;
        Ok(())
    }

    fn intake_state(&mut self, key: MechtronKey, state: Arc<ReadOnlyState>)->Result<(),Error> {

        let wasm_membrane = self.context.cache.wasms.get_membrane(&state.config.wasm.artifact)?;
        let mechtron_membrane = MechtronMembrane::new(wasm_membrane,state);

        match self.kernels.intake(key, mechtron_membrane)
        {
            Ok(_) => {}
            Err(e) => self.panic(e)
        }
        Ok(())
    }

    /*
    fn tron(
        &mut self,
        key: &MechtronKey,
    ) -> Result<(MechtronShell, Arc<Mutex<State>>), Error> {
        let mut state = self.state.get(key)?;
        let artifact = {
            let state = state.lock()?;
            state.get_mechtron_config_artifact()?
        };
        let tron_config = self.context.cache.configs.binds.get(&artifact)?;

        let info = TronInfo {
            key: key.clone(),
            config: tron_config.clone(),
        };

        let tron_shell = MechtronShell::new(init_tron(&tron_config)?, info );

        Ok((tron_shell,state))
    }

     */

    fn process(&mut self, message: &Message) -> Result<(), Error> {
        match message.kind {
            MessageKind::Create => self.process_create(message),
            MessageKind::Update => self.process_update(message),
            _ => Err("not implemented yet".into()),
        }
    }

    fn process_create(&mut self, message: &Message) -> Result<(), Error> {
        // ensure this is addressed to a neutron
        if !Nucleus::valid_neutron_id(message.to.tron.mechtron.clone()) {
            return Err(format!(
                "not a valid neutron id: {:?}",
                message.to.tron.mechtron.id.clone()
            )
            .into());
        }

        let neutron_state_key = StateKey {
            tron: message.to.tron.clone(),
            revision: Revision {
                cycle: self.revision.cycle,
            },
        };

        let mut neutron_state = self.kernels.get(&neutron_state_key.tron)?;

        let info = self.context.info_for(message.to.tron.clone(), &CORE_BIND_NEUTRON)?;

        Ok(())
    }

    fn process_update(&mut self, message: &Message) -> Result<(), Error> {
        let state_key = StateKey {
            tron: message.to.tron.clone(),
            revision: Revision {
                cycle: self.revision.cycle,
            },
        };
//        let tron = self.tron(&message.to.tron)?;
        Ok(())
    }
}

pub trait MechtronShellContext
{
    fn configs<'get>(&'get self) -> &'get Configs;
    fn revision(&self) -> &Revision;
    fn get_state(&self, key: &StateKey) -> Result<Arc<ReadOnlyState>, Error>;
    fn lookup_nucleus(&self, name: &str) -> Result<Id, Error>;
    fn lookup_mechtron(&self, nucleus_id: &Id, name: &str) -> Result<MechtronKey, Error>;
    fn timestamp(&self) -> u64;
    fn seq(&self) -> Arc<IdSeq>;
    fn panic(&self, error: String);

    fn neutron_api_create(&self, state: ReadOnlyState, create: Message);
    fn create_api_create_nucleus(&self, nucleus_config: Arc<NucleusConfig>) -> Result<Id, Error>;
    fn phase(&self)->String;
}






impl MechtronShellContext for NucleusCycle
{

    fn configs<'get>(&'get self) -> &'get Configs {
        self.context.cache.configs.borrow()
    }


    fn revision(&self) -> &Revision {
        &self.revision
    }

    fn get_state(&self, key: &StateKey) -> Result<Arc<ReadOnlyState>, Error> {
        if key.revision.cycle >= self.revision.cycle {
            return Err(format!("attempted to read the state of tron {:?} in a present or future cycle, which is not allowed", key).into());
        }
        let state = &self.history;
        let state = state.read_only(key)?;
        Ok(state)
    }

    fn lookup_nucleus(&self, name: &str) -> Result<Id, Error> {
        let nucleus_id = self.info.id.clone();
        let lookup = Lookup::new( self.history.clone(), self.revision.clone() );
        lookup.lookup_nucleus(nucleus_id,name)
    }


    fn lookup_mechtron(
        &self,
        nucleus_id: &Id,
        name: &str,
    ) -> Result<MechtronKey, Error> {
        let lookup = Lookup::new(self.history.clone(), self.revision.clone());
        lookup.lookup_tron(nucleus_id, name)
    }

    fn timestamp(&self) -> u64 {
        0
    }

    fn seq(&self) -> Arc<IdSeq> {
        self.context.seq.clone()
    }

    fn panic(&self, error: String) {
        if self.panic.borrow().is_none()
        {
            self.panic.replace(Option::Some(error));
        }
    }

    fn neutron_api_create(&self, mut state: ReadOnlyState, create_message: Message)
    {
        let config = state.config.clone();
        let mechtron_id = state.get_mechtron_id();

        match mechtron_id {
            Err(e) => { self.panic(e.into()) }
            Ok(mechtron_id) => {
                match self.context.new_kernel_from_state(Arc::new(state))
                {
                    Err(e) => {
                        self.panic(e);
                    }
                    Ok(kernel) => {
                        let key = MechtronKey::new(self.info.id, mechtron_id);

                        let kernel = match kernel.lock()
                        {
                            Ok(kernel) => {kernel}
                            Err(e) => {
                                self.panic(e.into() );
                                return ()
                            }
                        };

                        match MechtronShell::new(kernel, key.clone(), &self.context.cache.configs )
                        {
                            Ok(mut shell) => {
                                shell.create(&create_message, self);
                            }
                            Err(e) => {
                                println!("{:?}",e);
                            }
                        }
                    }
                }
            }
        }
    }

    fn create_api_create_nucleus(&self, nucleus_config: Arc<NucleusConfig>) -> Result<Id, Error> {
        match self.context.get_nuclei()
        {
            None => {
                self.panic("could not get nuclei".into());
                Err("could not get nuclei".into())
            }
            Some(nuclei) => {
                match nuclei.create(self.info.sim_id.clone(), Option::None, nucleus_config)
                {
                    Ok(id) => {
                        Ok(id)
                    }
                    Err(e) => {
                        self.panic(e.clone());
                        Err(e)
                    }
                }
            }
        }
    }

    fn phase(&self) -> String {
        "default".to_string()
    }
}




impl InternalRouter for NucleusCycle {
    fn send(&self, message: Arc<Message>) {
        unimplemented!()
        /*
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

         */
    }

    fn receive(&self, message: Arc<Message>) {
        unimplemented!()
    }

    fn has_nucleus_local(&self, nucleus: &Id) -> HasNucleus {
        unimplemented!()
    }

}


mod message
{
    use std::collections::HashMap;
    use std::sync::{Arc, RwLock};
    use std::time::Instant;

    use mechtron_common::error::Error;
    use mechtron_common::id::MechtronKey;
    use mechtron_common::message::{Cycle, DeliveryMoment, Message};

    use crate::nucleus::Nucleus;

    pub struct CyclicMessagingStructure {
        store: RwLock<HashMap<i64, HashMap<MechtronKey, TronMessageChamber>>>,
    }

    impl CyclicMessagingStructure {
        pub fn new() -> Self {
            CyclicMessagingStructure {
                store: RwLock::new(HashMap::new()),
            }
        }

        pub fn intake(
            &self,
            message: Arc<Message>,
            context: CycleMessagingContext
        ) -> Result<(), Error> {
            let delivery = MessageDelivery::new(message, context.clone());

            let mut store = self.store.write()?;

            let desired_cycle = match delivery.message.to.cycle {
                Cycle::Exact(cycle) => cycle,
                Cycle::Present => {
                    // Nucleus intake is InterCyclic therefore cannot accept present cycles
                    context.clone().head.clone() + 1
                }
                Cycle::Next => context.clone().head.clone() + 1
            };

            // at some point we must determine if the nucleus policy allows for message deliveries to this
            // nucleus after x number of cycles and then send a rejection message if needed

            if !store.contains_key(&desired_cycle) {
                store.insert(desired_cycle.clone(), HashMap::new());
            }

            let mut store = store.get_mut(&desired_cycle).unwrap();

            if !store.contains_key(&delivery.message.to.tron) {
                store.insert(
                    delivery.message.to.tron.clone(),
                    TronMessageChamber::new(),
                );
            }

            let mut chamber = store.get_mut(&delivery.message.to.tron).unwrap();
            chamber.intake(delivery)?;

            Ok(())
        }

        pub fn query(&self, cycle: i64) -> Result<Vec<Arc<Message>>, Error> {
            let store = self.store.read()?;
            match store.get(&cycle) {
                None => Ok(vec![]),
                Some(chambers) => {
                    let mut rtn = vec![];
                    for chamber in chambers.values() {
                        rtn.append(&mut chamber.messages());
                    }
                    Ok(rtn)
                }
            }
        }

        pub fn release(&mut self, before_cycle: i64) -> Result<usize, Error>
        {
            let mut releases = vec!();

            {
                let store = self.store.read()?;

                for cycle in store.keys()
                {
                    if cycle < &before_cycle
                    {
                        releases.push(cycle.clone());
                    }
                }
            }

            if releases.is_empty()
            {
                return Ok(0);
            }

            let mut store = self.store.write()?;
            for cycle in &releases
            {
                store.remove(cycle);
            }

            Ok(releases.len())
        }
    }

    pub struct TronMessageChamber {
        deliveries: Vec<MessageDelivery>,
    }

    impl TronMessageChamber {
        pub fn new() -> Self {
            TronMessageChamber { deliveries: vec![] }
        }

        pub fn messages(&self) -> Vec<Arc<Message>> {
            self.deliveries.iter().map(|d| d.message.clone()).collect()
        }

        pub fn intake(&mut self, delivery: MessageDelivery) -> Result<(), Error> {
            self.deliveries.push(delivery);
            Ok(())
        }
    }

    pub struct PhasicMessagingStructure {
        store: HashMap<String, Vec<Arc<Message>>>,
    }

    impl PhasicMessagingStructure {
        pub fn new() -> Self {
            PhasicMessagingStructure {
                store: HashMap::new(),
            }
        }

        pub fn intake(&mut self, message: Arc<Message>) -> Result<(), Error> {
            if !self.store.contains_key(&message.to.phase) {
                self.store.insert(message.to.phase.clone(), vec![]);
            }
            let mut messages = self.store.get_mut(&message.to.phase).unwrap();
            messages.push(message);

            Ok(())
        }


        pub fn drain(&mut self, phase: &String) -> Result<Option<Vec<Arc<Message>>>, Error> {
            Ok(self.store.remove(phase))
        }
        pub fn drain_all(&mut self ) -> Result<Vec<Arc<Message>>, Error> {
            let mut rtn = vec!();
            for (key,mut message) in self.store.drain()
            {
                rtn.append( & mut message );
            }
            Ok(rtn)
        }

    }



    #[derive(Clone)]
    pub struct CycleMessagingContext {
        pub head: i64
    }

    pub struct OutboundMessaging {
        queue: Vec<Message>,
    }

    impl OutboundMessaging {
        pub fn new() -> Self {
            OutboundMessaging { queue: vec![] }
        }

        pub fn drain(&mut self) -> Vec<Message> {
            let mut rtn = vec![];

            while let Some(message) = self.queue.pop() {
                rtn.push(message)
            }

            return rtn;
        }

        pub fn push(&mut self, message: Message) {
            self.queue.push(message);
        }
    }

    pub struct MessageDelivery {
        received: Instant,
        cycle: i64,
        message: Arc<Message>,
    }

    impl MessageDelivery {
        fn new(message: Arc<Message>, context: CycleMessagingContext) -> Self {
            MessageDelivery {
                received: Instant::now(),
                cycle: context.head.clone(),
                message: message,
            }
        }
    }


    #[cfg(test)]
    mod tests {
        use std::sync::Arc;

        use mechtron_common::buffers::Buffer;
        use mechtron_common::configs::Configs;
        use mechtron_common::core::*;
        use mechtron_common::id::{Id, IdSeq, MechtronKey};
        use mechtron_common::message::{Cycle, DeliveryMoment, MechtronLayer, Message, MessageBuilder, MessageKind, Payload, To};
        use mechtron_common::message::DeliveryMoment::Cyclic;

        use crate::nucleus::message::{CycleMessagingContext, CyclicMessagingStructure, PhasicMessagingStructure};
        use crate::test::*;

        fn message(configs: &mut Configs) -> Message {
            let mut seq = IdSeq::new(0);

            configs.schemas.cache(&CORE_SCHEMA_EMPTY).unwrap();
//        configs.buffer_factory_keeper.cache(&CORE_SCHEMA_NUCLEUS_LOOKUP_NAME_MESSAGE).unwrap();
            let factory = configs.schemas.get(&CORE_SCHEMA_EMPTY).unwrap();
            let buffer = factory.new_buffer(Option::None);
            let buffer = Buffer::new(buffer);
            let buffer = buffer.read_only();
            let payload = Payload {
                buffer: buffer,
                schema: CORE_SCHEMA_META_CREATE.clone(),
            };


            let seq_borrow = &Arc::new(seq);

            Message::single_payload(seq_borrow.clone(),
                                    MessageKind::Update,
                                    mechtron_common::message::From {
                                        tron: MechtronKey::new(seq_borrow.next(), seq_borrow.next()),
                                        cycle: 0,
                                        timestamp: 0,
                                        layer: MechtronLayer::Kernel
                                    },
                                    To {
                                        tron: MechtronKey::new(seq_borrow.next(), seq_borrow.next()),
                                        port: "someport".to_string(),
                                        cycle: Cycle::Present,
                                        phase: "default".to_string(),
                                        delivery: DeliveryMoment::Cyclic,
                                        layer: MechtronLayer::Kernel
                                    },
                                    payload,
            )
        }

        fn message_builder(configs: &mut Configs) -> MessageBuilder {
            configs.schemas.cache(&CORE_SCHEMA_EMPTY).unwrap();
            let mut builder = MessageBuilder::new();
            builder.to_nucleus_id = Option::Some(Id::new(0, 0));
            builder.to_tron_id = Option::Some(Id::new(0, 0));
            builder.to_phase = Option::Some("default".to_string());
            builder.to_cycle_kind = Option::Some(Cycle::Next);
            builder.to_port = Option::Some("port".to_string());
            builder.from = Option::Some(mock_from());
            builder.kind = Option::Some(MessageKind::Update);

            let factory = configs.schemas.get(&CORE_SCHEMA_EMPTY).unwrap();
            let buffer = factory.new_buffer(Option::None);
            let buffer = Buffer::new(buffer);

            builder.payloads.replace(Option::Some(vec![Payload{
                buffer: buffer.read_only(),
                schema: CORE_SCHEMA_EMPTY.clone(),
            }]));

            let mut seq = IdSeq::new(0);

            configs.schemas.cache(&CORE_SCHEMA_EMPTY).unwrap();
            let factory = configs.schemas.get(&CORE_SCHEMA_EMPTY).unwrap();
            let buffer = factory.new_buffer(Option::None);
            let buffer = Buffer::new(buffer);
            let payload = Payload{
                buffer: buffer.read_only(),
                schema: CORE_SCHEMA_META_CREATE.clone(),
            };

            builder
        }

        fn mock_from() -> mechtron_common::message::From {
            return mechtron_common::message::From {
                tron: mock_tron_key(),
                cycle: 0,
                timestamp: 0,
                layer: MechtronLayer::Kernel
            }
        }

        fn mock_tron_key() -> MechtronKey {
            MechtronKey {
                mechtron: Id {
                    seq_id: 0,
                    id: 0,
                },
                nucleus: Id {
                    seq_id: 0,
                    id: 0,
                },
            }
        }

        struct MockNucleusMessagingContext;



        #[test]
        fn test_intake_next()
        {
            let mut messaging = CyclicMessagingStructure::new();
            let mut configs = create_configs();
            let configs_ref = &mut configs;
            let mut builder = message_builder(configs_ref);
            builder.to_cycle_kind = Option::Some(Cycle::Next);
            let message = Arc::new(builder.build(Arc::new(IdSeq::new(0))).unwrap());
            let context = CycleMessagingContext{
                head: 0
            };

            messaging.intake(message, context);

            let query = messaging.query(0).unwrap();
            assert_eq!(0, query.len());

            let query = messaging.query(1).unwrap();
            assert_eq!(1, query.len());
        }

        #[test]
        fn test_intake_exact()
        {
            let mut messaging = CyclicMessagingStructure::new();
            let mut configs = create_configs();
            let configs_ref = &mut configs;
            let mut builder = message_builder(configs_ref);
            builder.to_cycle_kind = Option::Some(Cycle::Exact(0));
            let message = Arc::new(builder.build(Arc::new(IdSeq::new(0))).unwrap());
            let context =CycleMessagingContext{
                head: 0
            };

            messaging.intake(message, context);


            let query = messaging.query(0).unwrap();
            assert_eq!(1, query.len());

            let query = messaging.query(1).unwrap();
            assert_eq!(0, query.len());
        }


        fn mock_intake(cycle: i64, messaging: &mut CyclicMessagingStructure, configs: &mut Configs)
        {
            let mut builder = message_builder(configs);
            builder.to_cycle_kind = Option::Some(Cycle::Exact(cycle));
            let message = builder.build(Arc::new(IdSeq::new(0))).unwrap();
            let context =  CycleMessagingContext{head:0};

            messaging.intake(Arc::new(message), context);
        }


        #[test]
        fn test_intake_release()
        {
            let mut messaging = CyclicMessagingStructure::new();
            let mut configs = create_configs();
            let configs_ref = &mut configs;

            mock_intake(0, &mut messaging, configs_ref);
            mock_intake(1, &mut messaging, configs_ref);

            let query = messaging.query(0).unwrap();
            assert_eq!(1, query.len());
            let query = messaging.query(1).unwrap();
            assert_eq!(1, query.len());

            let releases = messaging.release(1).unwrap();
            assert_eq!(1, query.len());
            let query = messaging.query(0).unwrap();
            assert_eq!(0, query.len());
            let query = messaging.query(1).unwrap();
            assert_eq!(1, query.len());


            let releases = messaging.release(2).unwrap();
            assert_eq!(1, query.len());

            let query = messaging.query(0).unwrap();
            assert_eq!(0, query.len());
            let query = messaging.query(0).unwrap();
            assert_eq!(0, query.len());
        }

        fn mock_nucleus_messaging_structure() -> CyclicMessagingStructure
        {
            let mut messaging = CyclicMessagingStructure::new();
            let mut configs = create_configs();
            let configs_ref = &mut configs;


            mock_intake(0, &mut messaging, configs_ref);
            mock_intake(1, &mut messaging, configs_ref);

            messaging
        }

        #[test]
        fn test_nucleus_cycle_messaging_structure1()
        {
            let mut nucleus_messaging = CyclicMessagingStructure::new();
            let mut cyclic_messaging = PhasicMessagingStructure::new();
            let mut configs = create_configs();
            let configs_ref = &mut configs;

            mock_intake(0, &mut nucleus_messaging, configs_ref);
            mock_intake(1, &mut nucleus_messaging, configs_ref);

            let mut messages = nucleus_messaging.query(0).unwrap();

            assert_eq!(1, messages.len());
            while let Some(message) = messages.pop()
            {
                cyclic_messaging.intake(message).unwrap();
            }

            let messages = cyclic_messaging.drain(&"default".to_string()).unwrap().unwrap();

            assert_eq!(1, messages.len());


            let mut messages = nucleus_messaging.query(1).unwrap();

            assert_eq!(1, messages.len());
            while let Some(message) = messages.pop()
            {
                cyclic_messaging.intake(message).unwrap();
            }

            let messages = cyclic_messaging.drain(&"default".to_string()).unwrap().unwrap();
        }

        #[test]
        fn test_nucleus_cycle_messaging_structure2()
        {
            let mut cyclic_messaging = PhasicMessagingStructure::new();
            let mut configs = create_configs();
            let mut id_seq = Arc::new(IdSeq::new(0));
            let configs_ref = &mut configs;

            let mut builder = message_builder(configs_ref);
            builder.to_phase = Option::Some("default".to_string());
            let message = builder.build(id_seq.clone()).unwrap();
            cyclic_messaging.intake(Arc::new(message));

            let mut builder = message_builder(configs_ref);
            builder.to_phase = Option::Some("1".to_string());
            let message = builder.build(id_seq.clone()).unwrap();
            cyclic_messaging.intake(Arc::new(message));

            assert_eq!(1, cyclic_messaging.drain(&"default".to_string()).unwrap().unwrap().len());
            assert!(cyclic_messaging.drain(&"0".to_string()).unwrap().is_none());
            assert_eq!(1, cyclic_messaging.drain(&"1".to_string()).unwrap().unwrap().len());
        }
    }

    #[derive(Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone)]
    pub enum IntraPhasicStep
    {
        Message,
        Update,
        Create,
        Destroy
    }


    #[derive(Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone)]
    struct IntraPhasicKey
    {
        step: IntraPhasicStep,
        phase: u8
    }
}


mod state
{
    use std::collections::HashMap;
    use std::rc::Rc;
    use std::sync::{Arc, Mutex, RwLock};

    use mechtron_common::id::{Id, MechtronKey, Revision, StateKey};
    use mechtron_common::state::{ReadOnlyState, State};

    use crate::nucleus::Error;
    use crate::membrane::{MechtronMembrane, StateExtraction};

    pub struct StateHistory {
        history: RwLock<HashMap<Revision, HashMap<MechtronKey, Arc<ReadOnlyState>>>>,
    }

    impl StateHistory {
        pub fn new() -> Self {
            StateHistory {
                history: RwLock::new(HashMap::new()),
            }
        }

        fn unwrap<V>(&self, option: Option<V>) -> Result<V, Error> {
            match option {
                None => return Err("option was none".into()),
                Some(value) => Ok(value),
            }
        }

        pub fn get(&self, key: &StateKey) -> Result<Arc<ReadOnlyState>, Error> {
            let history = self.history.read()?;
            let history = self.unwrap(history.get(&key.revision))?;
            let state = self.unwrap(history.get(&key.tron))?;
            Ok(state.clone())
        }

        pub fn read_only(&self, key: &StateKey) -> Result<Arc<ReadOnlyState>, Error> {
            Ok(self.get(key)?.clone())
        }

        pub fn query(&self, revision: Revision) -> Result<Option<Vec<(MechtronKey, Arc<ReadOnlyState>)>>, Error> {
            let history = self.history.read()?;
            let history = history.get(&revision);
            match history {
                None => Ok(Option::None),
                Some(history) => {
                    let mut rtn: Vec<(MechtronKey, Arc<ReadOnlyState>)> = vec![];
                    for key in history.keys() {
                        let state = history.get(key).unwrap();
                        rtn.push((key.clone(), state.clone()))
                    }
                    Ok(Option::Some(rtn))
                }
            }
        }

        pub fn intake(&self, state: Arc<ReadOnlyState>, key: StateKey) -> Result<(), Error> {
            let mut history = self.history.write()?;
            let mut history =
                history
                    .entry(key.revision.clone())
                    .or_insert(HashMap::new());
            history.insert(key.tron.clone(), state);
            Ok(())
        }

        fn drain(&mut self, before_cycle: i64) -> Result<Vec<(StateKey, Arc<ReadOnlyState>)>, Error>
        {
            let mut history = self.history.write()?;
            let mut revisions = vec!();
            {
                for revision in history.keys()
                {
                    if revision.cycle < before_cycle
                    {
                        revisions.push(revision.clone());
                    }
                }
            }

            if revisions.is_empty()
            {
                return Ok(vec!());
            }

            let mut rtn = vec!();

            for revision in revisions {

                let mut revision_states = history.remove(&revision).unwrap();
                for state_entry in revision_states.drain().map(|(key, state)| {
                    let key = StateKey
                    {
                        tron: key,
                        revision: revision.clone(),
                    };

                    return (key, state);
                }) {
                    rtn.push(state_entry);
                }
            }

            Ok(rtn)
        }
    }


    pub struct MechtronKernels {
        store: RwLock<HashMap<MechtronKey, Arc<Mutex<MechtronMembrane>>>>,
    }

    impl MechtronKernels {
        pub fn new() -> Self {
            MechtronKernels {
                store: RwLock::new(HashMap::new()),
            }
        }

        pub fn intake(&mut self, key: MechtronKey, mechtron: MechtronMembrane) -> Result<(), Error> {
            let mut store = self.store.write()?;
            store.insert(key, Arc::new(Mutex::new(mechtron)));
            Ok(())
        }

        pub fn get(&self, key: &MechtronKey) -> Result<Arc<Mutex<MechtronMembrane>>, Error> {
            let store = self.store.read()?;
            let rtn = store.get(key);

            match rtn {
                None => Err("could not find state".into()),
                Some(rtn) => {
                    Ok(rtn.clone())
                }
            }
        }

        pub fn drain(&mut self, revision: &Revision) -> Result<Vec<(StateKey, Arc<ReadOnlyState>)>, Error> {
            let mut store = self.store.write()?;
            let mut rtn = vec![];
            for (key, kernel) in store.drain() {
                let key = StateKey {
                    tron: key.clone(),
                    revision: revision.clone(),
                };
                let state = {kernel.lock()?.extract()?};

                let state = match state {
                    StateExtraction::Unchanged(state) => {
                        state
                    }
                    StateExtraction::Changed(state) => {
                        let state = state.lock()?;
                        Arc::new(state.read_only()?)
                    }
                };
                rtn.push((key, state));
            }
            return Ok(rtn);
        }
    }


    #[cfg(test)]
    mod test {
        use std::rc::Rc;
        use std::sync::{Arc, Mutex};

        use mechtron_common::buffers::Buffer;
        use mechtron_common::configs::Configs;
        use mechtron_common::core::*;
        use mechtron_common::id::{Id, MechtronKey, Revision, StateKey};
        use mechtron_common::state::State;

        use crate::nucleus::state::StateHistory;
        use crate::test::create_configs;

        pub fn mock_state(configs_ref: &Configs) -> State
        {
            let config = configs_ref.mechtrons.get( &CORE_MECHTRON_NEUTRON).unwrap();
            let state = State::new(configs_ref, config).unwrap();
            state
        }

        pub fn mock_state_key() -> StateKey
        {
            StateKey {
                tron: MechtronKey {
                    nucleus: Id::new(0,0),
                    mechtron: Id ::new(0, 0)
                },

                revision: Revision {
                    cycle: 0
                }
            }
        }

        #[test]
        pub fn test()
        {
            let mut configs = create_configs();
            let mut configs_ref = &mut configs;
            let mut history = StateHistory::new();
            let state = mock_state(configs_ref).read_only().unwrap();

            let key = mock_state_key();
            history.intake(Arc::new(state), key ).unwrap();

            let revision = Revision{
                cycle: 0
            };

            let states = history.query(revision.clone()).unwrap().unwrap();
            assert_eq!(1, states.len());

            let states = history.drain(1).unwrap();
            assert_eq!(1, states.len());

            let states = history.query(revision.clone()).unwrap();
            assert!(states.is_none());
        }
    }


    pub struct Lookup
    {
        history: Arc<StateHistory>,
        revision: Revision
    }

    impl Lookup {

        pub fn new( history: Arc<StateHistory>, revision: Revision)->Self
        {
            Lookup{
                history: history,
                revision: revision
            }
        }

        pub fn lookup_nucleus(&self, nucleus_id: Id, name: &str) -> Result<Id, Error> {
            let neutron_key = MechtronKey {
                nucleus: nucleus_id,
                mechtron: Id::new(nucleus_id.seq_id, 0),
            };
            let state_key = StateKey {
                tron: neutron_key,
                revision: Revision {
                    cycle: self.revision.cycle - 1,
                },
            };

            let neutron_state = self.history.get(&state_key)?;

            let sim_id = Id::new(
                neutron_state
                    .buffers.get("data").unwrap()
                    .get::<i64>(&path![&"sim_id", &"seq_id"])?,
                neutron_state
                    .buffers.get("data").unwrap()
                    .get::<i64>(&path![&"sim_id", &"id"])?,
            );

            let simtron_key = MechtronKey {
                nucleus: sim_id.clone(),
                mechtron: Id::new(sim_id.seq_id, 1),
            };

            let state_key = StateKey {
                tron: simtron_key,
                revision: Revision {
                    cycle: self.revision.cycle - 1,
                },
            };
            let simtron_state = self.history.get(&state_key)?;

            let nucleus_id = Id::new(
                simtron_state
                    .buffers.get("data").unwrap()
                    .get::<i64>(&path![&"nucleus_names", name, &"seq_id"])?,
                simtron_state
                    .buffers.get("data").unwrap()
                    .get::<i64>(&path![&"nucleus_names", name, &"id"])?,
            );

            Ok(nucleus_id)
        }

        pub fn lookup_tron(
            &self,
            nucleus_id: &Id,
            name: &str,
        ) -> Result<MechtronKey, Error> {
            let neutron_key = MechtronKey {
                nucleus: nucleus_id.clone(),
                mechtron: Id::new(nucleus_id.seq_id, 0),
            };
            let state_key = StateKey {
                tron: neutron_key,
                revision: Revision {
                    cycle: self.revision.cycle - 1,
                },
            };
            let neutron_state = self.history.get(&state_key)?;
            let tron_id = Id::new(
                neutron_state
                    .buffers.get("data").unwrap()
                    .get::<i64>(&path![&"tron_names", name, &"seq_id"])?,
                neutron_state
                    .buffers.get("data").unwrap()
                    .get::<i64>(&path![&"tron_names", name, &"id"])?,
            );

            let tron_key = MechtronKey {
                nucleus: nucleus_id.clone(),
                mechtron: tron_id,
            };

            Ok(tron_key)
        }
    }
}



#[cfg(test)]
mod test
{
    use std::cell::RefCell;
    use std::sync::Arc;

    use mechtron_common::artifact::Artifact;
    use mechtron_common::configs::SimSpark;
    use mechtron_common::core::*;
    use mechtron_common::id::{Id, MechtronKey};
    use mechtron_common::message::*;
    use mechtron_common::util::PingPayloadBuilder;

    use std::io;
    use std::io::Write;
    use crate::star::Star;

    fn create_node() -> Star
    {
//        let node = Node::new(Central::new(), Option::None);
 //       node
        unimplemented!()
    }

    #[test]
    fn test_create_sim()
    {
        unimplemented!()
        /*
        let cache = Node::default_cache();
        let SIM_CONFIG = Artifact::from("mechtron.io:examples:0.0.1:/hello-world/simulation.yaml:sim").unwrap();
        cache.configs.sims.cache(&SIM_CONFIG ).unwrap();
        let SIM_CONFIG = cache.configs.sims.get(&SIM_CONFIG).unwrap();

        let node = Node::new(Option::Some(cache));
        let sim_id = node.create_sim_from_scratch(SIM_CONFIG.clone()).unwrap();

        // verify sim exists

        node.shutdown();

         */
    }

    /*
    #[test]
    fn test_ping_neutron()
    {

        let node = create_node();

        let nucleus_config = node.cache.configs.nucleus.get( &CORE_NUCLEUS_SIMULATION).unwrap();

        let (sim_id,nucleus1)= node.create_sim().unwrap();
        let nucleus2 = node.create_nucleus(&sim_id,nucleus_config.clone()).unwrap();
        println!("nucleus 1 {:?}", nucleus1);
        println!("nucleus 2 {:?}", nucleus2);

        let ping = PingPayloadBuilder::new(&node.cache.configs).unwrap();
        let message = Message::longform(node.net.seq().clone(),
                                              MessageKind::Request,
                                              From {
                                                  tron: TronKey { nucleus: nucleus2.clone(), tron: Id { seq_id: 0, id: 0 } },
                                                  cycle: 0,
                                                  timestamp: 0,
                                                  layer: TronLayer::Shell
                                              },
                                              To {
                                                  tron: TronKey {
                                                      nucleus: nucleus1,
                                                      tron: Id { seq_id: 0, id: 0 },
                                                  },
                                                  port: "ping".to_string(),
                                                  cycle: Cycle::Present,
                                                  phase: "default".to_string(),
                 delivery: DeliveryMoment::ExtraCyclic,
                 layer: TronLayer::Shell
             },Option::Some(To {
                tron: TronKey {
                    nucleus: nucleus2,
                    tron: Id { seq_id: 0, id: 0 },
                },
                port: "pong".to_string(),
                cycle: Cycle::Present,
                phase: "default".to_string(),
                delivery: DeliveryMoment::ExtraCyclic,
                layer: TronLayer::Shell
            }),
                                              vec!(ping),
        Option::None,
        Option::None
        );

        node.send(message);

        node.shutdown();
    }
     */

}



#[derive(Clone)]
pub struct TronInfo
{
    pub key: MechtronKey,
    pub config: Arc<MechtronConfig>,
    pub bind: Arc<BindConfig>
}

