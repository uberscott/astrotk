use crate::content::{ContentStructure, ContentIntake, Content};
use crate::nucleus::{NucleiStore, NeuTron};
use crate::message::{MessagingStructure, MessageIntake};
use std::error::Error;
use crate::app::SYS;
use mechtron_common::revision::Revision;
use mechtron_common::id::{Id, ContentKey};
use mechtron_common::id::TronKey;
use mechtron_common::id::Revision;
use mechtron_common::configs::SimConfig;
use mechtron_common::artifact::{ArtifactCacher, Artifact};
use std::sync::Arc;
use crate::tron::{Context, Tron, init_tron_of_kind, TronShell};
use mechtron_common::message::{Message, MessageKind, To, Cycle, InterDeliveryType, Payload};
use mechtron_common::content::Content;

pub struct Source
{
    sim_id: Id,
    pub content: ContentStructure,
    pub messaging: MessagingStructure,
    pub head: Revision
}

impl Source
{
    pub fn launch(sim_config:Arc<SimConfig>)->Result<Self,Box<dyn Error>> {
        sim_config.cache(&mut SYS.local.configs)?;

        let sim_id = SYS.net.id_seq.next();

        let mut source = Source {
            sim_id: sim_id,
            content: ContentStructure::new(),
            messaging: MessagingStructure::new(),
            head: Revision { cycle: 0 }
        };

        source.bootstrap();

        Ok(source)
    }

    fn bootstrap( &mut self ) -> Result<(),Box<dyn Error>>
    {
        let timestamp= since_the_epoch.as_secs() * 1000 +
            since_the_epoch.subsec_nanos() as u64 / 1_000_000;

        let nucleus_id = SYS.net.id_seq.next();
        self.add_nuclues(nucleus_id.clone());

        let neutron_config= SYS.local.configs.core_tron_config("tron/neutron")?;
        let neutron_key = TronKey::new(nucleus_id: nucleus_id.clone(), Id::new( SYS.net.id_seq().clone(), 0 ));

        let context = Context {
            sim_id: sim_id.clone(),
            id: neutron_key.clone(),
            revision: self.head.clone(),
            tron_config: neutron_config.clone(),
            timestamp: timestamp.clone(),
            phase: 0
        };

        // first we create a neutron for the simulation nucleus
        let neutron_create_artifact = neutron_config.messages.unwrap().create.unwrap().artifact;
        let neutron_create_buffer_factory = SYS.local.configs.buffer_factory_keeper.get(&neutron_create_artifact)?;
        let neutron_create_buffer = neutron_create_buffer_factory.new_buffer(Option::None)?;
        let create= Message::single_payload( &mut SYS.net.id_seq,
                                               MesssageKind::Create,
                                               mechtron_common::message::From{ tron: context.id.clone(), cycle: 0, timestamp },
                                               To{
                                                   tron: context.id.clone(),
                                                   port: "create".to_string(),
                                                   cycle: Cycle::Next,
                                                   phase: 0,
                                                   inter_delivery_type: InterDeliveryType::Cyclic
                                               },
                                               Payload{ buffer: Arc::new(neutron_create_buffer),
                                                   artifact: sim_artifact.clone()});


        let neutron_content_artifact = neutron_config.content.unwrap().artifact;
        let mut content = Content::new(&SYS.local.configs, neutron_content_artifact.clone() );

        let neutron = TronShell::new( init_tron_of_kind("neutron", &context )? );
        neutron.create(&context,&mut content,&create);

        let content_key = ContentKey {
            tron_id: context.id,
            revision: revision
        };

        self.content.intake(content,content_key)?;

        // now we send a request to the neutron to create the simtron

        let simtron_config = SYS.local.configs.core_tron_config("trons/sim")?;

        let sim_create_artifact = sim_config.messages.unwrap().create.unwrap().artifact;
        let sim_create_buffer_factory = SYS.local.configs.buffer_factory_keeper.get(&sim_create_artifact)?;
        let sim_create_buffer = sim_create_buffer_factory.new_buffer(Option::None );


        let message = Message::single_payload( &mut SYS.net.id_seq,
                     MesssageKind::Create,
                     mechtron_common::message::From{ tron: context.id.clone(), cycle: 0, timestamp },
                         To::basic(
                              neutron_key.clone(),
                              "create-simtron".to_string(),
                         ),
                         Payload{ buffer: Arc::new(sim_create_buffer),
                          artifact: sim_artifact.clone()});


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
        if from.cycle != to.cycle-1
        {
            return Err("cycles must be sequential. 'from' revision cycle must be exactly 1 less than 'to' revision cycle".into());
        }

        Ok(())
    }
}