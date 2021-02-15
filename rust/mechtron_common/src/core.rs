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
pub static ref CORE_SCHEMA_EMPTY: Artifact = CORE.path(&"schema/empty.json");
pub static ref CORE_SCHEMA_NUCLEUS_LOOKUP_NAME_MESSAGE: Artifact = CORE.path(&"schema/neutron/nucleus_lookup_name_message.schema");


pub static ref CORE_NEUTRON_CONFIG: Artifact = CORE.path(&"tron/neutron.yaml");
pub static ref CORE_SIMTRON_CONFIG: Artifact = CORE.path(&"tron/sim.yaml");

}