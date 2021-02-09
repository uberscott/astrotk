use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::Instant;

use mechtron_common::message::Cycle;
use mechtron_common::message::Message;

use crate::nucleus::Nucleus;
use crate::simulation::Simulation;
use mechtron_common::revision::Revision;
use std::sync::RwLock;

pub struct MessageIntakeChamber<'a>
{
    store: RwLock<MessageStore<'a>>
}

struct MessageStore<'a>{
    delivery_seq: AtomicI64,
    deliveries: HashMap<i64, MessageDelivery<'a>>,
    cycle_to_deliveries: HashMap<i64, Vec<i64>>,
}

struct MessageDelivery<'a>
{
    id: i64,
    received: i64,
    message: Message<'a>
}

pub enum IntakeMessageResult<'a>
{
    Accept,
    Reject(Message<'a>)
}

impl <'a> MessageIntakeChamber<'a>
{
    pub fn new() -> Self
    {
        MessageIntakeChamber {
            store: RwLock::new(MessageStore {
                delivery_seq: AtomicI64::new(0),
                deliveries: HashMap::new(),
                cycle_to_deliveries: HashMap::new(),
            })
        }
    }
}

impl <'a> MessageIntake<'a> for MessageIntakeChamber<'a>
{
    fn intake(&mut self, current_cycle: i64, message: Message) -> IntakeMessageResult
    {
        let delivery = MessageDelivery {
            id: self.delivery_seq.fetch_add(1, Ordering::Relaxed),
            received: current_cycle,
            message: message,
        };

        let cycle = match delivery.message.to.cycle {
            Cycle::Some(cycle) => cycle,
            Cycle::Next => current_cycle + 1
        };

        if self.cycle_to_deliveries.contains_key(&cycle)
        {
            let vec = self.cycle_to_deliveries.get_mut(&cycle).unwrap();
            vec.push(delivery.id.clone());
        } else {
            let mut vec = vec!();
            vec.push(delivery.id.clone());
            self.cycle_to_deliveries.insert(cycle, vec);
        }

        self.deliveries.insert(delivery.id, delivery);
        return IntakeMessageResult::Accept;
    }
}

impl <'a> MessageChamber<'a> for MessageIntakeChamber<'a>
{
    fn messages(&self, revision: &Revision ) -> NucleusCycleMessages
    {
        let mut rtn = vec!();
        if self.cycle_to_deliveries.is_empty() {
            for id in self.cycle_to_deliveries.get(&revision.cycle).unwrap()
            {
                let delivery = self.deliveries.get(id).unwrap();
                rtn.push(&delivery.message);
            }
        }

        return NucleusCycleMessages {
            cycle: revision.cycle,
            messages: rtn,
        };
    }

    fn release(&mut self, cycle: i64) {
        let mut cycle_removals = vec!();
        let mut delivery_removals= vec!();
        for c in self.cycle_to_deliveries.keys()
        {
            if *c <= cycle
            {
                cycle_removals.push(cycle);
                let deliveries = self.cycle_to_deliveries.get(c).unwrap();

                for delivery in deliveries{
                    delivery_removals.push(*delivery);
                }
            }
        }

        for cycle_removal in cycle_removals{
            self.cycle_to_deliveries.remove(&cycle_removal);
        }

        for delivery_removal in delivery_removals{
            self.deliveries.remove(&delivery_removal);
        }
    }
}

pub trait MessageIntake<'a>
{
    fn intake(&mut self, current_cycle: i64, message: Message<'a>) -> IntakeMessageResult;
}

pub trait MessageChamber<'a>
{
    fn messages(&self, revision: &Revision) -> NucleusCycleMessages;
    fn release(&mut self, cycle:i64);
}

pub struct NucleusCycleMessages<'a>
{
    cycle: i64,
    messages: Vec<&'a Message<'a>>,
}

pub trait MessageRouter<'a>
{
    fn send( &self, messages: Vec<Message<'a>> );
}

