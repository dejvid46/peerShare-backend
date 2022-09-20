use std::{
    collections::{HashMap, HashSet}
};

use std::sync::Mutex;
use actix_web::web::Data;
use actix::prelude::*;
use rand::{self, rngs::ThreadRng, Rng};

use crate::queue::Queue;

#[derive(Message)]
#[rtype(result = "()")]
pub struct Message(pub String);

#[derive(Message)]
#[rtype(usize)]
pub struct Connect {
    pub addr: Recipient<Message>,
    pub room: String
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Disconnect {
    pub id: usize,
    pub room: String
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct ClientMessage {
    /// Id of the client session
    pub id: usize,
    /// Peer message
    pub msg: String,
    /// Room name
    pub room: String,
}

pub struct ListRooms;

impl actix::Message for ListRooms {
    type Result = Vec<String>;
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Join {
    /// Client ID
    pub id: usize,

    /// Room name
    pub name: String,

    pub room: String
}

#[derive(Debug)]
pub struct ChatServer {
    sessions: HashMap<usize, Recipient<Message>>,
    rooms: HashMap<String, HashSet<usize>>,
    pub queue: Data<Mutex<Queue>>,
    rng: ThreadRng
}

impl ChatServer {
    pub fn new(queue: Data<Mutex<Queue>>) -> ChatServer {
        let rooms = HashMap::new();

        ChatServer {
            sessions: HashMap::new(),
            rooms,
            queue,
            rng: rand::thread_rng()
        }
    }
}

impl ChatServer {
    fn send_message(&self, room: &str, message: &str, skip_id: usize) {
        if let Some(sessions) = self.rooms.get(room) {
            for id in sessions {
                if *id != skip_id {
                    if let Some(addr) = self.sessions.get(id) {
                        addr.do_send(Message(message.to_owned()));
                    }
                }
            }
        }
    }
}

impl Actor for ChatServer {
    type Context = Context<Self>;
}

impl Handler<Connect> for ChatServer {
    type Result = usize;

    fn handle(&mut self, msg: Connect, _: &mut Context<Self>) -> Self::Result {
        println!("Someone joined");

        self.rooms.insert(msg.room.to_owned(), HashSet::new());

        // register session with random id
        let id = self.rng.gen::<usize>();
        self.sessions.insert(id, msg.addr);

        // auto join session to main room
        self.rooms
            .entry(msg.room)
            .or_insert_with(HashSet::new)
            .insert(id);

        // send id back
        id
    }
}

/// Handler for Disconnect message.
impl Handler<Disconnect> for ChatServer {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _: &mut Context<Self>) {
        let Disconnect { id, room } = msg;
        println!("Someone disconnected");

        // remove address
        if self.sessions.remove(&id).is_some() {

            // remove session from room
            if let Some(sessions) = self.rooms.get_mut(&room) {
                sessions.remove(&id);
                
                if sessions.is_empty() {
                    self.rooms.remove(&room);
                }
            }
        }
        // send message to other users
        self.send_message(&room, "Someone disconnected", 0);
    }
}

impl Handler<ClientMessage> for ChatServer {
    type Result = ();

    fn handle(&mut self, msg: ClientMessage, _: &mut Context<Self>) {
        self.send_message(&msg.room, msg.msg.as_str(), msg.id);
    }
}

impl Handler<ListRooms> for ChatServer {
    type Result = MessageResult<ListRooms>;

    fn handle(&mut self, _: ListRooms, _: &mut Context<Self>) -> Self::Result {
        let mut rooms = Vec::new();

        for key in self.rooms.keys() {
            rooms.push(key.to_owned())
        }

        MessageResult(rooms)
    }
}

impl Handler<Join> for ChatServer {
    type Result = ();

    fn handle(&mut self, msg: Join, _: &mut Context<Self>) {
        let Join { id, name, room } = msg;

        // remove session from room
        if let Some(sessions) = self.rooms.get_mut(&room) {
            sessions.remove(&id);

            if sessions.is_empty() {
                self.rooms.remove(&room);
            }
        }
        // send message to other users
        self.send_message(&room, "Someone disconnected", 0);

        self.rooms
            .entry(name.clone())
            .or_insert_with(HashSet::new)
            .insert(id);

        self.send_message(&name, "Someone connected", id);
    }
}