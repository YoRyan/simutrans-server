use std::fs;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::{Command, exit};
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::{Duration, SystemTime};

use clap::Parser;
use log::{error, info, log_enabled};
use shared_child::{SharedChild, unix::SharedChildExt};
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
    /// Set simutrans debug level (to set the log level for this wrapper, use the RUST_LOG environment variable)
    #[arg(long, default_value_t = 1)]
    debug: u32,
    /// Pass arguments directly to simutrans.
    #[arg(trailing_var_arg = true)]
    args: Option<Vec<String>>,
}

fn main() {
    let args = Cli::parse();
    env_logger::init();

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
        if let Err(err) = copy_save_to_game(&args) {
            error!("Error copying save to simutrans: {}", err);
        }

        let child_arc: Arc<SharedChild>;
        {
            let mut cmd = Command::new(args.simutrans.join("simutrans"));
            cmd.arg("-singleuser");
            cmd.arg("-server");
            cmd.args(["-objects", "pak"]);
            cmd.args(["-load", "network"]);
            cmd.args(["-debug", &args.debug.to_string().as_str()]);

            if let Some(a) = &args.args {
                cmd.args(a.iter().map(|s| s.as_str()));
            }

            if log_enabled!(log::Level::Info) {
                let cmd_args: Vec<&str> = cmd
                    .get_args()
                    .into_iter()
                    .map(|os| os.to_str().unwrap_or_default())
                    .collect();
                let log_args = cmd_args.join(" ");
                info!("Starting simutrans with args: {}", log_args);
            }

            let shared_child = match SharedChild::spawn(&mut cmd) {
                Ok(child) => child,
                Err(err) => {
                    panic!("Unable to start simutrans: {}", err);
                }
            };
            child_arc = Arc::new(shared_child);
        }

        // Spawn a thread to wait until simutrans exits.
        let child_arc_clone = child_arc.clone();
        let wait_op_s = loop_op_s.clone();
        let wait_thread = thread::spawn(move || {
            child_arc_clone.wait().unwrap();
            // We may need to wake the main (kill) thread if simutrans exits
            // before any kill was signaled.
            // Don't block in case another source already sent some other value
            // and there is nobody listening to consume ours.
            let _ = wait_op_s.try_send(Operation::Wakeup);
        });

        let received = &op_r.recv().unwrap();
        info!(
            "{}",
            match received {
                Operation::Reload => "Received request to restart Simutrans...",
                Operation::Stop => "Received request to kill Simutrans and exit...",
                Operation::Wakeup => "Simutrans exited, restarting...",
            }
        );
        let _ = child_arc.send_signal(signal_hook::consts::SIGTERM);
        wait_thread.join().unwrap();

        if let Err(err) = copy_game_to_save(&args) {
            error!("Error copying simutrans autosave: {}", err);
        }

        match received {
            Operation::Reload | Operation::Wakeup => {
                // Ensure the message queue is completely drained, so that we
                // don't wake ourselves up right away on the next iteration.
                while let Ok(_) = &op_r.try_recv() {}
                continue;
            }
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
    let dest_dir = &args.simutrans.join("save");
    let _ = mkdir(dest_dir);
    copy_file(src_save, &dest_dir.join("network.sve"))?;
    copy_file(
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

    copy_file_if_newer(src_save, &args.save.join("network.sve"))?;
    copy_file_if_newer(
        &args.simutrans.join("server13353-pwdhash.sve"),
        &args.save.join("pwdhash.sve"),
    )?;
    Ok(())
}

fn mkdir(dir: &Path) -> Result<()> {
    info!("Mkdir {:?}", dir);
    fs::create_dir(dir)?;
    Ok(())
}

fn copy_file_if_newer(src: &Path, dest: &Path) -> Result<()> {
    let do_copy: bool = match (mod_time(src), mod_time(dest)) {
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
    info!("Copying {:?} -> {:?}", src, dest);
    let data = fs::read(src)?;
    fs::write(dest, data)?;
    Ok(())
}

fn mod_time(path: &Path) -> Result<SystemTime> {
    let t = File::open(path)?.metadata()?.modified()?;
    Ok(t)
}
