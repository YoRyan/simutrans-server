use std::fs;
use std::fs::File;
use std::path::Path;
use std::process::{Command, exit};
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::SystemTime;

use shared_child::SharedChild;
use signal_hook::iterator::Signals;

static GAME_PATH: &str = "/game";
static SAVE_PATH: &str = "/save";

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

enum Operation {
    Reload,
    Stop,
    Wakeup,
}

fn main() {
    let game_path = Path::new(GAME_PATH);
    let exe = &game_path.join("simutrans");
    let (op_s, op_r) = mpsc::sync_channel::<Operation>(1);
    let loop_op_s = op_s.clone();

    // Spawn a thread to listen for commands sent via Linux signals.
    let mut signals = Signals::new([
        signal_hook::consts::SIGINT,
        signal_hook::consts::SIGTERM,
        signal_hook::consts::SIGUSR1,
    ])
    .unwrap();
    thread::spawn(move || {
        for sig in signals.forever() {
            let _ = op_s.try_send(if sig == signal_hook::consts::SIGUSR1 {
                Operation::Reload
            } else {
                Operation::Stop
            });
        }
    });

    loop {
        let _ = copy_save_to_game();

        let mut cmd = Command::new(exe);
        cmd.arg("-singleuser");
        cmd.arg("-server");
        cmd.args(["-objects", "pak"]);
        cmd.args(["-debug", "1"]);
        let shared_child = match SharedChild::spawn(&mut cmd) {
            Ok(child) => child,
            Err(err) => {
                panic!("Unable to start simutrans: {}", err);
            }
        };
        let child_arc = Arc::new(shared_child);

        // Spawn a thread to wait until simutrans exits.
        let child_arc_clone = child_arc.clone();
        let wait_op_s = loop_op_s.clone();
        let wait_thread = thread::spawn(move || {
            child_arc_clone.wait().unwrap();
            // We need to wake the main (kill) thread if simutrans exits before
            // any kill was attempted.
            let _ = wait_op_s.send(Operation::Wakeup);
        });

        let received = &op_r.recv().unwrap();
        eprintln!(
            "<*> {}",
            match received {
                Operation::Reload => "Received request to restart Simutrans...",
                Operation::Stop => "Received request to kill Simutrans and exit...",
                Operation::Wakeup => "Simutrans exited, restarting...",
            }
        );
        match received {
            // Kill simutrans when requested.
            Operation::Reload | Operation::Stop => {
                child_arc.kill().unwrap();
                // Consume the waiter thread's signal so we won't deadlock when
                // we join that thread.
                let _ = &op_r.recv();
            }
            // It's already dead...
            Operation::Wakeup => {}
        };
        wait_thread.join().unwrap();

        let _ = copy_game_to_save();

        match received {
            Operation::Reload | Operation::Wakeup => continue,
            Operation::Stop => break,
        }
    }

    exit(0);
}

fn copy_save_to_game() -> Result<()> {
    let save_path = Path::new(SAVE_PATH);
    let game_path = Path::new(GAME_PATH);
    copy_file(
        &save_path.join("network.sve"),
        &game_path.join("server13353-network.sve"),
    )?;
    copy_file(
        &save_path.join("pwdhash.sve"),
        &game_path.join("server13353-pwdhash.sve"),
    )?;
    Ok(())
}

fn copy_game_to_save() -> Result<()> {
    let save_path = Path::new(SAVE_PATH);
    let game_path = Path::new(GAME_PATH);

    let kill_save = &game_path.join("server13353-restore.sve");
    let join_save = &game_path.join("server13353-network.sve");
    let src_save = match (mod_time(&kill_save), mod_time(&join_save)) {
        (Ok(kill_t), Ok(join_t)) => {
            if kill_t > join_t {
                kill_save
            } else {
                join_save
            }
        }
        (Ok(_), _) => kill_save,
        (_, Ok(_)) => join_save,
        (_, _) => return Ok(()),
    };

    copy_file_if_newer(src_save, &save_path.join("network.sve"))?;
    copy_file_if_newer(
        &game_path.join("server13353-pwdhash.sve"),
        &save_path.join("pwdhash.sve"),
    )?;
    Ok(())
}

fn copy_file_if_newer(src: &Path, dest: &Path) -> Result<()> {
    let do_copy = match (mod_time(src), mod_time(dest)) {
        (Ok(src_t), Ok(dest_t)) => src_t > dest_t,
        (Ok(_), _) => true,
        _ => false,
    };
    if do_copy {
        copy_file(src, dest)
    } else {
        Ok(())
    }
}

fn copy_file(src: &Path, dest: &Path) -> Result<()> {
    let data = fs::read(src)?;
    fs::write(dest, data)?;
    Ok(())
}

fn mod_time(path: &Path) -> Result<SystemTime> {
    let t = File::open(path)?.metadata()?.modified()?;
    Ok(t)
}
