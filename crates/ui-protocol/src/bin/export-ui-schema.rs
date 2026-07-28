use std::{env, fs, path::PathBuf};

fn main() {
    let output = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
        .join("schema/ui-engine-ipc.v1.schema.json");
    let rendered = format!(
        "{}\n",
        serde_json::to_string_pretty(&spacegame2d_ui_protocol::schema_bundle()).unwrap()
    );
    if env::args().any(|arg| arg == "--check") {
        let current = fs::read_to_string(&output).unwrap_or_default();
        if current != rendered {
            panic!(
                "{} is stale; run cargo run -p spacegame2d-ui-protocol --bin export-ui-schema",
                output.display()
            );
        }
    } else {
        fs::create_dir_all(output.parent().unwrap()).unwrap();
        fs::write(output, rendered).unwrap();
    }
}
