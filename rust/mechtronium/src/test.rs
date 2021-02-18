

#[cfg(test)]
    use crate::artifact::FileSystemArtifactRepository;
    use crate::mechtron_core::configs::Configs;
    use crate::mechtron_core::core::*;
    use std::sync::Arc;

    pub fn create_configs<'a>()->Configs<'a>
    {
        let repo = FileSystemArtifactRepository::new("../../repo");
        let mut configs = Configs::new(Arc::new(repo) );


        configs.artifacts.cache(&CORE_SCHEMA_EMPTY).unwrap();
        configs.artifacts.cache(&CORE_STATE_META).unwrap();
        configs.schemas.cache(&CORE_SCHEMA_EMPTY).unwrap();
        configs.schemas.cache(&CORE_STATE_META ).unwrap();

        configs.artifacts.get(&CORE_STATE_META );
        configs
    }


