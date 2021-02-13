use std::error::Error;
use std::sync::Arc;

use no_proto::memory::NP_Memory_Owned;

use mechtron_common::artifact::{Artifact, ArtifactCacher};
use mechtron_common::buffers::{get, set};
use mechtron_common::configs::{Configs, SimConfig};
use mechtron_common::content::{Content, ReadOnlyContent};
use mechtron_common::id::{ContentKey, Id};
use mechtron_common::id::Revision;
use mechtron_common::id::TronKey;
use mechtron_common::message::{Cycle, InterDeliveryType, Message, MessageKind, Payload, To};
use mechtron_common::revision::Revision;

use crate::app::SYS;
use crate::content::{Content, ContentIntake, ContentRetrieval, InterCyclicContentStructure, IntraCyclicContentStructure};
use crate::message::{MessageIntake, MessagingStructure};
use crate::nucleus::{NeuTron, NucleiStore};
use crate::tron::{Context, CreatePayloadsBuilder, init_tron, init_tron_of_kind, Neutron, Tron, TronShell};

pub struct Source
{
    sim_id: Id,
    pub content: InterCyclicContentStructure,
    pub messaging: MessagingStructure,
    pub head: Revision,
}

pub fn timestamp()->i64
{
    let timestamp = since_the_epoch.as_secs() * 1000 +
        since_the_epoch.subsec_nanos() as u64 / 1_000_000;

    return timestamp;
}

impl Source
{
    pub fn launch(sim_config:Arc<SimConfig>)->Result<Self,Box<dyn Error>> {
        sim_config.cache(&mut SYS.local.configs)?;

        let sim_id = SYS.net.id_seq.next();

        let mut source = Source {
            sim_id: sim_id,
            content: InterCyclicContentStructure::new(),
            messaging: MessagingStructure::new(),
            head: Revision { cycle: 0 }
        };

        source.bootstrap(sim_config);

        Ok(source)
    }

    fn bootstrap(&mut self, sim_config: Arc<SimConfig>) -> Result<(), Box<dyn Error>>
    {
        let timestamp = timestamp();

        let nucleus_id = SYS.net.id_seq.next();
        self.add_nuclues(nucleus_id.clone());

        let neutron_config = SYS.local.configs.core_tron_config("tron/neutron")?;
        let neutron_key = TronKey::new(nucleus_id: nucleus_id.clone(), Id::new(SYS.net.id_seq().clone(), 0));

        let context = Context {
            sim_id: sim_id.clone(),
            id: neutron_key.clone(),
            revision: self.head.clone(),
            tron_config: neutron_config.clone(),
            timestamp: timestamp.clone(),
        };

        // first we create a neutron for the simulation nucleus
        let mut neutron_create_payload_builder = CreatePayloadsBuilder::new(&SYS.local.configs, &neutron_config)?;

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

        // now we send a request to the neutron to create the simtron

        let simtron_config = SYS.local.configs.core_tron_config("trons/sim")?;
        let mut sim_create_payload_builder = CreatePayloadsBuilder::new(&SYS.local.configs, &simtron_config)?;
        sim_create_payload_builder.set_lookup_name("simtron");
        set(&mut sim_create_payload_builder.constructor, &[&"sim_config_artifact"], sim_config.source.to())?;


        let message = Message::multi_payload(&mut SYS.net.id_seq,
                                             MesssageKind::Create,
                                             mechtron_common::message::From { tron: context.id.clone(), cycle: 0, timestamp },
                                             To::basic(
                                                 neutron_key.clone(),
                                                 "create".to_string(),
                                             ),
                                             CreatePayloadsBuilder::payloads(&SYS.local.configs, sim_create_payload_builder));


        self.messaging.cyclic_intake().intake(message)?;
        self.revise( revision.clone(), Revision{ cycle: 1 });

        Ok(())
    }

    pub fn id(&self)->i64
    {
        self.id
    }

    pub fn add_nucleus(&mut self, nucleus_id: Id, nuctron_id: Id) -> Result<(),Box<dyn Error>>
    {
        self.content.create(&TronKey::new(nucleus_id, nuctron_id ) )?;
        return Ok(())
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

        let mut nuclei = vec!();
        for nucleus_id in self.content.query_nuclei(&to)?
        {
            nuclei.push(Nucleus::init(nucleus_id.clone(), context.clone()));
        }

        for mut nucleus in nuclei{
            for (content,content_key) in self.content.query_nucleus_content(nucleus_id, &to, &SYS.local.configs )?
            {
                nucleus.content.intake(content,content_key)?;
            }
            for message in self.messaging.query_messages(nucleus_id, &SYS.local.configs )?
            {
                nucleus.messaging.intake(message)?;
            }
            nucleus.update()?;
        }




        Ok(())
    }
}

struct Nucleus
{
    id: Id,
    content: IntraCyclicContentStructure,
    context: RevisionContext,
}

impl Nucleus
{
    fn init(id: Id, context: RevisionContext, ) -> Self
    {
        Nucleus {
            id: id,
            content: IntraCyclicContentStructure::new(revision),
            context: context
        }
    }


    fn process(&mut self, message: &Message) -> Result<(), Box<dyn Error>>
    {
        match message.kind {
            MessageKind::Create => self.process_create( message),
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

        let content_key = ContentKey { tron_id: message.to.tron.clone(), revision: Revision { cycle: context.cycle } };

        let content = self.content.copy(&content_key)?;

        let context = Context {
            sim_id: self.sim_id.clone(),
            id: message.to.tron.clone(),
            revision: Revision { cycle: self.context.cycle },
            tron_config: neutron_config.clone(),
            timestamp: self.context.timestamp.clone(),
        };



        let neutron = Neutron {};
        neutron.create_tron(&context, message);

        Ok(())
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