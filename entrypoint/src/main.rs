use std::fs;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::{Command, exit};
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::{Duration, SystemTime};

use clap::Parser;
use shared_child::SharedChild;
use signal_hook::iterator::Signals;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

enum Operation {
    Reload,
    Stop,
    Wakeup,
}

#[derive(Parser)]
struct Cli {
    /// Path to the single-user install of simutrans.
    #[arg(long, default_value_os_t = PathBuf::from("/game"))]
    simutrans: PathBuf,
    /// Path to the directory containing the target save data.
    #[arg(long, default_value_os_t = PathBuf::from("/save"))]
    save: PathBuf,
    /// Kill simutrans periodically to force a save (set to 0 to disable)
    #[arg(long, default_value_t = 120)]
    reload_mins: u64,
    /// Print debug messages to stderr.
    #[arg(long, default_value_t = false)]
    verbose: bool,
}

fn main() {
    let args = Cli::parse();
    let (op_s, op_r) = mpsc::sync_channel::<Operation>(1);
    let timer_op_s = op_s.clone();
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

    // Spawn a thread to periodically kill simutrans and force a save.
    if args.reload_mins > 0 {
        thread::spawn(move || {
            loop {
                thread::sleep(Duration::from_mins(args.reload_mins));
                let _ = timer_op_s.try_send(Operation::Reload);
            }
        });
    }

    loop {
        let _ = copy_save_to_game(&args);

        let mut cmd = Command::new(args.simutrans.join("simutrans"));
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
        log(&args, || {
            let cmd_args: Vec<&str> = cmd
                .get_args()
                .into_iter()
                .map(|os| os.to_str().unwrap_or_default())
                .collect();
            format!("Starting simutrans with args: {}", cmd_args.join(" "))
        });
        let child_arc = Arc::new(shared_child);

        // Spawn a thread to wait until simutrans exits.
        let child_arc_clone = child_arc.clone();
        let wait_op_s = loop_op_s.clone();
        let wait_thread = thread::spawn(move || {
            child_arc_clone.wait().unwrap();
            // We may need to wake the main (kill) thread if simutrans exits
            // before any kill was attempted.
            // Don't block in case another source already sent some other value
            // and there is nobody listening to consume ours.
            let _ = wait_op_s.try_send(Operation::Wakeup);
        });

        let received = &op_r.recv().unwrap();
        log(&args, || {
            match received {
                Operation::Reload => "Received request to restart Simutrans...",
                Operation::Stop => "Received request to kill Simutrans and exit...",
                Operation::Wakeup => "Simutrans exited, restarting...",
            }
            .to_owned()
        });
        let _ = child_arc.kill();
        wait_thread.join().unwrap();

        let _ = copy_game_to_save(&args);

        match received {
            Operation::Reload | Operation::Wakeup => continue,
            Operation::Stop => break,
        }
    }

    exit(0);
}

fn copy_save_to_game(args: &Cli) -> Result<()> {
    let src_save = &args.save.join("network.sve");
    match mod_time(src_save) {
        Ok(_) => {}
        Err(_) => {
            panic!(
                "Expected save file at {:?}, but it does not exist.",
                src_save
            );
        }
    }
    copy_file(
        args,
        src_save,
        &args.simutrans.join("server13353-network.sve"),
    )?;
    copy_file(
        args,
        &args.save.join("pwdhash.sve"),
        &args.simutrans.join("server13353-pwdhash.sve"),
    )?;
    Ok(())
}

fn copy_game_to_save(args: &Cli) -> Result<()> {
    let kill_save = &args.simutrans.join("server13353-restore.sve");
    let join_save = &args.simutrans.join("server13353-network.sve");
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

    copy_file_if_newer(args, src_save, &args.save.join("network.sve"))?;
    copy_file_if_newer(
        args,
        &args.simutrans.join("server13353-pwdhash.sve"),
        &args.save.join("pwdhash.sve"),
    )?;
    Ok(())
}

fn copy_file_if_newer(args: &Cli, src: &Path, dest: &Path) -> Result<()> {
    let do_copy: bool = match (mod_time(src), mod_time(dest)) {
        (Ok(src_t), Ok(dest_t)) => src_t > dest_t,
        (Ok(_), _) => true,
        _ => false,
    };
    if do_copy {
        copy_file(args, src, dest)
    } else {
        Ok(())
    }
}

fn copy_file(args: &Cli, src: &Path, dest: &Path) -> Result<()> {
    log(args, || format!("Copying {:?} -> {:?}", src, dest));
    let data = fs::read(src)?;
    fs::write(dest, data)?;
    Ok(())
}

fn mod_time(path: &Path) -> Result<SystemTime> {
    let t = File::open(path)?.metadata()?.modified()?;
    Ok(t)
}

fn log<F>(args: &Cli, msg: F)
where
    F: Fn() -> String,
{
    if args.verbose {
        eprintln!("<*> {}", msg());
    }
}
