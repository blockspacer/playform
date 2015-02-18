//! Defines the messages passed between client and server.

use block_position::BlockPosition;
use entity::EntityId;
use lod::{LODIndex, OwnerId};
use nalgebra::{Vec2, Vec3, Pnt3};
use nanomsg::{Endpoint, Socket, Protocol};
use process_events::{process_channel, process_socket};
use rustc_serialize::{Encodable, Decodable, json};
use std::fmt::Debug;
use std::old_io::timer;
use std::sync::mpsc::{channel, Sender, Receiver};
use std::time::duration::Duration;
use std::thread::Thread;
use terrain_block::TerrainBlock;
use vertex::ColoredVertex;

#[derive(Debug, Clone)]
#[derive(RustcDecodable, RustcEncodable)]
/// Messages the client sends to the server.
pub enum ClientToServer {
  /// Notify the server that the client exists, and provide a "return address".
  Init(String),
  /// Add a vector the player's acceleration.
  Walk(Vec3<f32>),
  /// Rotate the player by some amount.
  RotatePlayer(Vec2<f32>),
  /// [Try to] start a jump for the player.
  StartJump,
  /// [Try to] stop a jump for the player.
  StopJump,
  /// Ask the server to send a block of terrain.
  RequestBlock(BlockPosition, LODIndex),
}

#[derive(Debug, Clone)]
#[derive(RustcDecodable, RustcEncodable)]
/// Messages the server sends to the client.
pub enum ServerToClient {
  /// Give the client an OwnerId for terrain load requests.
  LeaseId(OwnerId),

  /// Update the player's position.
  UpdatePlayer(Pnt3<f32>),

  /// Tell the client to add a new mob with the given mesh.
  AddMob(EntityId, Vec<ColoredVertex>),
  /// Update the client's view of a mob with a given mesh.
  UpdateMob(EntityId, Vec<ColoredVertex>),

  /// The sun as a [0, 1) portion of its cycle.
  UpdateSun(f32),

  /// Provide a block of terrain to a client.
  AddBlock(BlockPosition, TerrainBlock, LODIndex),
}

/// Spawn a new thread to send messages to a socket and wait for acks.
pub fn spark_socket_sender<T>(url: String) -> (Sender<T>, Endpoint)
  where T: Send + Encodable + Debug
{
  let mut socket = Socket::new(Protocol::Push).unwrap();
  let endpoint = socket.connect(url.as_slice()).unwrap();

  let (send, recv) = channel();

  Thread::spawn(move || {
    loop {
      process_channel(
        &recv,
        |request| {
          let request = json::encode(&request).unwrap();
          if let Err(e) = socket.write_all(request.as_bytes()) {
            panic!("Error sending message: {:?}", e);
          }
          true
        }
      );

      println!("thread done!");

      timer::sleep(Duration::milliseconds(0));
    }
  });

  (send, endpoint)
}

/// Spawn a new thread to read messages from a socket and ack.
pub fn spark_socket_receiver<T>(url: String) -> (Receiver<T>, Endpoint)
  where T: Send + Decodable
{
  let mut socket = Socket::new(Protocol::Pull).unwrap();
  let endpoint = socket.bind(url.as_slice()).unwrap();

  let (send, recv) = channel();

  Thread::spawn(move || {
    loop {
      process_socket(
        &mut socket,
        |t| {
          send.send(t).unwrap();
          true
        },
      );

      timer::sleep(Duration::milliseconds(0));
    }
  });

  (recv, endpoint)
}
