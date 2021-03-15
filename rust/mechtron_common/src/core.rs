use crate::artifact::{Artifact, ArtifactBundle, ArtifactCache, ArtifactRepository, ArtifactYaml};
use semver::Version;

lazy_static! {
    pub static ref CORE: ArtifactBundle = ArtifactBundle {
        group: "mechtron.io".to_string(),
        id: "core".to_string(),
        version: Version::new(1, 0, 0)
    };

    pub static ref STD: ArtifactBundle = ArtifactBundle {
        group: "mechtron.io".to_string(),
        id: "std".to_string(),
        version: Version::new(1, 0, 0)
    };

    pub static ref CORE_SCHEMA_MESSAGE: Artifact = CORE.path_and_kind(&"schema/message/message.schema", "schema");
    pub static ref CORE_SCHEMA_MESSAGE_BUILDERS: Artifact = CORE.path_and_kind(&"schema/message/message-builders.schema", "schema");

    pub static ref CORE_SCHEMA_STATE: Artifact = CORE.path_and_kind(&"schema/state/state.schema", "schema");
    pub static ref CORE_SCHEMA_META_STATE: Artifact = CORE.path_and_kind(&"schema/mechtron/state-meta.schema", "schema");
    pub static ref CORE_SCHEMA_META_CREATE : Artifact = CORE.path_and_kind(&"schema/mechtron/create-meta.schema", "schema");
    pub static ref CORE_SCHEMA_META_API: Artifact = CORE.path_and_kind(&"schema/api/meta.schema", "schema");

    pub static ref CORE_SCHEMA_EMPTY: Artifact = CORE.path_and_kind(&"schema/empty.schema", "schema");
    pub static ref CORE_SCHEMA_NUCLEUS_LOOKUP_NAME_MESSAGE: Artifact = CORE.path_and_kind(&"schema/neutron/nucleus_lookup_name_message.schema", "schema");

    pub static ref CORE_SCHEMA_NEUTRON_CREATE: Artifact = CORE.path_and_kind(&"schema/neutron/create.schema", "schema");
    pub static ref CORE_SCHEMA_NEUTRON_STATE : Artifact = CORE.path_and_kind(&"schema/neutron/state.schema", "schema");

    pub static ref CORE_SCHEMA_PING: Artifact = CORE.path_and_kind(&"schema/util/ping.schema", "schema");
    pub static ref CORE_SCHEMA_PONG: Artifact = CORE.path_and_kind(&"schema/util/pong.schema", "schema");
    pub static ref CORE_SCHEMA_TEXT : Artifact = CORE.path_and_kind(&"schema/util/text.schema", "schema");
    pub static ref CORE_SCHEMA_ARTIFACT: Artifact = CORE.path_and_kind(&"schema/util/artifact.schema", "schema");
    pub static ref CORE_SCHEMA_OK : Artifact = CORE.path_and_kind(&"schema/util/ok.schema", "schema");
    pub static ref CORE_SCHEMA_MECHTRON_CONTEXT : Artifact = CORE.path_and_kind(&"schema/mechtron/context.schema", "schema");
    pub static ref CORE_SCHEMA_TYPE_ID: Artifact = CORE.path_and_kind(&"schema/util/type_id.schema", "schema");
    pub static ref CORE_SCHEMA_TYPE_ID_VALUE: Artifact = CORE.path_and_kind(&"schema/util/type_id_value.schema", "schema");

    pub static ref CORE_NUCLEUS_SIMULATION: Artifact = CORE.path_and_kind(&"nucleus/sim.yaml", "nucleus");

    pub static ref CORE_BIND_NEUTRON: Artifact = CORE.path_and_kind(&"bind/neutron.yaml", "bind");
    pub static ref CORE_BIND_SIMTRON: Artifact = CORE.path_and_kind(&"bind/simtron.yaml", "bind");

    pub static ref CORE_MECHTRON_SIMTRON: Artifact = CORE.path_and_kind(&"mechtron/simtron.yaml", "mechtron");
    pub static ref CORE_MECHTRON_NEUTRON: Artifact = CORE.path_and_kind(&"mechtron/neutron.yaml", "mechtron");



    pub static ref STD_SCHEMA_KEY_VALUE_ID: Artifact = STD.path_and_kind(&"schema/util/key_value_id.yaml", "schema");
    pub static ref STD_SCHEMA_ID: Artifact = STD.path_and_kind(&"schema/util/id.yaml", "schema");

}

