use crate::artifact::{Artifact, ArtifactBundle, ArtifactCache, ArtifactRepository, ArtifactYaml};
use semver::Version;

lazy_static! {
pub static ref CORE: ArtifactBundle = ArtifactBundle
    {
        group: "mechtron.io".to_string(),
        id: "core".to_string(),
        version: Version::new( 1, 0, 0 )
    };

pub static ref CORE_CONTENT_META: Artifact = CORE.path(&"schema/content/meta.json");
pub static ref CORE_SCHEMAT_EMPTY: Artifact = CORE.path(&"schema/empty.json");
}