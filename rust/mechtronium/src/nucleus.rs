use std::{fmt, io};
use std::borrow::BorrowMut;
use std::collections::HashMap;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::rc::Rc;
use std::sync::{Arc, Mutex, PoisonError, RwLock};
use std::time::{Instant, SystemTime};

use no_proto::error::NP_Error;
use no_proto::memory::NP_Memory_Owned;

use mechtron_core::artifact::{Artifact, ArtifactCacher};
use mechtron_core::configs::{Configs, Keeper, SimConfig, TronConfig};
use mechtron_core::core::*;
use mechtron_core::error::MechError;
use mechtron_core::id::{Id, StateKey, IdSeq};
use mechtron_core::id::Revision;
use mechtron_core::id::TronKey;
use mechtron_core::message::{Cycle, DeliveryMoment, Message, MessageKind, Payload, To};
use mechtron_core::state::{ReadOnlyState, ReadOnlyStateMeta, State};

use crate::node::{Node, NucleusContext, Local};
use crate::nucleus::message::{CycleMessagingContext, CyclicMessagingStructure, OutboundMessaging, PhasicMessagingStructure};
use crate::nucleus::state::{PhasicStateStructure, StateHistory};
use crate::router::Router;
use crate::tron::{CreatePayloadsBuilder, init_tron, Neutron, Tron, TronContext, TronShell};
use std::cell::Cell;

pub struct Nuclei<'nuclei> {
    local: Cell<Option<&'nuclei Local<'nuclei>>>,
    nuclei: RwLock<HashMap<Id, Arc<Nucleus<'nuclei>>>>,
}

impl<'nuclei> Nuclei<'nuclei> {
    pub fn new() -> Self {
        Nuclei {
            nuclei: RwLock::new(HashMap::new()),
            local: Cell::new(Option::None)
        }
    }

    pub fn init(&self, local: &'nuclei Local<'nuclei>) {
        self.local.set( Option::Some(local));
    }

    fn local(&self)->&'nuclei Local<'nuclei>
    {
        self.local.get().expect("nucleus must be initialized before it can be used: Nucleus.init()")
    }

    fn configs(&self)->&'nuclei Configs<'nuclei>
    {
        self.local().configs()
    }

    fn node(&self)->&'nuclei Node<'nuclei>
    {
        self.local().node()
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

    pub fn add(&'nuclei self, id:Id, sim_id: Id, lookup_name: Option<String>) -> Result<(), NucleusError> {
        let mut sources = self.nuclei.write()?;
        let nucleus = Nucleus::new( id, sim_id, lookup_name, self )?;
        sources.insert(nucleus.id.clone(), Arc::new(nucleus));
        Ok(())
    }
}

pub struct Nucleus<'nucleus> {
    id: Id,
    sim_id: Id,
    state: StateHistory,
    messaging: CyclicMessagingStructure,
    head: Revision,
    nuclei: &'nucleus Nuclei<'nucleus>,
    lookup_name: Option<String>
}

fn timestamp() -> Result<u64, Box<dyn Error>> {
    let since_the_epoch = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;
    let timestamp =
        since_the_epoch.as_secs() * 1000 + since_the_epoch.subsec_nanos() as u64 / 1_000_000;

    return Ok(timestamp);
}

impl <'nucleus> Nucleus<'nucleus> {
    fn new(
        id: Id,
        sim_id: Id,
        lookup_name: Option<String>,
        nuclei: &'nucleus Nuclei<'nucleus>
    ) -> Result<Self, NucleusError> {

        let mut nucleus = Nucleus {
            id: id,
            sim_id: sim_id,
            state: StateHistory::new(),
            messaging: CyclicMessagingStructure::new(),
            head: Revision { cycle: 0 },
            lookup_name: lookup_name,
            nuclei: nuclei
        };

        Ok(nucleus)
    }

    fn context_from(
        &self,
        key: TronKey,
        artifact: &Artifact,
    ) -> TronContext {
        /*
        let sys = unimplemented!();
        let local = sys.local();
        let configs = &local.configs;
        let tron_config_keeper = &configs.tron_config_keeper;
        let config = tron_config_keeper.get(&artifact).unwrap();

        let config = config.clone();
        let rtn = self.context_for(key, config);
        rtn

         */
        unimplemented!()
    }

    fn context_for(
        &self,
        key: TronKey,
        config: Arc<TronConfig>,
    ) -> TronContext {
        /*
        let sys = self.context.sys();
        TronContext::new(
            key,
            self.head.clone(),
            config.clone(),
            timestamp().unwrap().clone(),
        )

         */
        unimplemented!()
    }

    fn local(&self)->&'nucleus Local<'nucleus>
    {
        self.nuclei.local()
    }

    fn configs(&self)->&'nucleus Configs<'nucleus>
    {
        self.nuclei.local().configs()
    }

    fn node(&self)->&'nucleus Node<'nucleus>
    {
        self.nuclei.local().node()
    }

    fn seq(&self)->Arc<IdSeq>
    {
        self.nuclei.local().node().net().seq()
    }

    fn bootstrap(&mut self, lookup_name: Option<String>) -> Result<Id, Box<dyn Error+'_>> {
        let configs = self.configs();
        let mut seq= self.seq();

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
             seq,
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

        let neutron_state_artifact = context.tron_config.as_ref().content.as_ref().unwrap().artifact.clone();
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

impl <'c> CycleMessagingContext for Nucleus<'c> {
    fn head(&self) -> i64 {
        self.head.cycle
    }

}

struct NucleusCycle<'cycle,'nucleus> {
    nucleus: &'cycle Nucleus<'nucleus>,
    id: Id,
    state: PhasicStateStructure,
    messaging: PhasicMessagingStructure,
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
            state: PhasicStateStructure::new(),
            messaging: PhasicMessagingStructure::new(),
            outbound: OutboundMessaging::new(),
            context: context,
            phase: 0,
        })
    }

    fn panic( &self, error: Box<dyn Debug>)
    {
        println!( "nucleus cycle panic! {:?}",error )
    }

    fn intake_message(&mut self, message: Arc<Message>) -> Result<(), Box<dyn Error>> {
        self.messaging.intake(message)?;
        Ok(())
    }

    fn intake_state(&mut self, key: TronKey, state: &ReadOnlyState) {
        match self.state.intake(key, state)
        {
            Ok(_) => {}
            Err(e) => self.panic(Box::new(e))
        }
    }

    fn run(&mut self) -> Result<(), Box<dyn Error>> {
        let phase: u8 = 0;
        match self.messaging.drain(&phase)? {
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
        /*
        (
            self.state.drain(self.context.revision()),
            self.outbound.drain(),
        )
         */
        unimplemented!()
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

impl<'cycle,'nucleus> Router for NucleusCycle<'cycle,'nucleus> {
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

    pub fn sys(&self) -> Arc<Node<'context>>{
        unimplemented!()
    }
}


#[derive(Debug, Clone)]
pub struct NucleusError{
    error: String
}

impl fmt::Display for NucleusError{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "nucleus error: {:?}",self)
    }
}

impl From<NP_Error> for NucleusError{
    fn from(e: NP_Error) -> Self {
        NucleusError{
            error: format!("{:?}",e)
        }
    }
}

impl From<io::Error> for NucleusError{
    fn from(e: io::Error) -> Self {
        NucleusError{
            error: format!("{:?}",e)
        }
    }
}

impl From<Box<dyn Error>> for NucleusError{
    fn from(e: Box<dyn Error>) -> Self {
        NucleusError{
            error: format!("{:?}",e)
        }
    }
}

impl From<&dyn Error> for NucleusError{
    fn from(e: &dyn Error) -> Self {
        NucleusError{
            error: format!("{:?}",e)
        }
    }
}

impl From<Box<dyn Debug>> for NucleusError{
    fn from(e: Box<Debug>) -> Self {
        NucleusError{
            error: format!("{:?}",e)
        }
    }
}

impl From<&dyn Debug> for NucleusError{
    fn from(e: &dyn Debug) -> Self {
        NucleusError{
            error: format!("{:?}",e)
        }
    }
}


impl From<&str> for NucleusError{
    fn from(e: &str) -> Self {
        NucleusError{
            error: format!("{:?}",e)
        }
    }
}

impl<T> From<PoisonError<T>> for NucleusError {
    fn from(e: PoisonError<T>) -> Self {
        NucleusError {
            error: format!("{:?}", e)
        }
    }
}

mod message
{
    use std::collections::HashMap;
    use std::error::Error;
    use std::sync::{Arc, RwLock};
    use std::time::Instant;

    use mechtron_core::id::TronKey;
    use mechtron_core::message::{Cycle, Message};

    use crate::nucleus::NucleusError;

    pub struct CyclicMessagingStructure {
        store: RwLock<HashMap<i64, HashMap<TronKey, TronMessageChamber>>>,
    }

    impl CyclicMessagingStructure {
        pub fn new() -> Self {
            CyclicMessagingStructure {
                store: RwLock::new(HashMap::new()),
            }
        }

        pub fn intake(
            &mut self,
            message: Message,
            context: &dyn CycleMessagingContext,
        ) -> Result<(), Box<dyn Error + '_>> {
            let delivery = MessageDelivery::new(message, context);

            let mut store = self.store.write()?;

            let desired_cycle = match delivery.message.to.cycle {
                Cycle::Exact(cycle) => cycle,
                Cycle::Present => {
                    // Nucleus intake is InterCyclic therefore cannot accept present cycles
                    context.head() + 1
                }
                Cycle::Next => context.head() + 1
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

        pub fn query(&self, cycle: i64) -> Result<Vec<Arc<Message>>, Box<dyn Error + '_>> {
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

        pub fn release(&mut self, before_cycle: i64) -> Result<usize, NucleusError>
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

        pub fn intake(&mut self, delivery: MessageDelivery) -> Result<(), Box<dyn Error>> {
            self.deliveries.push(delivery);
            Ok(())
        }
    }

    pub struct PhasicMessagingStructure {
        store: HashMap<u8, Vec<Arc<Message>>>,
    }

    impl PhasicMessagingStructure {
        pub fn new() -> Self {
            PhasicMessagingStructure {
                store: HashMap::new(),
            }
        }

        pub fn intake(&mut self, message: Arc<Message>) -> Result<(), Box<dyn Error>> {
            if !self.store.contains_key(&message.to.phase) {
                self.store.insert(message.to.phase.clone(), vec![]);
            }
            let mut messages = self.store.get_mut(&message.to.phase).unwrap();
            messages.push(message);

            Ok(())
        }


        pub fn drain(&mut self, phase: &u8) -> Result<Option<Vec<Arc<Message>>>, Box<dyn Error>> {
            Ok(self.store.remove(phase))
        }
    }



    pub trait CycleMessagingContext {
        fn head(&self) -> i64;
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
        fn new(message: Message, context: &dyn CycleMessagingContext) -> Self {
            MessageDelivery {
                received: Instant::now(),
                cycle: context.head(),
                message: Arc::new(message),
            }
        }
    }


    #[cfg(test)]
    mod tests {
        use std::sync::Arc;

        use mechtron_core::buffers::Buffer;
        use mechtron_core::configs::Configs;
        use mechtron_core::core::*;
        use mechtron_core::id::{Id, IdSeq, TronKey};
        use mechtron_core::message::{Cycle, DeliveryMoment, Message, MessageBuilder, MessageKind, Payload, PayloadBuilder, To};

        use crate::nucleus::message::{CycleMessagingContext, CyclicMessagingStructure, PhasicMessagingStructure};
        use crate::test::*;

        fn message(configs: &mut Configs) -> Message {
            let mut seq = IdSeq::new(0);

            configs.buffer_factory_keeper.cache(&CORE_SCHEMA_EMPTY).unwrap();
//        configs.buffer_factory_keeper.cache(&CORE_SCHEMA_NUCLEUS_LOOKUP_NAME_MESSAGE).unwrap();
            let factory = configs.buffer_factory_keeper.get(&CORE_SCHEMA_EMPTY).unwrap();
            let buffer = factory.new_buffer(Option::None);
            let buffer = Buffer::new(buffer);
            let buffer = buffer.read_only();
            let payload = Payload {
                buffer: buffer,
                artifact: CORE_CREATE_META.clone(),
            };


            let seq_borrow = &Arc::new(seq);

            Message::single_payload(seq_borrow.clone(),
                                    MessageKind::Update,
                                    mechtron_core::message::From {
                                        tron: TronKey::new(seq_borrow.next(), seq_borrow.next()),
                                        cycle: 0,
                                        timestamp: 0,
                                    },
                                    To {
                                        tron: TronKey::new(seq_borrow.next(), seq_borrow.next()),
                                        port: "someport".to_string(),
                                        cycle: Cycle::Present,
                                        phase: 0,
                                        delivery: DeliveryMoment::Cyclic,
                                    },
                                    payload,
            )
        }

        fn message_builder(configs: &mut Configs) -> MessageBuilder {
            configs.buffer_factory_keeper.cache(&CORE_SCHEMA_EMPTY).unwrap();
            let mut builder = MessageBuilder::new();
            builder.to_nucleus_id = Option::Some(Id::new(0, 0));
            builder.to_tron_id = Option::Some(Id::new(0, 0));
            builder.to_phase = Option::Some(0);
            builder.to_cycle_kind = Option::Some(Cycle::Next);
            builder.to_port = Option::Some("port".to_string());
            builder.from = Option::Some(mock_from());
            builder.kind = Option::Some(MessageKind::Update);

            let factory = configs.buffer_factory_keeper.get(&CORE_SCHEMA_EMPTY).unwrap();
            let buffer = factory.new_buffer(Option::None);
            let buffer = Buffer::new(buffer);

            builder.payloads = Option::Some(vec![PayloadBuilder {
                buffer: buffer,
                artifact: CORE_SCHEMA_EMPTY.clone(),
            }]);

            let mut seq = IdSeq::new(0);

            configs.buffer_factory_keeper.cache(&CORE_SCHEMA_EMPTY).unwrap();
            let factory = configs.buffer_factory_keeper.get(&CORE_SCHEMA_EMPTY).unwrap();
            let buffer = factory.new_buffer(Option::None);
            let buffer = Buffer::new(buffer);
            let payload = PayloadBuilder {
                buffer: buffer,
                artifact: CORE_CREATE_META.clone(),
            };

            builder.payloads = Option::Some(vec![payload]);

            builder
        }

        fn mock_from() -> mechtron_core::message::From {
            return mechtron_core::message::From {
                tron: mock_tron_key(),
                cycle: 0,
                timestamp: 0,
            }
        }

        fn mock_tron_key() -> TronKey {
            TronKey {
                tron: Id {
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

        impl CycleMessagingContext for MockNucleusMessagingContext
        {
            fn head(&self) -> i64 {
                0
            }
        }


        #[test]
        fn test_intake_next()
        {
            let mut messaging = CyclicMessagingStructure::new();
            let mut configs = create_configs();
            let configs_ref = &mut configs;
            let mut builder = message_builder(configs_ref);
            builder.to_cycle_kind = Option::Some(Cycle::Next);
            let message = builder.build(&mut IdSeq::new(0)).unwrap();
            let context = MockNucleusMessagingContext;

            messaging.intake(message, &context);

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
            let message = builder.build(&mut IdSeq::new(0)).unwrap();
            let context = MockNucleusMessagingContext;

            messaging.intake(message, &context);


            let query = messaging.query(0).unwrap();
            assert_eq!(1, query.len());

            let query = messaging.query(1).unwrap();
            assert_eq!(0, query.len());
        }


        fn mock_intake(cycle: i64, messaging: &mut CyclicMessagingStructure, configs: &mut Configs)
        {
            let mut builder = message_builder(configs);
            builder.to_cycle_kind = Option::Some(Cycle::Exact(cycle));
            let message = builder.build(&mut IdSeq::new(0)).unwrap();
            let context = MockNucleusMessagingContext;

            messaging.intake(message, &context);
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

            let messages = cyclic_messaging.drain(&0).unwrap().unwrap();

            assert_eq!(1, messages.len());


            let mut messages = nucleus_messaging.query(1).unwrap();

            assert_eq!(1, messages.len());
            while let Some(message) = messages.pop()
            {
                cyclic_messaging.intake(message).unwrap();
            }

            let messages = cyclic_messaging.drain(&0).unwrap().unwrap();
        }

        #[test]
        fn test_nucleus_cycle_messaging_structure2()
        {
            let mut cyclic_messaging = PhasicMessagingStructure::new();
            let mut configs = create_configs();
            let mut id_seq = IdSeq::new(0);
            let configs_ref = &mut configs;

            let mut builder = message_builder(configs_ref);
            builder.to_phase = Option::Some(0);
            let message = builder.build(&mut id_seq).unwrap();
            cyclic_messaging.intake(Arc::new(message));

            let mut builder = message_builder(configs_ref);
            builder.to_phase = Option::Some(1);
            let message = builder.build(&mut id_seq).unwrap();
            cyclic_messaging.intake(Arc::new(message));

            assert_eq!(1, cyclic_messaging.drain(&0).unwrap().unwrap().len());
            assert!(cyclic_messaging.drain(&0).unwrap().is_none());
            assert_eq!(1, cyclic_messaging.drain(&1).unwrap().unwrap().len());
            assert!(cyclic_messaging.drain(&1).unwrap().is_none());
            assert!(cyclic_messaging.drain(&2).unwrap().is_none());
        }
    }
}

mod state
{
    use std::collections::HashMap;
    use std::rc::Rc;
    use std::sync::{Arc, Mutex, RwLock};

    use mechtron_core::id::{Revision, StateKey, TronKey};
    use mechtron_core::state::{ReadOnlyState, State};

    use crate::nucleus::NucleusError;

    pub struct StateHistory {
        history: RwLock<HashMap<Revision, HashMap<TronKey, Arc<ReadOnlyState>>>>,
    }

    impl StateHistory {
        pub fn new() -> Self {
            StateHistory {
                history: RwLock::new(HashMap::new()),
            }
        }

        fn unwrap<V>(&self, option: Option<V>) -> Result<V, NucleusError> {
            match option {
                None => return Err("option was none".into()),
                Some(value) => Ok(value),
            }
        }

        pub fn get(&self, key: &StateKey) -> Result<Arc<ReadOnlyState>, NucleusError> {
            let history = self.history.read()?;
            let history = self.unwrap(history.get(&key.revision))?;
            let state = self.unwrap(history.get(&key.tron))?;
            Ok(state.clone())
        }

        pub fn read_only(&self, key: &StateKey) -> Result<Arc<ReadOnlyState>, NucleusError> {
            Ok(self.get(key)?.clone())
        }

        pub fn query(&self, revision: &Revision) -> Result<Option<Vec<(TronKey, Arc<ReadOnlyState>)>>, NucleusError> {
            let history = self.history.read()?;
            let history = history.get(&revision);
            match history {
                None => Ok(Option::None),
                Some(history) => {
                    let mut rtn: Vec<(TronKey, Arc<ReadOnlyState>)> = vec![];
                    for key in history.keys() {
                        let state = history.get(key).unwrap();
                        rtn.push((key.clone(), state.clone()))
                    }
                    Ok(Option::Some(rtn))
                }
            }
        }

        fn intake(&mut self, state: Rc<State>, key: StateKey) -> Result<(), NucleusError> {
            let state = state.read_only()?;
            let mut history = self.history.write()?;
            let mut history =
                history
                    .entry(key.revision.clone())
                    .or_insert(HashMap::new());
            history.insert(key.tron.clone(), Arc::new(state));
            Ok(())
        }

        fn drain(&mut self, before_cycle: i64) -> Result<Vec<(StateKey, Arc<ReadOnlyState>)>, NucleusError>
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
//            let history = self.history.write()?;

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


    pub struct PhasicStateStructure {
        store: RwLock<HashMap<TronKey, Arc<Mutex<State>>>>,
    }

    impl PhasicStateStructure {
        pub fn new() -> Self {
            PhasicStateStructure {
                store: RwLock::new(HashMap::new()),
            }
        }

        pub fn intake(&mut self, key: TronKey, state: &ReadOnlyState) -> Result<(), NucleusError> {
            let mut store = self.store.write()?;
            store.insert(key, Arc::new(Mutex::new(state.copy())));
            Ok(())
        }

        pub fn get(&mut self, key: &TronKey) -> Result<Arc<Mutex<State>>, NucleusError> {
            let store = self.store.read()?;
            let rtn = store.get(key);

            match rtn {
                None => Err("could not find state".into()),
                Some(rtn) => {
                    Ok(rtn.clone())
                }
            }
        }

        pub fn drain(&mut self, revision: &Revision) -> Result<Vec<(StateKey, Arc<Mutex<State>>)>, NucleusError> {
            let mut store = self.store.write()?;
            let mut rtn = vec![];
            for (key, state) in store.drain() {
                let key = StateKey {
                    tron: key.clone(),
                    revision: revision.clone(),
                };
                rtn.push((key, state));
            }
            return Ok(rtn);
        }
    }


    #[cfg(test)]
    mod test{
        use crate::test::create_configs;
        use crate::nucleus::state::StateHistory;
        use mechtron_core::state::State;
        use mechtron_core::buffers::Buffer;
        use mechtron_core::core::*;
        use mechtron_core::configs::Configs;
        use std::rc::Rc;
        use mechtron_core::id::{StateKey, TronKey, Id, Revision};

        pub fn mock_state(configs_ref:&Configs)->State
        {
            let state = State::new(configs_ref, CORE_SCHEMA_EMPTY.clone() ).unwrap();
            state
        }

        pub fn mock_state_key()->StateKey
        {
            StateKey{
                tron: TronKey{
                    nucleus: Id::new(0,0),
                    tron: Id ::new(0,0)
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
            let state = mock_state(configs_ref);

            let key = mock_state_key();
            history.intake(Rc::new(state), key ).unwrap();

            let revision = Revision{
                cycle: 0
            };

            let states = history.query(&revision).unwrap().unwrap();
            assert_eq!(1,states.len());

            let states= history.drain( 1 ).unwrap();
            assert_eq!(1,states.len());

            let states = history.query(&revision).unwrap();
            assert!(states.is_none());
        }
    }

}




