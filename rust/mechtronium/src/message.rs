use mechtron_core::id::{Id, Revision, TronKey};
use mechtron_core::message::{Cycle, Message};
use std::collections::hash_map::RandomState;
use std::collections::HashMap;
use std::error::Error;
use std::sync::{Arc, RwLock};
use std::time::Instant;

use crate::mechtronium::Mechtronium;
use crate::nucleus::{Nucleus, NucleusError};
use wasmer::wasmparser::NameType::Local;




pub struct NucleusMessagingStructure {
    store: RwLock<HashMap<i64, HashMap<TronKey, TronMessageChamber>>>,
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
        context: &dyn NucleusMessagingContext,
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

    pub fn release( &mut self, before_cycle: i64 )-> Result<usize, NucleusError>
    {
        let mut releases= vec!();

        {
            let store = self.store.read()?;

            for cycle in store.keys()
            {
                if cycle < &before_cycle
                {
                    releases.push(cycle.clone() );
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

struct MessageDelivery {
    received: Instant,
    cycle: i64,
    message: Arc<Message>,
}

impl MessageDelivery {
    fn new(message: Message, context: &dyn NucleusMessagingContext) -> Self {
        MessageDelivery {
            received: Instant::now(),
            cycle: context.head(),
            message: Arc::new(message),
        }
    }
}

pub trait NucleusMessagingContext {
    fn head(&self) -> i64;
}

struct TronMessageChamber {
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

pub struct NucleusCycleMessageStructure {
    store: HashMap<u8, Vec<Arc<Message>>>,
}

impl NucleusCycleMessageStructure {
    pub fn new() -> Self {
        NucleusCycleMessageStructure {
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
#[cfg(test)]
mod tests {
    use crate::message::{NucleusMessagingStructure, NucleusMessagingContext, NucleusCycleMessageStructure};
    use mechtron_core::message::{Message, MessageKind, To, Cycle, DeliveryMoment, Payload, MessageBuilder, PayloadBuilder};
    use mechtron_core::core::*;
    use crate::test::*;
    use mechtron_core::id::{IdSeq, TronKey, Id};
    use mechtron_core::configs::Configs;
    use mechtron_core::buffers::Buffer;
    use std::sync::Arc;


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
            artifact: CORE_CREATE_META.clone()
        };


        let seq_borrow = &mut seq;

        Message::single_payload(seq_borrow,
                                MessageKind::Update,
                                mechtron_core::message::From {
                                    tron: TronKey::new(seq_borrow.next(), seq_borrow.next()),
                                    cycle: 0,
                                    timestamp: 0
                                },
                                To {
                                    tron: TronKey::new(seq_borrow.next(), seq_borrow.next()),
                                    port: "someport".to_string(),
                                    cycle: Cycle::Present,
                                    phase: 0,
                                    delivery: DeliveryMoment::Cyclic
                                },
                                payload
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
            artifact: CORE_SCHEMA_EMPTY.clone()
        }]);

        let mut seq = IdSeq::new(0);

        configs.buffer_factory_keeper.cache(&CORE_SCHEMA_EMPTY).unwrap();
        let factory = configs.buffer_factory_keeper.get(&CORE_SCHEMA_EMPTY).unwrap();
        let buffer = factory.new_buffer(Option::None);
        let buffer = Buffer::new(buffer);
        let payload = PayloadBuilder {
            buffer: buffer,
            artifact: CORE_CREATE_META.clone()
        };

        builder.payloads = Option::Some(vec![payload]);

        builder
    }

    fn mock_from() -> mechtron_core::message::From {
        return mechtron_core::message::From {
            tron: mock_tron_key(),
            cycle: 0,
            timestamp: 0
        }
    }

    fn mock_tron_key() -> TronKey {
        TronKey {
            tron: Id {
                seq_id: 0,
                id: 0
            },
            nucleus: Id {
                seq_id: 0,
                id: 0
            }
        }
    }

    struct MockNucleusMessagingContext;

    impl NucleusMessagingContext for MockNucleusMessagingContext
    {
        fn head(&self) -> i64 {
            0
        }
    }


    #[test]
    fn test_intake_next()
    {
        let mut messaging = NucleusMessagingStructure::new();
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
        let mut messaging = NucleusMessagingStructure::new();
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


    fn mock_intake(cycle: i64, messaging: &mut NucleusMessagingStructure, configs: &mut Configs )
    {
        let mut builder = message_builder(configs );
        builder.to_cycle_kind = Option::Some( Cycle::Exact(cycle));
        let message = builder.build(&mut IdSeq::new(0)).unwrap();
        let context = MockNucleusMessagingContext;

        messaging.intake(message,&context);
    }


    #[test]
    fn test_intake_release()
    {
        let mut messaging = NucleusMessagingStructure::new();
        let mut configs = create_configs();
        let configs_ref = &mut configs;

        mock_intake(0, &mut messaging, configs_ref );
        mock_intake(1, &mut messaging, configs_ref );

        let query = messaging.query(0).unwrap();
        assert_eq!( 1, query.len());
        let query = messaging.query(1).unwrap();
        assert_eq!( 1, query.len());

        let releases = messaging.release(1).unwrap();
        assert_eq!( 1, query.len());
        let query = messaging.query(0).unwrap();
        assert_eq!( 0, query.len());
        let query = messaging.query(1).unwrap();
        assert_eq!( 1, query.len());


        let releases = messaging.release(2).unwrap();
        assert_eq!( 1, query.len());

        let query = messaging.query(0).unwrap();
        assert_eq!( 0, query.len());
        let query = messaging.query(0).unwrap();
        assert_eq!( 0, query.len());
    }

    fn mock_nucleus_messaging_structure()->NucleusMessagingStructure
    {
        let mut messaging = NucleusMessagingStructure::new();
        let mut configs = create_configs();
        let configs_ref = &mut configs;


        mock_intake(0, &mut messaging, configs_ref );
        mock_intake(1, &mut messaging, configs_ref );

        messaging
    }

    #[test]
    fn test_nucleus_cycle_messaging_structure1()
    {
        let mut nucleus_messaging = NucleusMessagingStructure::new();
        let mut cyclic_messaging = NucleusCycleMessageStructure::new();
        let mut configs = create_configs();
        let configs_ref = &mut configs;

        mock_intake(0, &mut nucleus_messaging, configs_ref );
        mock_intake(1, &mut nucleus_messaging, configs_ref );

        let mut messages = nucleus_messaging.query(0 ).unwrap();

        assert_eq!(1,messages.len());
        while let Some(message) = messages.pop()
        {
            cyclic_messaging.intake(message).unwrap();
        }

        let messages = cyclic_messaging.drain(&0).unwrap().unwrap();

        assert_eq!(1,messages.len());


        let mut messages = nucleus_messaging.query(1 ).unwrap();

        assert_eq!(1,messages.len());
        while let Some(message) = messages.pop()
        {
            cyclic_messaging.intake(message).unwrap();
        }

        let messages = cyclic_messaging.drain(&0).unwrap().unwrap();
    }

    #[test]
    fn test_nucleus_cycle_messaging_structure2()
    {
        let mut cyclic_messaging = NucleusCycleMessageStructure::new();
        let mut configs = create_configs();
        let mut id_seq = IdSeq::new(0);
        let configs_ref = &mut configs;

        let mut builder = message_builder(configs_ref);
        builder.to_phase=Option::Some(0);
        let message = builder.build(&mut id_seq).unwrap();
        cyclic_messaging.intake(Arc::new(message));

        let mut builder = message_builder(configs_ref);
        builder.to_phase= Option::Some(1);
        let message = builder.build(&mut id_seq).unwrap();
        cyclic_messaging.intake(Arc::new(message));

        assert_eq!( 1, cyclic_messaging.drain(&0).unwrap().unwrap().len() );
        assert!( cyclic_messaging.drain(&0).unwrap().is_none() );
        assert_eq!( 1, cyclic_messaging.drain(&1).unwrap().unwrap().len() );
        assert!( cyclic_messaging.drain(&1).unwrap().is_none() );
        assert!( cyclic_messaging.drain(&2).unwrap().is_none() );
    }

}