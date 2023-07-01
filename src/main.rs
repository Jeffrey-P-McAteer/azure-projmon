
use futures::prelude::*;

use tokio::io::AsyncWriteExt;

#[macro_use]
pub mod macros;
use macros::*;

fn main() {
  let args: Vec<String> = std::env::args().collect();
  let rt = tokio::runtime::Builder::new_multi_thread()
      .enable_all()
      .worker_threads(2)
      .build()
      .expect("Could not build tokio runtime!");

  rt.block_on(async_main(args));
}

const PROJECTOR_DISPLAY_NAMES:  &'static [&'static str] = &[
  "DP1", "DP-1"
];

const PROJ_MON_NAME: &'static str = "PROJMON-1";


async fn async_main(args: Vec<String>) {
  let exit_signal_handler = tokio::task::spawn(handle_exit_signals());

  let mut sway_conn = dump_error_and_ret!( swayipc_async::Connection::new().await );

  let mut connected_projector_name = String::new();
  let mut connected_projector_ws = String::new();
  let mut have_projmon_display = false;

  println!("Waiting for one of {:?} to be connected...", PROJECTOR_DISPLAY_NAMES);
  loop {

    {
      let mut stdout = tokio::io::stdout();
      dump_error!( stdout.write_all(b".").await );
      dump_error!( stdout.flush().await );
    }

    if let Ok(outputs) = sway_conn.get_outputs().await {
      for output in outputs.iter() {
        if !output.active || !output.dpms {
          continue; // output is either inactive or DPMS says it's powered off.
        }
        if PROJECTOR_DISPLAY_NAMES.contains( &output.name.as_str() ) && !output.current_workspace.is_none() {
          connected_projector_name = output.name.clone();
          connected_projector_ws = output.current_workspace.clone().expect("Alread checked we are !.is_none()");
        }
        if PROJ_MON_NAME == output.name.as_str() {
          have_projmon_display = true;
        }
      }
    }

    if connected_projector_name.len() > 1 {
      break; // found projector!
    }

    if EXIT_FLAG.load(std::sync::atomic::Ordering::SeqCst) {
      EXIT_TASKS_DONE.store(true, std::sync::atomic::Ordering::SeqCst);
      return; // no cleanup yet
    }
    

    tokio::time::sleep( std::time::Duration::from_millis(250) ).await;
  }

  println!("");
  println!("Saw connected projector at {:?} with workspace {:?}", &connected_projector_name, &connected_projector_ws);

  // Set black wallpaper + virtual location to -10 screens to the left (sway prevents mouse from jumping! \o/)
  dump_error!( sway_conn.run_command(format!("output {} mode 1920x1080 pos -19200 0 bg #000000 solid_color", &connected_projector_name).as_str()).await );

  // Create new virtual display
  if ! have_projmon_display {
    println!("Telling sway to create {}", PROJ_MON_NAME);
    dump_error!( sway_conn.run_command(format!("create_output {}", PROJ_MON_NAME).as_str()).await );
  }

  let mut headless_display_name = 

  // Move virtual display to -1920,0; to left of main screen
  dump_error!( sway_conn.run_command(format!("output {} mode 1920x1080 pos -1920 0 bg #000000 solid_color", PROJ_MON_NAME).as_str()).await );


  // Print sway displays
  if let Ok(outputs) = sway_conn.get_outputs().await {
    for output in outputs.iter() {
      println!("");
      println!("output = {:?}", output);
      println!("");
    }
  }

  tokio::time::sleep( std::time::Duration::from_millis(250) ).await;

  // Now remove the display
  dump_error!( sway_conn.run_command(format!("output {} unplug", PROJ_MON_NAME).as_str()).await );
  for i in 0..12 {
    dump_error!( sway_conn.run_command(format!("output HEADLESS-{} unplug", i).as_str()).await );
  }

  println!("Done!");

  EXIT_TASKS_DONE.store(true, std::sync::atomic::Ordering::SeqCst);

}

static EXIT_FLAG: once_cell::sync::Lazy<std::sync::atomic::AtomicBool> = once_cell::sync::Lazy::new(||
  std::sync::atomic::AtomicBool::new(false)
);

static EXIT_TASKS_DONE: once_cell::sync::Lazy<std::sync::atomic::AtomicBool> = once_cell::sync::Lazy::new(||
  std::sync::atomic::AtomicBool::new(false)
);

async fn handle_exit_signals() {
  let mut int_stream = dump_error_and_ret!(
    tokio::signal::unix::signal(
      tokio::signal::unix::SignalKind::interrupt()
    )
  );
  let mut term_stream = dump_error_and_ret!(
    tokio::signal::unix::signal(
      tokio::signal::unix::SignalKind::terminate()
    )
  );
  loop {
    let mut want_shutdown = false;
    tokio::select!{
      _sig_int = int_stream.recv() => { want_shutdown = true; }
      _sig_term = term_stream.recv() => { want_shutdown = true; }
    };
    if want_shutdown {
      println!("Got SIG{{TERM/INT}}, shutting down!");
      
      EXIT_FLAG.store(true, std::sync::atomic::Ordering::SeqCst);

      // Allow spawned futures to complete...
      for _ in 0..20 { // 50ms/poll * 20 == 1000ms max wait
        if EXIT_TASKS_DONE.load(std::sync::atomic::Ordering::SeqCst) {
          break;
        }
        tokio::time::sleep( tokio::time::Duration::from_millis(50) ).await;
      }

      println!("Goodbye!");
      std::process::exit(0);
    }
  }
}






