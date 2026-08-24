use std::{
    io::{BufRead, BufReader, Write},
    process::{Command, Stdio},
};

fn main() {
    let mut stdout = std::io::BufWriter::new(std::io::stdout());
    writeln!(stdout, "READY").expect("announce fixture readiness");
    stdout.flush().expect("flush fixture readiness");

    let stdin = std::io::stdin();
    let mut child = None;
    for line in BufReader::new(stdin.lock()).lines().map_while(Result::ok) {
        match line.trim() {
            "start" if child.is_none() => {
                let process = Command::new(if cfg!(windows) { "ping.exe" } else { "sleep" })
                    .args(if cfg!(windows) {
                        vec!["127.0.0.1", "-t"]
                    } else {
                        vec!["600"]
                    })
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()
                    .expect("spawn fixture child");
                child = Some(process);
            }
            "stop" => {
                if let Some(mut process) = child.take() {
                    let _ = process.kill();
                    let _ = process.wait();
                }
                break;
            }
            _ => {}
        }
    }
}
