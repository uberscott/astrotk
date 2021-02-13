use std::error::Error;
use std::sync::Arc;

use no_proto::memory::NP_Memory_Owned;

use mechtron_common::artifact::{Artifact, ArtifactCacher};
use mechtron_common::buffers::{get, set};
use mechtron_common::configs::{Configs, SimConfig, TronConfig};
use mechtron_common::content::{Content, ReadOnlyContent};
use mechtron_common::id::{ContentKey, Id};
use mechtron_common::id::Revision;
use mechtron_common::id::TronKey;
use mechtron_common::message::{Cycle, InterDeliveryType, Message, MessageKind, Payload, To};
use mechtron_common::revision::Revision;

use crate::app::SYS;
use crate::content::{Content, ContentIntake, ContentRetrieval, NucleusContentStructure, NucleusIntraCyclicContentStructure, NucleusPhasicContentStructure};
use crate::message::{MessageIntake, MessagingStructure, NucleusMessagingStructure, IntakeContext, MessageRouter, NucleusPhasicMessagingStructure, OutboundMessaging};
use crate::nucleus::{NeuTron, NucleiStore};
use crate::tron::{Context, CreatePayloadsBuilder, init_tron, init_tron_of_kind, Neutron, Tron, TronShell};

pub struct Nucleus
{
    id : Id,
    sim_id : Id,
    content: NucleusContentStructure,
    messaging: NucleusMessagingStructure,
    head: Revision
}

fn timestamp()->i64
{
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
            content: NucleusContentStructure::new(),
            messaging: MessagingStructure::new(),
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
        let neutron_key = TronKey::new(nucleus_id: self.id.clone(), Id::new(self.id.seq_id, 0));

        let context = Context {
            sim_id: sim_id.clone(),
            id: neutron_key.clone(),
            revision: self.head.clone(),
            tron_config: neutron_config.clone(),
            timestamp: timestamp.clone(),
        };

        // first we create a neutron for the simulation nucleus
        let mut neutron_create_payload_builder = CreatePayloadsBuilder::new(&SYS.local.configs, &neutron_config)?;
        set( &mut neutron_create_payload_builder.constructor, &[&"sim_id",&"seq_id"], self.sim_id.seq_id );
        set( &mut neutron_create_payload_builder.constructor, &[&"sim_id",&"id"], self.sim_id.id);
        if lookup_name.is_some()
        {
            set( &mut neutron_create_payload_builder.constructor, &[&"lookup_name"], lookup_name.unwrap() );
        }

        let create = Message::multi_payload(&mut SYS.net.id_seq,
                                            MesssageKind::Create,
                                            mechtron_common::message::From { tron: context.id.clone(), cycle: 0, timestamp },
                                            To {
                                                tron: context.id.clone(),
                                                port: "create".to_string(),
                                                cycle: Cycle::Next,
                                                phase: 0,
                                                inter_delivery_type: InterDeliveryType::Cyclic,
                                            },
                                            CreatePayloadsBuilder::payloads(&SYS.local.configs, neutron_create_payload_builder));


        let neutron_content_artifact = neutron_config.content.unwrap().artifact;
        let mut content = Content::new(&SYS.local.configs, neutron_content_artifact.clone() );

        let neutron = TronShell::new( init_tron_of_kind("neutron", &context )? );
        neutron.create(&context,&mut content,&create);

        let content_key = ContentKey {
            tron_id: context.id,
            revision: revision,
        };

        self.content.intake(content, content_key)?;

        Ok(nucleus_id.clone())
    }

    pub fn intake( &mut self, message: Message )
    {
       self.messaging.intake(messge, self );
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

        for (key,content) in self.content.query(&revision)?
        {
            nucleus_cycle.intake_content(key,content)?;
        }

        nucleus_cycle.step()?;

        let (contents, messages) = nucleus_cycle.commit();

        self.head = to;

        for (key, content) in contents
        {
            self.content.intake(content,key)?;
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
    content:  NucleusPhasicContentStructure,
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
            content: NucleusPhasicContentStructure::new(),
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

    fn intake_content(&mut self, key: TronKey, content: &ReadOnlyContent) -> Result<(), Box<dyn Error>>
    {
        self.content.intake(key, content)?;
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
                    process(message)?;
                }
            }
        }
    }

    fn commit(&mut self)->(Vec<(ContentKey,Content)>,Vec<Message>)
    {
        (self.content.drain(self.context.revision()),self.outbound.drain())
    }

    fn tron( &mut self, key: &TronKey )->Result<(Arc<TronConfig>,&mut Content,TronShell,Context),Box<dyn Error>>
    {
        let mut content  = self.content.get(key)?;
        let artifact = content.get_artifact()?;
        let tron_config = SYS.local.configs.tron_config_keeper.get(&artifact)?;
        let tron_shell = TronShell::new(init_tron(&tron_config)? )?;

        let context = Context {
            sim_id: self.sim_id.clone(),
            id: key.clone(),
            revision: Revision { cycle: self.context.cycle },
            tron_config: tron_config.clone(),
            timestamp: self.context.timestamp.clone(),
        };


        Ok( (tron_config, content, tron_shell,context))
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

        let neutron_content_key = ContentKey { tron_id: message.to.tron.clone(), revision: Revision { cycle: context.cycle } };

        let mut neutron_content = self.content.get(&neutron_content_key.tron_id)?;

        let context = Context {
            sim_id: self.sim_id.clone(),
            id: message.to.tron.clone(),
            revision: Revision { cycle: self.context.cycle },
            tron_config: neutron_config.clone(),
            timestamp: self.context.timestamp.clone(),
        };

        let neutron = Neutron{};
        neutron.create_tron(&context, neutron_content , message);

        Ok(())
    }

    fn process_update(&mut self, message: &Message )->Result<(),Box<dyn Error>>
    {
        let content_key = ContentKey { tron_id: message.to.tron.clone(), revision: Revision { cycle: context.cycle } };
        let (config,content,tron,context ) = self.tron(&message.to.tron )?;
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
            InterDeliveryType::Cyclic => {
                self.outbound.push(message);
                return Ok(());
            }
            InterDeliveryType::Phasic => {
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