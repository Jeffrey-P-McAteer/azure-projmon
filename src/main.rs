
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

  let mut sway_conn = dump_error_and_ret!( swayipc_async::Connection::new().await );

  let mut connected_projector_name = String::new();

  println!("Waiting for one of {:?} to be connected...", PROJECTOR_DISPLAY_NAMES);
  loop {

    {
      let mut stdout = tokio::io::stdout();
      dump_error!( stdout.write_all(b".").await );
      dump_error!( stdout.flush().await );
    }

    if let Ok(outputs) = sway_conn.get_outputs().await {
      for output in outputs.iter() {
        if PROJECTOR_DISPLAY_NAMES.contains( &output.name.as_str() ) {
          connected_projector_name = output.name.clone();
        }
      }
    }

    if connected_projector_name.len() > 1 {
      break; // found projector!
    }
    

    tokio::time::sleep( std::time::Duration::from_millis(1250) ).await;    
  }

  println!("");
  println!("Saw connected projector at {:?}", &connected_projector_name);

  // Once projector is connected, create EVDI virtual monitor & tell sway to position at -1080,0




  println!("Done!");

}