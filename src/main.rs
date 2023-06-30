
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


async fn async_main(args: Vec<String>) {
  let exit_signal_handler = tokio::task::spawn(handle_exit_signals());

  let mut sway_conn = dump_error_and_ret!( swayipc_async::Connection::new().await );

  let mut connected_projector_name = String::new();
  let mut connected_projector_ws = String::new();

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


  // Once projector is connected, create EVDI virtual monitor & tell sway to position at -1080,0

  let evidi_handle = unsafe { evdi_sys::evdi_open_attached_to( std::ptr::null() ) };
  if evidi_handle.is_null() {
    eprintln!("Got null pointer from evdi_sys::evdi_open_attached_to!");
    return;
  }

  // 1920x1080 * 3 bytes/pixel = 
  //let mut evidi_screen_mem: [u8; 6220800] = [0; 6220800]; // blows stack
  let mut evidi_screen_mem = vec![0_u8; 6220800]; // Directly allocated on heap

  let evidi_buff = evdi_sys::evdi_buffer {
    id: 0,
    buffer: evidi_screen_mem.as_mut_ptr() as *mut core::ffi::c_void,
    width: 1920,
    height: 1080,
    stride: 3,
    rects:  std::ptr::null_mut::<evdi_sys::evdi_rect>(), // these 2 are modified by EVDI runtime to inform us of damaged areas to re-paint.
    rect_count: 0,
  };

  unsafe { evdi_sys::evdi_connect(evidi_handle, std::ptr::null(), 0, 6220800 ) };

  tokio::time::sleep( std::time::Duration::from_millis(250) ).await;

  // Print sway displays
  if let Ok(outputs) = sway_conn.get_outputs().await {
    for output in outputs.iter() {
      println!("");
      println!("output = {:?}", output);
      println!("");
    }
  }

  tokio::time::sleep( std::time::Duration::from_millis(250) ).await;

  // Finally close it
  unsafe { evdi_sys::evdi_disconnect(evidi_handle); }
  unsafe { evdi_sys::evdi_close(evidi_handle) };


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






