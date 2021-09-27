use insta::assert_snapshot;
use serde::Serialize;
use serde_kdl::to_string;
use std::collections::BTreeMap;

#[test]
fn cargo() {
    //! https://github.com/kdl-org/kdl/blob/main/examples/Cargo.kdl
    //! LICENSE: CC BY-SA 4.0.

    #[derive(Debug, Serialize)]
    struct CargoManifest {
        package: CargoPackage,
        dependencies: BTreeMap<String, String>,
    }

    #[derive(Debug, Serialize)]
    struct CargoPackage {
        name: String,
        version: String,
        edition: String,
        authors: Vec<String>,
        description: String,
        #[serde(rename = "license-file")]
        license_file: String,
    }

    let manifest = CargoManifest {
        package: CargoPackage {
            name: "kdl".to_string(),
            version: "0.0.0".to_string(),
            description: "kat's document language".to_string(),
            authors: vec!["Kat Marchán <kzm@zkat.tech>".to_string()],
            license_file: "LICENSE.md".to_string(),
            edition: "2018".to_string(),
        },
        dependencies: [
            ("nom".to_string(), "6.0.1".to_string()),
            ("thiserror".to_string(), "1.0.22".to_string()),
        ]
        .into_iter()
        .collect(),
    };

    assert_snapshot!(&to_string(&manifest).unwrap(), @r###"

    package name=r"kdl" version=r"0.0.0" edition=r"2018" {
        authors r"Kat Marchán <kzm@zkat.tech>"
        description r"kat's document language"
        license-file r"LICENSE.md"
    }
    dependencies {
        - key=r"nom" value=r"6.0.1"
        - key=r"thiserror" value=r"1.0.22"
    }
    "###);
}
