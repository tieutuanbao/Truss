use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn inward_layers_do_not_import_outward_layers_or_frameworks() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    assert_forbidden(
        &root.join("domain"),
        &[
            "crate::application",
            "crate::infrastructure",
            "crate::interface",
            "serde",
            "clap",
            "fs2",
            "std::fs",
            "std::process",
        ],
    );
    assert_forbidden(
        &root.join("application"),
        &[
            "crate::infrastructure",
            "crate::interface",
            "serde",
            "clap",
            "fs2",
            "std::fs",
            "std::process",
        ],
    );
    assert_forbidden(&root.join("infrastructure"), &["crate::interface"]);
    assert_forbidden(&root.join("interface"), &["crate::infrastructure"]);
    // The interface maps arguments and presents reports: it reaches the
    // application facade and never opens a state, session, or manifest file.
    assert_forbidden(&root.join("interface"), &["std::fs"]);
}

#[test]
fn composition_root_is_the_only_layer_wiring_infrastructure_to_interface() {
    let main =
        fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/main.rs")).unwrap();
    assert!(main.contains("truss::infrastructure"));
    assert!(main.contains("truss::interface"));
    assert!(main.contains("CoreApplication::new"));
    assert!(main.contains("SelfUpdateApplication::new"));
    assert!(main.contains("AddOnApplication::new"));
    assert!(main.contains("MigrationApplication::new"));
    assert!(main.contains("FileSystemMigration"));
}

/// REQ-035: the migration boundary is layered, and the adapter names the
/// proven primitives it reuses instead of extending the core transaction.
#[test]
fn migration_boundary_is_layered_and_reuses_the_safe_primitives() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let cases: [(&str, &[&str]); 4] = [
        (
            "domain/migration.rs",
            &[
                "crate::application",
                "crate::infrastructure",
                "crate::interface",
                "serde",
                "clap",
                "fs2",
                "std::fs",
                "std::process",
            ],
        ),
        (
            "application/migration.rs",
            &[
                "crate::infrastructure",
                "crate::interface",
                "serde",
                "clap",
                "fs2",
                "std::fs",
                "std::process",
            ],
        ),
        (
            "application/migration_ports.rs",
            &[
                "crate::infrastructure",
                "crate::interface",
                "serde",
                "clap",
                "fs2",
                "std::fs",
                "std::process",
            ],
        ),
        (
            "infrastructure/filesystem_migration.rs",
            &["crate::interface"],
        ),
    ];
    for (relative, forbidden) in cases {
        let path = root.join(relative);
        assert!(path.is_file(), "{relative} must exist");
        let source = fs::read_to_string(&path).unwrap();
        for pattern in forbidden {
            assert!(
                !source.contains(pattern),
                "{relative} imports forbidden dependency {pattern}"
            );
        }
    }
    let adapter = fs::read_to_string(root.join("infrastructure/filesystem_migration.rs")).unwrap();
    for primitive in [
        "copy_bytes_atomic",
        "hash_bytes",
        "reject_symlink",
        "io_error",
    ] {
        assert!(
            adapter.contains(primitive),
            "the adapter reuses the proven primitive {primitive}"
        );
    }
    assert!(
        !adapter.contains("transaction::run"),
        "migration must not extend the core update transaction"
    );
}

fn assert_forbidden(root: &Path, forbidden: &[&str]) {
    for entry in fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|value| value.to_str()) != Some("rs") {
            continue;
        }
        let source = fs::read_to_string(&path).unwrap();
        for pattern in forbidden {
            assert!(
                !source.contains(pattern),
                "{} imports forbidden dependency {pattern}",
                path.display()
            );
        }
    }
}
