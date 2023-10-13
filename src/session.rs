use std::time::{Duration, Instant};

use actix::prelude::*;
use actix_web::body::None;
use actix_web_actors::ws::{self, WebsocketContext};

use crate::server::{self, ChatServer};

/// How often heartbeat pings are sent
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);

/// How long before lack of client response causes a timeout
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug)]
pub struct WsChatSession {
    /// unique session id
    pub id: usize,

    /// Client must send ping at least once per 10 seconds (CLIENT_TIMEOUT),
    /// otherwise we drop connection.
    pub hb: Instant,

    /// joined room
    pub room: usize,

    /// Chat server
    pub addr: Addr<server::ChatServer>,
}

impl WsChatSession {
    /// helper method that sends ping to client every 5 seconds (HEARTBEAT_INTERVAL).
    ///
    /// also this method checks heartbeats from client
    fn hb(&self, ctx: &mut ws::WebsocketContext<Self>) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            // check client heartbeats
            if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
                // heartbeat timed out
                println!("Websocket Client heartbeat failed, disconnecting!");

                // notify chat server
                act.addr.do_send(server::Disconnect { id: act.id, room: act.room.clone() });

                // stop actor
                ctx.stop();

                // don't try to send a ping
                return;
            }

            ctx.ping(b"");
        });
    }

    pub fn commandHandler<T>(
        &mut self, 
        msg: T, 
        match_res: &'static fn(
            Result<<T as actix::Message>::Result, MailboxError>, 
            &mut WebsocketContext<WsChatSession>)     
        -> None, 
        ctx: &mut ws::WebsocketContext<Self>
    )
    where
        ChatServer: actix::Handler<T>,
        T: Message + Send + 'static,
        T::Result: Send,
    {
        self.addr.send(msg)
        .into_actor(self)
        .then(|res, _, ctx| {
            match_res(res, ctx);
            fut::ready(())
        })
        .wait(ctx)
    }
}

impl Actor for WsChatSession {
    type Context = ws::WebsocketContext<Self>;

    /// Method is called on actor start.
    /// We register ws session with ChatServer
    fn started(&mut self, ctx: &mut Self::Context) {
        // we'll start heartbeat process on session start.
        self.hb(ctx);

        // register self in chat server. `AsyncContext::wait` register
        // future within context, but context waits until this future resolves
        // before processing any other events.
        // HttpContext::state() is instance of WsChatSessionState, state is shared
        // across all routes within application
        let addr = ctx.address();
        self.addr
            .send(server::Connect {
                addr: addr.recipient(),
                room: self.room.to_owned()
            })
            .into_actor(self)
            .then(|res, act, ctx| {
                match res {
                    Ok(res) => act.id = res,
                    // something is wrong with chat server
                    _ => {
                        act.addr.do_send(server::Disconnect { id: act.id, room: act.room.clone() });
                        ctx.stop()
                    },
                }
                fut::ready(())
            })
            .wait(ctx);
    }

    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        self.addr.do_send(server::Disconnect { id: self.id, room: self.room.clone() });
        Running::Stop
    }
}

/// Handle messages from chat server, we simply send it to peer websocket
impl Handler<server::Message> for WsChatSession {
    type Result = ();

    fn handle(&mut self, msg: server::Message, ctx: &mut Self::Context) {
        ctx.text(msg.0);
    }
}

/// WebSocket message handler
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsChatSession {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        let msg = match msg {
            Err(_) => {
                self.addr.do_send(server::Disconnect { id: self.id, room: self.room.clone() });
                ctx.stop();
                return;
            }
            Ok(msg) => msg,
        };

        match msg {
            ws::Message::Ping(msg) => {
                self.hb = Instant::now();
                ctx.pong(&msg);
            }
            ws::Message::Pong(_) => {
                self.hb = Instant::now();
            }
            ws::Message::Text(text) => {
                println!("WEBSOCKET MESSAGE: {text:?}");
                let m = text.trim();
                // we check for /sss type of messages
                if m.starts_with('/') {
                    let v: Vec<&str> = m.splitn(2, ' ').collect();
                    match v[0] {
                        "/list" => {
                            // Send ListRooms message to chat server and wait for
                            // response

                            self.addr
                                .send(server::ListRooms)
                                .into_actor(self)
                                .then(|res, _, ctx| {
                                    match res {
                                        Ok(rooms) => {
                                            for room in rooms {
                                                ctx.text(room.to_string());
                                            }
                                        }
                                        _ => println!("!!! something is wrong"),
                                    }
                                    fut::ready(())
                                })
                                .wait(ctx)
                            // .wait(ctx) pauses all events in context,
                            // so actor wont receive any new messages until it get list
                            // of rooms back
                        }
                        "/invite" => {
                            if v.len() != 2 {
                                ctx.text("!!! syntax error");
                                return;
                            }
                            if let Some(room) = v[1].parse::<usize>().ok() {
                                self.addr
                                .send(server::Invite { 
                                    id: self.id,
                                    room: room.clone(),
                                    from_room: self.room
                                })
                                .into_actor(self)
                                .then(|res, _, ctx| {
                                    match res {
                                        Ok(ivite_res) => {
                                            match ivite_res {
                                                server::InviteResult::Asked => ctx.text("/asked"),
                                                server::InviteResult::RoomDontExist => ctx.text("!!! room does not exist")
                                            }
                                        }
                                        _ => println!("!!! something is wrong"),
                                    }
                                    fut::ready(())
                                })
                                .wait(ctx)
                            } else {
                                ctx.text("!!! room name required ");
                            }
                        }
                        "/send" => {
                            if v.len() == 2 {
                                let user_data: Vec<&str> = v[1].splitn(2, ' ').collect();
                                if let Some(room) = user_data.get(0).map_or(None, |x| x.parse::<usize>().ok()) {
                                    if let Some(id) = user_data.get(1).map_or(None, |x| x.parse::<usize>().ok()) {
                                        self.addr.send(server::SendRoomKey{
                                            room: room.clone(),
                                            from_room: self.room,
                                            id: id.clone()
                                        })
                                        .into_actor(self)
                                        .then(|res, _, ctx| {
                                            match res {
                                                Ok(send_res) => {
                                                    match send_res {
                                                        server::SendRoomKeyResult::Send => ctx.text("/send"),
                                                        server::SendRoomKeyResult::RoomDontExist => ctx.text("!!! room does not exist")
                                                    }
                                                    
                                                }
                                                _ => ctx.text("!!! somethig go wrong"),
                                            }
                                            fut::ready(())
                                        })
                                        .wait(ctx)
                                    } else {
                                        ctx.text("!!! user id must be integer");
                                    }
                                } else {
                                    ctx.text("!!! room name must be integer");
                                }
                            }else{
                                ctx.text("!!! room name and key is required");
                            }
                        }
                        "/join" => {
                            if v.len() == 2 {
                                let room_data: Vec<&str> = v[1].splitn(2, ' ').collect();
                                if let Some(name) = room_data.get(0).map_or(None, |x| x.parse::<usize>().ok()) {
                                    if let Some(key) = room_data.get(1).map_or(None, |x| x.parse::<usize>().ok()) {

                                        let old = self.room.clone();
                                        self.room = name;

                                        self.addr.send(server::Join {
                                            id: self.id,
                                            name: self.room.clone(),
                                            key,
                                            room: old,
                                        })
                                        .into_actor(self)
                                        .then(|res, _, ctx| {
                                            match res {
                                                Ok(join_res) => {
                                                    match join_res {
                                                        server::JoinResult::Joined => ctx.text("/joined"),
                                                        server::JoinResult::RoomDontExist => ctx.text("!!! room does not exist"),
                                                        server::JoinResult::BadKey => ctx.text("!!! bad key")
                                                    }
                                                    
                                                }
                                                _ => ctx.text("!!! somethig go wrong"),
                                            }
                                            fut::ready(())
                                        })
                                        .wait(ctx)
                                    } else {
                                        ctx.text("!!! room key must be integer");
                                    }
                                } else {
                                    ctx.text("!!! room name must be integer");
                                }
                                
                            } else {
                                ctx.text("!!! room name and key is required");
                            }
                        }
                        "/room" => {
                            if v.len() != 1 {
                                ctx.text("!!! syntax error");
                                return;
                            }
                            self.addr.send(server::Room{ name: self.room.clone() })
                            .into_actor(self)
                            .then(|res, session, ctx| {
                                match res {
                                    Ok(key) => {
                                        match key {
                                            Some(x) => ctx.text(format!("/room {:?} {:?}", session.room, x)),
                                            None => ctx.text("!!! cant get key"),
                                        }
                                    }
                                    _ => ctx.text("!!! somethig go wrong"),
                                }
                                fut::ready(())
                            })
                            .wait(ctx)
                        }
                        "/id" => {
                            if v.len() != 1 {
                                ctx.text("!!! syntax error");
                                return;
                            }
                            ctx.text(format!("/id {:?}", self.id))
                        }
                        "/members" => {
                            if v.len() != 1 {
                                ctx.text("!!! syntax error");
                                return;
                            }
                            self.addr.send(server::Members{ room: self.room.clone() })
                            .into_actor(self)
                            .then(|res, _, ctx| {
                                match res {
                                    Ok(key) => {
                                        ctx.text(format!("/members {:?}", key));
                                    }
                                    _ => ctx.text("!!! somethig go wrong"),
                                }
                                fut::ready(())
                            })
                            .wait(ctx)
                        }
                        "/direct_message" => {
                            if v.len() != 2 {
                                ctx.text("!!! syntax error");
                                return;
                            }
                            let data: Vec<&str> = v[1].splitn(2, ' ').collect();
                            if let Some(id) = data.get(0).map_or(None, |x| x.parse::<usize>().ok()) {
                                if let Some(mess) = data.get(1) {
                                    self.addr.send(server::Direct{ 
                                        room: self.room,
                                        id_to: id,
                                        id_from: self.id,
                                        mess: mess.to_string()
                                    })
                                    .into_actor(self)
                                    .then(|res, _, ctx| {
                                        match res {
                                            Ok(server::DirectResult::Send) => ctx.text("/send"),
                                            Ok(server::DirectResult::IdDontExist) => ctx.text("!!! id not found"),
                                            _ => ctx.text("!!! somethig go wrong"),
                                        }
                                        fut::ready(())
                                    })
                                    .wait(ctx)
                                }else{
                                    ctx.text("!!! offer must be string");
                                    return;
                                }
                            }else{
                                ctx.text("!!! user id must be integer");
                                return;
                            }
                        }
                        _ => ctx.text(format!("!!! unknown command: {m:?}")),
                    }
                } else {
                    // send message to chat server
                    self.addr.do_send(server::ClientMessage {
                        id: self.id,
                        msg: m.to_string(),
                        room: self.room.clone(),
                    })
                }
            }
            ws::Message::Binary(_) => println!("Unexpected binary"),
            ws::Message::Close(reason) => {
                ctx.close(reason);
                self.addr.do_send(server::Disconnect { id: self.id, room: self.room.clone() });
                ctx.stop();
            }
            ws::Message::Continuation(_) => {
                self.addr.do_send(server::Disconnect { id: self.id, room: self.room.clone() });
                ctx.stop();
            }
            ws::Message::Nop => (),
        }
    }
}