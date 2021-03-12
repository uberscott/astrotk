

#[cfg(test)]
    use crate::artifact::MechtroniumArtifactRepository;
    use crate::mechtron_common::configs::Configs;
    use crate::mechtron_common::core::*;
    use std::sync::Arc;

    pub fn create_configs()->Configs
    {
        let repo = MechtroniumArtifactRepository::new("../../repo");
        let mut configs = Configs::new(Arc::new(repo) );

//        configs.cache_common();
        configs
    }


