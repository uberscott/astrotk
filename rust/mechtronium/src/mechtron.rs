use mechtron_common::artifact::Artifact;
use mechtron_common::buffers::Buffer;
use mechtron_common::configs::{Configs, MechtronConfig};
use crate::error::Error;
use std::sync::Arc;
use mechtron_common::message::Payload;
use mechtron_common::core::*;

pub struct CreatePayloadsBuilder {
    pub constructor_artifact: Artifact,
    pub meta: Buffer,
    pub constructor: Buffer,
}

impl CreatePayloadsBuilder {
    pub fn new (
        configs: &Configs,
        config: Arc<MechtronConfig>,
    ) -> Result<Self, Error> {

        let meta_factory = configs.schemas.get(&CORE_SCHEMA_META_CREATE)?;
        let mut meta = Buffer::new(meta_factory.new_buffer(Option::None));
        meta.set(&path![&"config"], config.source.to().clone())?;
        let (constructor_artifact, constructor) =
            CreatePayloadsBuilder::constructor(configs, config.clone())?;
        Ok(CreatePayloadsBuilder {
            meta: meta,
            constructor_artifact: constructor_artifact,
            constructor: constructor,
        })
    }

    pub fn set_lookup_name(&mut self, lookup_name: &str) -> Result<(), Error> {
        self.meta.set(&path![&"lookup_name"], lookup_name)?;
        Ok(())
    }

    pub fn set_config(&mut self, config: &MechtronConfig) -> Result<(), Error> {
        self.constructor
            .set(&path!["config"], config.source.to())?;
        Ok(())
    }

    fn constructor(
        configs: &Configs,
        config: Arc<MechtronConfig>,
    ) -> Result<(Artifact, Buffer), Error> {
            let bind = configs.binds.get( &config.bind.artifact )?;
            let constructor_artifact = bind.message.create.artifact.clone();
            let factory = configs.schemas.get(&constructor_artifact)?;
            let constructor = factory.new_buffer(Option::None);
            let constructor = Buffer::new(constructor);

            Ok((constructor_artifact, constructor))
    }

    pub fn payloads<'configs>(configs: &'configs Configs, builder: CreatePayloadsBuilder) -> Vec<Payload> {
        let meta_artifact = CORE_SCHEMA_META_CREATE.clone();
        vec![
            Payload {
                schema: meta_artifact,
                buffer: builder.meta.read_only(),
            },
            Payload {
                schema: builder.constructor_artifact,
                buffer: builder.constructor.read_only(),
            },
        ]
    }

    pub fn payloads_builders( builder: CreatePayloadsBuilder) -> Vec<Payload> {
        let meta_artifact = CORE_SCHEMA_META_CREATE.clone();
        vec![
            Payload{
                schema: meta_artifact,
                buffer: builder.meta.read_only(),
            },
            Payload{
                schema: builder.constructor_artifact,
                buffer: builder.constructor.read_only(),
            },
        ]
    }
}





