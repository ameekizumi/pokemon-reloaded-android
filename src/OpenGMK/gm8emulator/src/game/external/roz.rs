//! Native stand-in for the Reloaded Online mod's `ROZ.dll`.
//!
//! Android (and any non x86-Windows build) cannot load the DLL, so the 15 exports are
//! re-implemented here. Behaviour is inferred from the DLL's exported signatures, its string
//! table and a read of its disassembly; it never executes or embeds DLL code.
//!
//! Wire format (newline-terminated text lines, TCP):
//!   client -> server: `HELLO <name>`, `JOIN <room>`, `MSG <payload>`, `IN <epoch> <a> <b>`
//!   server -> client: `WOK <side>`, `WFULL`, `START`, `PEER_LEFT`, `MSG <payload>`, `IN <epoch> <frame> <mask>`
//!
//! UNVERIFIED (check against a packet capture of the original DLL):
//!   * argument order of net_join and net_input_send
//!   * numeric meaning of net_status
//!   * whether net_line sends or parses a line
//!   * net_send_file payload encoding (assumed lowercase hex)

use super::dll::{PascalString, Value};
use std::{
    collections::{HashMap, VecDeque},
    io::{BufRead, BufReader, Read, Write},
    net::{TcpStream, ToSocketAddrs},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RozFn {
    Connect,
    Disconnect,
    Focus,
    InputGet,
    InputSend,
    Join,
    Line,
    Peer,
    Recv,
    Send,
    SendFile,
    SetEpoch,
    Side,
    Status,
    WriteFile,
}

impl RozFn {
    pub fn from_symbol(sym: &str) -> Option<Self> {
        Some(match sym {
            "net_connect" => Self::Connect,
            "net_disconnect" => Self::Disconnect,
            "net_focus" => Self::Focus,
            "net_input_get" => Self::InputGet,
            "net_input_send" => Self::InputSend,
            "net_join" => Self::Join,
            "net_line" => Self::Line,
            "net_peer" => Self::Peer,
            "net_recv" => Self::Recv,
            "net_send" => Self::Send,
            "net_send_file" => Self::SendFile,
            "net_set_epoch" => Self::SetEpoch,
            "net_side" => Self::Side,
            "net_status" => Self::Status,
            "net_write_file" => Self::WriteFile,
            _ => return None,
        })
    }
}

#[derive(Default)]
struct Shared {
    status: i32, // 0 idle, 1 connected, 2 waiting in room, 3 started, -1 room full, -2 peer left
    side: i32,
    peer: String,
    messages: VecDeque<String>,
    inputs: HashMap<(i32, i32), i32>, // (epoch, frame) -> mask
    closed: bool,
}

#[derive(Default)]
pub struct Roz {
    stream: Option<TcpStream>,
    shared: Arc<Mutex<Shared>>,
    epoch: i32,
}

fn text(v: Option<&Value>) -> String {
    match v {
        Some(Value::Str(s)) => String::from_utf8_lossy(s.as_slice()).trim_end_matches('\0').to_string(),
        _ => String::new(),
    }
}

fn num(v: Option<&Value>) -> f64 {
    match v {
        Some(Value::Real(r)) => *r,
        _ => 0.0,
    }
}

fn s(v: &str) -> Value {
    Value::Str(PascalString::new(v.as_bytes()))
}

fn r(v: f64) -> Value {
    Value::Real(v)
}

impl Roz {
    pub fn new() -> Self {
        Self::default()
    }

    fn send_line(&mut self, line: &str) -> bool {
        let Some(stream) = self.stream.as_mut() else { return false };
        let mut data = line.replace(['\r', '\n'], " ").into_bytes();
        data.push(b'\n');
        stream.write_all(&data).and_then(|_| stream.flush()).is_ok()
    }

    fn connect(&mut self, host: &str, port: i32) -> bool {
        self.disconnect();
        let Some(addr) = (host, port as u16).to_socket_addrs().ok().and_then(|mut a| a.next()) else {
            return false
        };
        let Ok(stream) = TcpStream::connect_timeout(&addr, Duration::from_secs(5)) else { return false };
        let _ = stream.set_nodelay(true);
        let Ok(reader) = stream.try_clone() else { return false };
        *self.shared.lock().unwrap() = Shared { status: 1, ..Default::default() };
        let shared = self.shared.clone();
        thread::spawn(move || {
            for line in BufReader::new(reader).lines() {
                let Ok(line) = line else { break };
                let mut sh = shared.lock().unwrap();
                let (cmd, rest) = line.split_once(' ').unwrap_or((line.as_str(), ""));
                match cmd {
                    "WOK" => {
                        sh.side = rest.trim().parse().unwrap_or(0);
                        sh.status = 2;
                    },
                    "WFULL" => sh.status = -1,
                    "START" => sh.status = 3,
                    "PEER_LEFT" => sh.status = -2,
                    "MSG" => {
                        // first token after MSG is taken as the peer's name when none is known
                        if sh.peer.is_empty() {
                            sh.peer = rest.split(' ').next().unwrap_or("").to_string();
                        }
                        sh.messages.push_back(rest.to_string());
                    },
                    "IN" => {
                        let p: Vec<i32> = rest.split(' ').filter_map(|x| x.parse().ok()).collect();
                        if let [epoch, frame, mask] = p[..] {
                            sh.inputs.insert((epoch, frame), mask);
                        }
                    },
                    _ => (),
                }
            }
            let mut sh = shared.lock().unwrap();
            sh.closed = true;
            sh.status = 0;
        });
        self.stream = Some(stream);
        true
    }

    fn disconnect(&mut self) {
        if let Some(stream) = self.stream.take() {
            let _ = stream.shutdown(std::net::Shutdown::Both);
        }
        *self.shared.lock().unwrap() = Shared::default();
    }

    pub fn call(&mut self, f: RozFn, args: &[Value]) -> Value {
        match f {
            RozFn::Connect => r(f64::from(self.connect(&text(args.get(0)), num(args.get(1)) as i32) as u8)),
            RozFn::Disconnect => {
                self.disconnect();
                r(1.0)
            },
            // The original checks that its own window is foreground; the emulator has no
            // background-input problem on Android, so always report focus.
            RozFn::Focus => r(1.0),
            RozFn::Join => {
                let (room, name) = (text(args.get(0)), text(args.get(1)));
                let ok = self.send_line(&format!("HELLO {name}")) && self.send_line(&format!("JOIN {room}"));
                r(f64::from(ok as u8))
            },
            RozFn::Line | RozFn::Send => {
                let msg = text(args.get(0));
                let line = if f == RozFn::Send { format!("MSG {msg}") } else { msg };
                r(f64::from(self.send_line(&line) as u8))
            },
            RozFn::Recv => {
                let msg = self.shared.lock().unwrap().messages.pop_front().unwrap_or_default();
                s(&msg)
            },
            RozFn::Peer => s(&self.shared.lock().unwrap().peer.clone()),
            RozFn::Side => r(f64::from(self.shared.lock().unwrap().side)),
            RozFn::Status => r(f64::from(self.shared.lock().unwrap().status)),
            RozFn::SetEpoch => {
                self.epoch = num(args.get(0)) as i32;
                r(1.0)
            },
            RozFn::InputSend => {
                let (frame, mask) = (num(args.get(0)) as i32, num(args.get(1)) as i32);
                let epoch = self.epoch;
                r(f64::from(self.send_line(&format!("IN {epoch} {frame} {mask}")) as u8))
            },
            // net_input_get(frame, timeout_ms): returns the peer's mask, or -1 on timeout
            RozFn::InputGet => {
                let frame = num(args.get(0)) as i32;
                let timeout = Duration::from_millis(num(args.get(1)).max(0.0) as u64);
                let start = Instant::now();
                loop {
                    if let Some(m) = self.shared.lock().unwrap().inputs.remove(&(self.epoch, frame)) {
                        return r(f64::from(m))
                    }
                    if start.elapsed() >= timeout || self.shared.lock().unwrap().closed {
                        return r(-1.0)
                    }
                    thread::sleep(Duration::from_millis(1));
                }
            },
            RozFn::SendFile => {
                let (tag, path) = (text(args.get(0)), text(args.get(1)));
                let Ok(mut file) = std::fs::File::open(path) else { return r(0.0) };
                let mut bytes = Vec::new();
                if file.read_to_end(&mut bytes).is_err() {
                    return r(0.0)
                }
                let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
                r(f64::from(self.send_line(&format!("MSG {tag} {hex}")) as u8))
            },
            RozFn::WriteFile => {
                let (path, data) = (text(args.get(0)), text(args.get(1)));
                r(f64::from(std::fs::write(path, data.as_bytes()).is_ok() as u8))
            },
        }
    }
}
