use mechtron_core::id::{Id, Revision, TronKey};
use mechtron_core::message::{Cycle, Message};
use std::collections::hash_map::RandomState;
use std::collections::HashMap;
use std::error::Error;
use std::sync::{Arc, RwLock};
use std::time::Instant;

use crate::mechtronium::Mechtronium;
use crate::nucleus::Nucleus;
use wasmer::wasmparser::NameType::Local;

pub trait MessageRouter {
    fn send(&mut self, message: Message);
}

pub struct GlobalMessageRouter<'configs> {
    local: LocalMessageRouter<'configs>,
    sys: Option<Arc<Mechtronium<'configs>>>,
}

impl <'configs> GlobalMessageRouter<'configs> {
    pub fn new() -> Self {
        GlobalMessageRouter {
            local: LocalMessageRouter::new(),
            sys: Option::None,
        }
    }

    pub fn init(&mut self, sys: Arc<Mechtronium<'configs>>) {
        self.local.init(sys.clone());
        self.sys = Option::Some(sys);
    }

    pub fn sys<'get>(&'get self) -> Result<Arc<Mechtronium<'configs>>, Box<dyn Error>> {
        match &self.sys {
            None => Err("sys is not set".into()),
            Some(sys) => Ok(sys.clone()),
        }
    }
}

impl <'configs> MessageRouter for GlobalMessageRouter<'configs> {
    fn send(&mut self, message: Message) {
        self.local.send(message);
    }
}

struct LocalMessageRouter<'configs> {
    sys: Option<Arc<Mechtronium<'configs>>>,
}

impl <'configs> LocalMessageRouter<'configs> {
    pub fn new() -> Self {
        LocalMessageRouter { sys: Option::None }
    }

    pub fn init(&mut self, sys: Arc<Mechtronium<'configs>>) {
        self.sys = Option::Some(sys);
    }

    pub fn sys(&self) -> Result<Arc<Mechtronium<'configs>>, Box<dyn Error>> {
        match &self.sys {
            None => Err("sys is not set".into()),
            Some(sys) => Ok(sys.clone()),
        }
    }
}

impl <'configs>MessageRouter for LocalMessageRouter<'configs> {
    fn send(&mut self, message: Message) {
        if self.sys().is_err() {
            println!("cannot send message because MessageRouter has no connection to the system");
            return;
        }

        match self
            .sys()
            .unwrap()
            .local
            .nuclei
            .get(&message.to.tron.nucleus)
        {
            Ok(nucleus) => {
                let mut nucleus = nucleus;
                nucleus.intake(message);
            }
            Err(e) => {
                print!("message failed to be sent: {:?}", e)
            }
        }
    }
}

struct MessageDelivery {
    received: Instant,
    cycle: i64,
    message: Arc<Message>,
}

impl MessageDelivery {
    fn new(message: Message, context: &dyn IntakeContext) -> Self {
        MessageDelivery {
            received: Instant::now(),
            cycle: context.head(),
            message: Arc::new(message),
        }
    }
}

pub trait IntakeContext {
    fn head(&self) -> i64;
    fn phase(&self) -> u8;
}

pub struct NucleusMessagingStructure {
    store: RwLock<HashMap<i64, HashMap<TronKey, TronMessagingChamber>>>,
}

impl NucleusMessagingStructure {
    pub fn new() -> Self {
        NucleusMessagingStructure {
            store: RwLock::new(HashMap::new()),
        }
    }

    pub fn intake(
        &mut self,
        message: Message,
        context: &dyn IntakeContext,
    ) -> Result<(), Box<dyn Error + '_>> {
        let delivery = MessageDelivery::new(message, context);

        let mut store = self.store.write()?;

        let desired_cycle = match delivery.message.to.cycle {
            Cycle::Exact(cycle) => cycle,
            Cycle::Present => {
                // Nucleus intake is InterCyclic therefore cannot accept present cycles
                context.head() + 1
            }
            Cycle::Next => context.head() + 1,
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
                TronMessagingChamber::new(),
            );
        }

        let mut chamber = store.get_mut(&delivery.message.to.tron).unwrap();
        chamber.intake(delivery)?;

        Ok(())
    }

    pub fn query(&self, cycle: &i64) -> Result<Vec<Arc<Message>>, Box<dyn Error + '_>> {
        let store = self.store.read()?;
        match store.get(cycle) {
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
}

struct TronMessagingChamber {
    deliveries: Vec<MessageDelivery>,
}

impl TronMessagingChamber {
    pub fn new() -> Self {
        TronMessagingChamber { deliveries: vec![] }
    }

    pub fn messages(&self) -> Vec<Arc<Message>> {
        self.deliveries.iter().map(|d| d.message.clone()).collect()
    }

    pub fn intake(&mut self, delivery: MessageDelivery) -> Result<(), Box<dyn Error>> {
        self.deliveries.push(delivery);
        Ok(())
    }
}

pub struct NucleusPhasicMessagingStructure {
    store: HashMap<u8, Vec<Arc<Message>>>,
}

impl NucleusPhasicMessagingStructure {
    pub fn new() -> Self {
        NucleusPhasicMessagingStructure {
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

    pub fn remove(&mut self, phase: &u8) -> Result<Option<Vec<Arc<Message>>>, Box<dyn Error>> {
        let option = self.store.get_mut(phase);
        match option {
            None => Ok(Option::None),
            Some(messages) => {
                let mut rtn = vec![];
                for message in messages {
                    rtn.push(message.clone());
                }
                Ok(Option::Some(rtn))
            }
        }
    }
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
