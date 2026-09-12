//! Port of `three.js/test/unit/src/animation/PropertyBinding.tests.js`.
//!
//! Only the pure-string tests are ported: `sanitizeNodeName` and
//! `parseTrackName`. See the `// SKIPPED:` notes below for the rest.

use three_rs::animation::property_binding as pb;
use three_rs::animation::property_binding::{ParsedTrackName, ParseTrackNameError};

// SKIPPED: `Instancing` — needs a real scene graph (`BoxGeometry` /
// `MeshBasicMaterial` / `Mesh`) and the `PropertyBinding` constructor, which is
// deferred until the `Object3D` tree lands.

#[test]
fn sanitize_node_name() {
    assert_eq!(
        pb::sanitize_node_name("valid-name-123_"),
        "valid-name-123_",
        "Leaves valid name intact."
    );

    assert_eq!(
        pb::sanitize_node_name("急須"),
        "急須",
        "Leaves non-latin unicode characters intact."
    );

    assert_eq!(
        pb::sanitize_node_name("space separated name 123_ -"),
        "space_separated_name_123__-",
        "Replaces spaces with underscores."
    );

    assert_eq!(
        pb::sanitize_node_name("\"Mátyás\" %_* 😇"),
        "\"Mátyás\"_%_*_😇",
        "Allows various punctuation and symbols."
    );

    assert_eq!(
        pb::sanitize_node_name("/invalid: name ^123.[_]"),
        "invalid_name_^123_",
        "Strips reserved characters."
    );
}

/// Spells out a [`ParsedTrackName`] the way the QUnit fixtures do.
fn parsed(
    node_name: Option<&str>,
    object_name: Option<&str>,
    object_index: Option<&str>,
    property_name: &str,
    property_index: Option<&str>,
) -> ParsedTrackName {
    ParsedTrackName {
        node_name: node_name.map(String::from),
        object_name: object_name.map(String::from),
        object_index: object_index.map(String::from),
        property_name: property_name.to_string(),
        property_index: property_index.map(String::from),
    }
}

#[test]
fn parse_track_name() {
    #[allow(clippy::type_complexity)]
    let paths: Vec<(&str, ParsedTrackName)> = vec![
        (".property", parsed(None, None, None, "property", None)),
        (
            "nodeName.property",
            parsed(Some("nodeName"), None, None, "property", None),
        ),
        ("a.property", parsed(Some("a"), None, None, "property", None)),
        (
            "no.de.Name.property",
            parsed(Some("no.de.Name"), None, None, "property", None),
        ),
        (
            "no.d-e.Name.property",
            parsed(Some("no.d-e.Name"), None, None, "property", None),
        ),
        (
            "nodeName.property[accessor]",
            parsed(Some("nodeName"), None, None, "property", Some("accessor")),
        ),
        (
            "nodeName.material.property[accessor]",
            parsed(
                Some("nodeName"),
                Some("material"),
                None,
                "property",
                Some("accessor"),
            ),
        ),
        (
            "no.de.Name.material.property",
            parsed(
                Some("no.de.Name"),
                Some("material"),
                None,
                "property",
                None,
            ),
        ),
        (
            "no.de.Name.material[materialIndex].property",
            parsed(
                Some("no.de.Name"),
                Some("material"),
                Some("materialIndex"),
                "property",
                None,
            ),
        ),
        (
            "uuid.property[accessor]",
            parsed(Some("uuid"), None, None, "property", Some("accessor")),
        ),
        (
            "uuid.objectName[objectIndex].propertyName[propertyIndex]",
            parsed(
                Some("uuid"),
                Some("objectName"),
                Some("objectIndex"),
                "propertyName",
                Some("propertyIndex"),
            ),
        ),
        (
            // directoryName is currently unused.
            "parentName/nodeName.property",
            parsed(Some("nodeName"), None, None, "property", None),
        ),
        (
            // directoryName is currently unused.
            "parentName/no.de.Name.property",
            parsed(Some("no.de.Name"), None, None, "property", None),
        ),
        (
            // directoryName is currently unused.
            "parentName/parentName/nodeName.property[index]",
            parsed(Some("nodeName"), None, None, "property", Some("index")),
        ),
        (
            ".bone[Armature.DEF_cog].position",
            parsed(
                None,
                Some("bone"),
                Some("Armature.DEF_cog"),
                "position",
                None,
            ),
        ),
        (
            "scene:helium_balloon_model:helium_balloon_model.position",
            parsed(
                Some("helium_balloon_model"),
                None,
                None,
                "position",
                None,
            ),
        ),
        (
            "急須.材料[零]",
            parsed(Some("急須"), None, None, "材料", Some("零")),
        ),
        (
            "📦.🎨[🔴]",
            parsed(Some("📦"), None, None, "🎨", Some("🔴")),
        ),
    ];

    for (path, expected) in paths {
        assert_eq!(
            pb::parse_track_name(path).unwrap(),
            expected,
            "Parses track name: {path}"
        );
    }
}

/// Not a QUnit test: covers the `throw new Error( 'THREE.PropertyBinding:
/// Cannot parse trackName: ...' )` branch, which Three's suite never exercises.
#[test]
fn parse_track_name_errors() {
    for path in ["", "property", "nodeName.", "[accessor]", "nodeName/"] {
        assert_eq!(
            pb::parse_track_name(path),
            Err(ParseTrackNameError::CannotParse(path.to_string())),
            "Cannot parse track name: {path}"
        );
    }

    assert_eq!(
        pb::parse_track_name("no.de.Name.").unwrap_err().to_string(),
        "THREE.PropertyBinding: Cannot parse trackName: no.de.Name."
    );
}

// SKIPPED: `setValue` — needs a real `Mesh` / `MeshBasicMaterial` and the
// binding machinery (`bind()` / `setValue()`), which is deferred until the
// `Object3D` tree lands.
