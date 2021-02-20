use crate::artifact::{Artifact, ArtifactBundle, ArtifactCache, ArtifactRepository, ArtifactYaml};
use semver::Version;

lazy_static! {
    pub static ref CORE: ArtifactBundle = ArtifactBundle {
        group: "mechtron.io".to_string(),
        id: "core".to_string(),
        version: Version::new(1, 0, 0)
    };
    pub static ref CORE_SCHEMA_META_STATE: Artifact = CORE.path_and_kind(&"schema/tron/state-meta.schema", "schema");
    pub static ref CORE_SCHEMA_META_CREATE : Artifact = CORE.path_and_kind(&"schema/tron/create-meta.schema", "schema");

    pub static ref CORE_SCHEMA_EMPTY: Artifact = CORE.path_and_kind(&"schema/empty.schema", "schema");
    pub static ref CORE_SCHEMA_NUCLEUS_LOOKUP_NAME_MESSAGE: Artifact = CORE.path_and_kind(&"schema/neutron/nucleus_lookup_name_message.schema", "schema");

    pub static ref CORE_SCHEMA_NEUTRON_CREATE: Artifact = CORE.path_and_kind(&"schema/neutron/create.schema", "schema");
    pub static ref CORE_SCHEMA_NEUTRON_STATE : Artifact = CORE.path_and_kind(&"schema/neutron/state.schema", "schema");

    pub static ref CORE_TRONCONFIG_NEUTRON: Artifact = CORE.path_and_kind(&"tron/neutron.yaml", "tron_config");
    pub static ref CORE_TRONCONFIG_SIMTRON: Artifact = CORE.path_and_kind(&"tron/sim.yaml", "tron_config");

    pub static ref CORE_SCHEMA_PING: Artifact = CORE.path_and_kind(&"schema/util/ping.schema", "schema");
    pub static ref CORE_SCHEMA_PONG: Artifact = CORE.path_and_kind(&"schema/util/pong.schema", "schema");
}

