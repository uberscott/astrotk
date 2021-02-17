

#[cfg(test)]
    use crate::artifact::FileSystemArtifactRepository;
    use crate::mechtron_core::configs::Configs;
    use crate::mechtron_core::core::*;
    use std::sync::Arc;

    pub fn create_configs<'a>()->Configs<'a>
    {
        let repo = FileSystemArtifactRepository::new("../../repo");
        let mut configs = Configs::new(Arc::new(repo) );


        configs.artifact_cache.cache(&CORE_SCHEMA_EMPTY).unwrap();
        configs.artifact_cache.cache(&CORE_STATE_META).unwrap();
        configs.buffer_factory_keeper.cache(&CORE_SCHEMA_EMPTY).unwrap();
        configs.buffer_factory_keeper.cache(&CORE_STATE_META ).unwrap();

        configs.artifact_cache.get(&CORE_STATE_META );
        configs
    }


