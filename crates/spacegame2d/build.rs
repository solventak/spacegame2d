use std::process::Command;

fn main() {
    let npm = if cfg!(target_os = "windows") {
        "npm.cmd"
    } else {
        "npm"
    };
    let status = Command::new(npm)
        .args(["run", "build"])
        .current_dir("hud")
        .status()
        .expect("failed to start npm; install Node.js and npm to build the HUD");

    assert!(status.success(), "HUD build failed with status {status}");
}
