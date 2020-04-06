use std::process::Command;

fn main() {
    // get the git short rev so we can embed it in the web page
    let version = Command::new("git")
        .args(&["rev-parse", "--short", "HEAD"])
        .output()
        .expect("failed to get git version");

    // this is how you pass an env var to Cargo at build time:
    // https://doc.rust-lang.org/cargo/reference/build-scripts.html#rustc-env
    println!(
        "cargo:rustc-env={}={}",
        "RAT_VERSION",
        std::str::from_utf8(&version.stdout).unwrap()
    );
}
