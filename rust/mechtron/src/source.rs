use crate::content::{ContentStore, ContentIntake, Content};
use crate::nucleus::{NucleiStore, NeuTron};
use crate::message::{MessageStore, MessageIntake};
use std::error::Error;
use crate::app::SYS;
use mechtron_common::revision::Revision;
use mechtron_common::id::{Id, ContentKey};
use mechtron_common::id::TronKey;
use mechtron_common::id::Revision;
use mechtron_common::configs::SimConfig;
use mechtron_common::artifact::{ArtifactCacher, Artifact};
use std::sync::Arc;
use crate::tron::{Context, Tron, init_tron_of_kind};
use mechtron_common::message::{Message, MessageKind, To, Cycle, InterDeliveryType, Payload};
use mechtron_common::content::Content;

pub struct Source
{
    sim_id: Id,
    pub content: ContentStore,
    pub nuclei: Vec<Id>,
    pub messages: MessageStore,
    pub head: Revision
}

impl Source
{
    pub fn launch(sim_config:Arc<SimConfig>)->Result<Self,Box<dyn Error>> {
        sim_config.cache(&mut SYS.local.configs)?;

        let sim_id = SYS.net.id_seq.next();

        let mut source = Source {
            sim_id: sim_id,
            content: ContentStore::new(),
            nuclei: vec!(),
            messages: MessageStore::new(),
            head: Revision{ cycle: 0 }
        };
        let timestamp= since_the_epoch.as_secs() * 1000 +
            since_the_epoch.subsec_nanos() as u64 / 1_000_000;

        let nucleus_id = SYS.net.id_seq.next();
        source.add_nuclues(nucleus_id.clone());

        let neutron_artifact = Artifact::from("mechtron.io:core:0.0.1:tron/neutron.yaml")?;
        SYS.local.configs.tron_config_keeper.cache(&neutron_artifact)?;

        // now we need to create the neutron
        let neutron_config = SYS.local.configs.tron_config_keeper.get(&neutron_artifact)?;
        neutron_config.cache(&mut SYS.local.configs);
        let neutron_create_artifact = neutron_config.messages.unwrap().create.unwrap().artifact;
        let neutron_create_buffer_factory = SYS.local.configs.buffer_factory_keeper.get(&neutron_create_artifact)?;
        let neutron_create_buffer = neutron_create_buffer_factory.new_buffer(Option::None)?;

        let neutron_content_artifact = neutron_config.content.unwrap().artifact;
        let neutron_content_buffer_factory = SYS.local.configs.buffer_factory_keeper.get(&neutron_content_artifact)?;
        let neutron_content_buffer = neutron_content_buffer_factory.new_buffer(Option::None)?;

        let revision = Revision { cycle: 0 };

        let context = Context {
            sim_id: sim_id.clone(),
            id: TronKey::new(nucleus_id: nucleus_id.clone(), SYS.net.id_seq.next()),
            revision: revision.clone(),
            config: neutron_config.clone(),
            timestamp: timestamp.clone()
        };
        let neutron = init_tron_of_kind("neutron", &context )?;
        neutron.create(&context, &mut neutron_content_buffer, &neutron_create_buffer)?;

        let content_key = ContentKey {
            content_id: context.id,
            revision: revision
        };

        let content = Content::new(content_key, neutron_content_buffer);


        source.content.intake(content)?;

        let sim_artifact= Artifact::from("mechtron.io:core:0.0.1:tron/sim.yaml")?;
        SYS.local.configs.tron_config_keeper.cache(&sim_artifact)?;
        let sim_create_artifact = sim_config.messages.unwrap().create.unwrap().artifact;
        let sim_create_buffer_factory = SYS.local.configs.buffer_factory_keeper.get(&sim_create_artifact)?;
        let sim_create_buffer = sim_create_buffer_factory.new_buffer(Option::None );


        let message = Message::single_payload( &mut SYS.net.id_seq,
                     MesssageKind::Create,
                     mechtron_common::message::From{ tron: context.id.clone(), cycle: 0, timestamp },
                         To{
                             tron: TronKey {nucleus_id: nucleus_id.clone(), tron_id: SYS.net.id_seq.next() },
                             port: "create".to_string(),
                             cycle: Cycle::Next,
                             phase: 0,
                             inter_delivery_type: InterDeliveryType::Cyclic,
                         },
                         Payload{ buffer: Arc::new(sim_create_buffer),
                          artifact: sim_artifact.clone()});


        source.messages.cyclic_intake().intake(message)?;
        source.revise( revision.clone(), Revision{ cycle: 1 });

        return Ok(source);
    }

    pub fn id(&self)->i64
    {
        self.id
    }

    pub fn add_nucleus(&mut self, nucleus_id: Id, nuctron_id: Id) -> Result<(),Box<dyn Error>>
    {
        self.nuclei.push(nucleus_id.clone() );
        self.content.create(&TronKey::new(nucleus_id, nuctron_id ) )?;
        return Ok(())
    }

    pub fn revise( &mut self, from: Revision, to: Revision )->Result<(),Box<dyn Error>>
    {
        if from.cycle != to.cycle-1
        {
            return Err("cycles must be sequential. 'from' revision cycle must be exactly 1 less than 'to' revision cycle".into());
        }

        Ok(())
    }
}