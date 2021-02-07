use mechtron_common::message::Message;
use mechtron_common::message::Cycle;
use std::time::Instant;
use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use crate::simulation::Simulation;
use crate::nucleus::Nucleus;

struct MessageStore<'a>
{
  delivery_seq: AtomicI64,
  deliveries: HashMap<i64,MessageDelivery<'a>>,
  cycle_to_deliveries: HashMap<i64,Vec<i64>>
}

struct MessageDelivery<'a>
{
    id: i64,
    received: i64,
    message: Message<'a>
}

enum ReceiveMessageResult<'a>
{
    Accept,
    Reject(Message<'a>)
}

impl <'a> MessageStore<'a>
{

    pub fn new()->Self
    {
        MessageStore{ delivery_seq: AtomicI64::new(0),
                      deliveries: HashMap::new(),
                      cycle_to_deliveries: HashMap::new() }
    }

    pub fn receive( &mut self, current_cycle: i64, message: Message<'a> )->ReceiveMessageResult
    {
        let delivery = MessageDelivery{
            id: self.delivery_seq.fetch_add(1,Ordering::Relaxed ),
            received: current_cycle,
            message: message
        };

        let cycle = match delivery.message.to.cycle {
            Cycle::Some(cycle) => cycle,
            Cycle::Next => current_cycle+1
        };

        if self.cycle_to_deliveries.contains_key(&cycle )
        {
            let vec = self.cycle_to_deliveries.get_mut(&cycle).unwrap();
            vec.push(delivery.id.clone());
        }
        else
        {
            let mut vec = vec!();
            vec.push(delivery.id.clone());
            self.cycle_to_deliveries.insert(cycle, vec );
        }

        self.deliveries.insert(delivery.id, delivery);
        return ReceiveMessageResult::Accept;
    }

    pub fn get_messages( &self, cycle: i64 ) -> Vec<&Message<'a>>
    {
        let mut rtn = vec!();
        if self.cycle_to_deliveries.is_empty() {
            for id in self.cycle_to_deliveries.get(&cycle).unwrap()
            {
                let delivery = self.deliveries.get(id).unwrap();
                rtn.push(&delivery.message);
            }
        }
        return rtn;
    }

}
