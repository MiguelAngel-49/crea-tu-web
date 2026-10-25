//! Smoke: spawn docker many times with CREATE_NO_WINDOW (mirrors app helper).
//! Run: cargo run --example silent_spawn_smoke
use std::process::Command;
use std::time::Instant;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

fn docker_silent(args: &[&str]) -> std::io::Result<std::process::Output> {
    let mut cmd = Command::new("docker");
    #[cfg(windows)]
    {
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd.args(args).output()
}

fn main() {
    let n = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(25usize);
    let site = std::env::args().nth(2);
    let start = Instant::now();
    let mut ok = 0usize;
    for i in 0..n {
        let out = docker_silent(&["compose", "version", "--short"]).expect("docker spawn");
        if out.status.success() {
            ok += 1;
        } else {
            eprintln!("iter {i}: {:?}", String::from_utf8_lossy(&out.stderr));
        }
    }
    if let Some(path) = site {
        println!("wpcli is-installed in {path}…");
        let mut cmd = Command::new("docker");
        #[cfg(windows)]
        {
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
        let out = cmd
            .args(["compose", "run", "--rm", "wpcli", "core", "is-installed"])
            .current_dir(&path)
            .output()
            .expect("wpcli");
        println!(
            "wpcli exit={:?} stderr_len={}",
            out.status.code(),
            out.stderr.len()
        );
    }
    println!(
        "silent_spawn_smoke: {ok}/{n} ok in {:.2}s (CREATE_NO_WINDOW on Windows)",
        start.elapsed().as_secs_f64()
    );
}
