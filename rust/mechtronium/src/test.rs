

#[cfg(test)]
    use crate::artifact::FileSystemArtifactRepository;
    use crate::mechtron_core::configs::Configs;
    use std::sync::Arc;

    pub fn create_configs<'a>()->Configs<'a>
    {
        let repo = FileSystemArtifactRepository::new("../../repo");
        let configs = Configs::new(Arc::new(repo) );
        configs
    }


