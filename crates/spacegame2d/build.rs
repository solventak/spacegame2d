use std::process::Command;

fn main() {
    let status = Command::new("npm")
        .args(["run", "build"])
        .current_dir("hud")
        .status()
        .expect("failed to start npm; install Node.js and npm to build the HUD");

    assert!(status.success(), "HUD build failed with status {status}");
}
