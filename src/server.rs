use std::collections::{HashMap, HashSet};

use actix::prelude::*;
use actix_web::web::Data;
use rand::{self, rngs::ThreadRng, Rng};
use std::sync::Mutex;

use crate::queue::Queue;

#[derive(Message)]
#[rtype(result = "()")]
pub struct Message(pub String);

#[derive(Message)]
#[rtype(usize)]
pub struct Connect {
    pub addr: Recipient<Message>,
    pub room: usize,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Disconnect {
    pub id: usize,
    pub room: usize,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct ClientMessage {
    /// Id of the client session
    pub id: usize,
    /// Peer message
    pub msg: String,
    /// Room name
    pub room: usize,
}

pub struct ListRooms;

impl actix::Message for ListRooms {
    type Result = Vec<usize>;
}

pub struct Room {
    pub name: usize,
}

impl actix::Message for Room {
    type Result = Option<usize>;
}

#[derive(Message)]
#[rtype(InviteResult)]
pub enum InviteResult {
    Asked,
    RoomDontExist,
}

pub struct Invite {
    pub id: usize,
    pub room: usize,
    pub from_room: usize,
}

impl actix::Message for Invite {
    type Result = InviteResult;
}

pub struct Members {
    pub room: usize,
}

impl actix::Message for Members {
    type Result = Vec<usize>;
}

pub struct Direct {
    pub room: usize,
    pub id_to: usize,
    pub id_from: usize,
    pub mess: String,
}

pub enum DirectResult {
    Send,
    IdDontExist,
}

impl actix::Message for Direct {
    type Result = DirectResult;
}

#[derive(Message)]
#[rtype(InviteResult)]
pub enum SendRoomKeyResult {
    Send,
    RoomDontExist,
}

pub struct SendRoomKey {
    pub room: usize,
    pub from_room: usize,
    pub id: usize,
}

impl actix::Message for SendRoomKey {
    type Result = SendRoomKeyResult;
}

#[derive(Message)]
#[rtype(JoinResult)]
pub enum JoinResult {
    Joined(usize),
    RoomDontExist,
    BadKey,
    FullRoom
}

pub struct Join {
    pub id: usize,

    pub name: usize,

    pub key: usize,

    pub room: usize,
}

impl actix::Message for Join {
    type Result = JoinResult;
}

#[derive(Debug)]
pub struct ChatServer {
    sessions: HashMap<usize, Recipient<Message>>,
    rooms: HashMap<usize, HashSet<usize>>,
    queue: Data<Mutex<Queue>>,
    keys: HashMap<usize, usize>,
    rng: ThreadRng,
}

impl ChatServer {
    pub fn new(queue: Data<Mutex<Queue>>) -> ChatServer {
        let rooms = HashMap::new();

        ChatServer {
            sessions: HashMap::new(),
            rooms,
            queue,
            keys: HashMap::new(),
            rng: rand::thread_rng(),
        }
    }
}

impl ChatServer {
    fn send_message(&self, room: &usize, message: &str, skip_id: usize) {
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

impl ChatServer {
    fn send_message_to_id(&self, room: &usize, message: &str, id: usize) {
        if let Some(sessions) = self.rooms.get(room) {
            for user_id in sessions {
                if *user_id == id {
                    if let Some(addr) = self.sessions.get(user_id) {
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
        self.rooms.insert(msg.room.to_owned(), HashSet::new());

        // register session with random id
        let id = self.rng.gen::<usize>();
        self.sessions.insert(id, msg.addr);

        // auto join session to main room
        self.rooms
            .entry(msg.room)
            .or_insert_with(HashSet::new)
            .insert(id);

        self.keys.insert(msg.room, self.rng.gen());

        // send id back
        id
    }
}

/// Handler for Disconnect message.
impl Handler<Disconnect> for ChatServer {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _: &mut Context<Self>) {
        let Disconnect { id, room } = msg;

        // remove address
        if self.sessions.remove(&id).is_some() {
            // remove session from room
            if let Some(sessions) = self.rooms.get_mut(&room) {
                sessions.remove(&id);

                if sessions.is_empty() {
                    {
                        let mut guard = self.queue.lock().unwrap();
                        guard.refund(&room);
                    }

                    self.rooms.remove(&room);
                    self.keys.remove(&room);
                }
            }
        }

        let mut ids_vec = Vec::new();

        if let Some(ids) = self.rooms.get(&room) {
            for room_id in ids {
                ids_vec.push(room_id.to_owned());
            }
        }

        // send message to other users
        self.send_message(&room, &format!("/members {:?}", ids_vec), 0);
    }
}

impl Handler<ClientMessage> for ChatServer {
    type Result = ();

    fn handle(&mut self, msg: ClientMessage, _: &mut Context<Self>) {
        self.send_message(
            &msg.room,
            &format!("/message {} {}", msg.id, msg.msg),
            msg.id,
        );
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

impl Handler<Members> for ChatServer {
    type Result = MessageResult<Members>;

    fn handle(&mut self, mem: Members, _: &mut Context<Self>) -> Self::Result {
        let mut ids_vec = Vec::new();

        if let Some(ids) = self.rooms.get(&mem.room) {
            for id in ids {
                ids_vec.push(id.to_owned())
            }
        }

        MessageResult(ids_vec)
    }
}

impl Handler<Direct> for ChatServer {
    type Result = MessageResult<Direct>;

    fn handle(&mut self, mess: Direct, _: &mut Context<Self>) -> Self::Result {
        let mut idisinroom = false;

        if let Some(ids) = self.rooms.get(&mess.room) {
            for id in ids {
                if id == &mess.id_to {
                    idisinroom = true;
                }
            }
        }

        if idisinroom {
            self.send_message_to_id(
                &mess.room,
                &format!("/direct_message {} {}", mess.id_from, mess.mess),
                mess.id_to,
            );
            return MessageResult(DirectResult::Send);
        }

        MessageResult(DirectResult::IdDontExist)
    }
}

impl Handler<Room> for ChatServer {
    type Result = MessageResult<Room>;

    fn handle(&mut self, room: Room, _: &mut Self::Context) -> Self::Result {
        MessageResult(self.keys.get(&room.name).map(|x| x.clone()))
    }
}

impl Handler<Invite> for ChatServer {
    type Result = MessageResult<Invite>;

    fn handle(&mut self, data: Invite, _: &mut Self::Context) -> Self::Result {
        if !self.rooms.contains_key(&data.room) {
            return MessageResult(InviteResult::RoomDontExist);
        }

        self.send_message(
            &data.room,
            &format!("/invite {} {}", &data.from_room, &data.id),
            0,
        );

        MessageResult(InviteResult::Asked)
    }
}

impl Handler<SendRoomKey> for ChatServer {
    type Result = MessageResult<SendRoomKey>;

    fn handle(&mut self, data: SendRoomKey, _: &mut Self::Context) -> Self::Result {
        if !self.rooms.contains_key(&data.room) {
            return MessageResult(SendRoomKeyResult::RoomDontExist);
        }
        if !self.keys.contains_key(&data.room) {
            return MessageResult(SendRoomKeyResult::RoomDontExist);
        }

        self.send_message_to_id(
            &data.room,
            &format!(
                "/send {} {:?}",
                &data.from_room,
                self.keys.get(&data.from_room)
            ),
            data.id,
        );

        MessageResult(SendRoomKeyResult::Send)
    }
}

impl Handler<Join> for ChatServer {
    type Result = MessageResult<Join>;

    fn handle(&mut self, msg: Join, _: &mut Context<Self>) -> Self::Result {
        let Join {
            id,
            name,
            key,
            room,
        } = msg;

        if let Some(room_key) = self.keys.get(&name) {
            if room_key != &key {
                return MessageResult(JoinResult::BadKey);
            };
        } else {
            return MessageResult(JoinResult::RoomDontExist);
        }

        if let Some(name ) = self.rooms.get(&name) {
            if name.len() > 10 {
                return MessageResult(JoinResult::FullRoom);   
            }
        } else {
            return MessageResult(JoinResult::RoomDontExist);
        }

        // remove session from room
        if let Some(sessions) = self.rooms.get_mut(&room) {
            sessions.remove(&id);

            if sessions.is_empty() {
                {
                    let mut guard = self.queue.lock().unwrap();
                    guard.refund(&room);
                }

                self.rooms.remove(&room);
                self.keys.remove(&room);
            }
        }

        self.rooms
            .entry(name.clone())
            .or_insert_with(HashSet::new)
            .insert(id);

        let mut ids_vec = Vec::new();

        if let Some(ids) = self.rooms.get(&msg.room) {
            for room_id in ids {
                ids_vec.push(room_id.to_owned());
            }
        }
        // send message to other users
        self.send_message(&room, &format!("/members {:?}", ids_vec), 0);

        let mut ids_vec = Vec::new();

        if let Some(ids) = self.rooms.get(&name) {
            for room_id in ids {
                ids_vec.push(room_id.to_owned());
            }
        }

        self.send_message(&name, &format!("/members {:?}", ids_vec), id);

        MessageResult(JoinResult::Joined(name))
    }
}
