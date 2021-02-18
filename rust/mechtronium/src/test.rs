

#[cfg(test)]
    use crate::artifact::FileSystemArtifactRepository;
    use crate::mechtron_core::configs::Configs;
    use crate::mechtron_core::core::*;
    use std::sync::Arc;

    pub fn create_configs<'a>()->Configs<'a>
    {
        let repo = FileSystemArtifactRepository::new("../../repo");
        let mut configs = Configs::new(Arc::new(repo) );

        configs.cache_core();
        configs
    }


