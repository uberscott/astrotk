use std::collections::HashMap;
use std::sync::{RwLock, Arc};
use mechtron_common::id::{Id, Revision, TronKey};
use mechtron_common::message::{Message, Cycle};
use std::error::Error;
use std::time::Instant;
use std::collections::hash_map::RandomState;

use crate::app::SYS;


pub trait MessageRouter
{
    fn send( message: Message) ->Result<(),Box<dyn Error>>;
}

pub struct GlobalMessageRouter
{

}

impl MessageRouter for GlobalMessageRouter
{
    fn send(message: Message) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}

struct LocalMessageRouter
{
}

impl MessageRouter for LocalMessageRouter
{
    fn send(message: Message) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}


pub struct InterCyclicMessagingStructure
{
    store: RwLock<HashMap<Id, NucleusCyclicMessagingStructure>>
}

struct MessageDelivery
{
    received: Instant,
    cycle: i64,
    message: Arc<Message>
}

impl MessageDelivery {
    fn new( message: Message, context: &dyn IntakeContext )->Self
    {
        MessageDelivery{
            received: Instant::now(),
            cycle: context.head(),
            message: Arc::new(message)
        }
    }
}

trait IntakeContext
{
    fn head()->i64;
    fn phase()->u8;
}

impl InterCyclicMessagingStructure
{
    pub fn intake( &mut self, message: Message, context: &dyn IntakeContext )->Result<(),Box<dyn Error>>
    {
        let delivery = MessageDelivery::new(message,context);
        let mut store = self.store.write()?;
        if !store.contains_key(&delivery.message.to.tron.nucleus_id )
        {
            store.insert( delivery.message.to.tron.nucleus_id.clone(), HashMap::new() );
        }
        let mut nucleus= store.get(&delivery.message.to.tron.nucleus_id ).unwrap();

        nucleus.intake(delivery,context)?;

        Ok(())
    }

    pub fn query( &self, nucleus_id: Id, cycle: i64 )->Result<Vec<Arc<Message>>,Box<dyn Error>>
    {
        let store = self.store.read()?;


        let nucleus = store.get(&nucleus_id );
        match nucleus{
            None => Ok(vec!()),
            Some(nucleus) => Ok(nucleus.query(cycle)?)
        }
    }

}

struct NucleusCyclicMessagingStructure
{
    store: HashMap<i64,HashMap<TronKey, TronMessagingChamber>>
}

impl NucleusCyclicMessagingStructure
{
   pub fn new()->Self
   {
       NucleusCyclicMessagingStructure {
           store: HashMap::new()
       }
   }

   pub fn intake( &mut self, delivery: MessageDelivery, context: &dyn IntakeContext )->Result<(),Box<dyn Error>>
   {
      let desired_cycle = match delivery.message.to.cycle{
          Cycle::Exact(cycle) => {
              cycle
          }
          Cycle::Present => {
              // Nucleus intake is InterCyclic therefore cannot accept present cycles
              contex.head()+1
          }
          Cycle::Next => {
              contex.head()+1
          }
      };
      // at some point we must determine if the nucleus policy allows for message deliveries to this
      // nucleus after x number of cycles and then send a rejection message if needed

       if !self.store.contains_key(&desired_cycle){
           self.store.insert(desired_cycle.clone(), HashMap::new() );
       }

       let store = self.store.get(&desired_cycle ).unwrap();

       if !store.contains_key(&delivery.message.to.tron )
       {
           store.inserts(delivery.message.to.tron.clone(), TronMessagingChamber::new() )
       }

       let mut chamber = store.get(&delivery.message.to.tron).unwrap();
       chamber.intake(delivery)?;

       Ok(())
   }

    pub fn query( &self, cycle: i64 )->Result<Vec<Arc<Message>>,Box<dyn Error>>
    {
        match self.store.get(&cycle )
        {
            None => Ok(vec!()),
            Some(chambers) => {
                let mut rtn = vec!();
                for chamber in chambers.values(){
                    rtn.append(&mut chamber.messages() );
                }
                Ok(rtn)
            }
        }
    }
}

struct TronMessagingChamber
{
    deliveries: Vec<MessageDelivery>
}

impl TronMessagingChamber
{
    pub fn new()->Self
    {
        TronMessagingChamber {
            deliveries: vec!()
        }
    }

    pub fn messages(&self)->Vec<Arc<Message>>
    {
        self.deliveries.iter().map( |d| { d.message.clone() } ).collect()
    }

    pub fn intake( &mut self, delivery: MessageDelivery )->Result<(),Box<dyn Error>>
    {
        self.deliveries.push(delivery);
        Ok(())
    }
}

struct NucleusPhasicMessagingStructure
{
    store: HashMap<u8,Vec<Message>>
}


struct PhasicMessagingRouter
{

}

impl PhasicMessageRouter
{

}

impl NucleusPhasicMessagingStructure
{
    pub fn intake( &mut self, message: Message, context: &IntakeContext ) -> Result<(),Box<dyn Error>>
    {
       if !self.store.contains_key(&message.to.phase )
       {
           self.store.insert(message.to.phase.clone(), vec!() );
       }
       let mut messages = self.store.get( &message.to.phase ).unwrap();
       messages.push( message );

       Ok(())
    }
}
