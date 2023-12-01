use std::sync::mpsc::Receiver;
use std::{time::Instant, net::SocketAddr, ffi::OsString, thread};
use mio::{net::UdpSocket, Events};
use quiche::{self, Connection};
use ring::rand::*;
use url::*;
use quiche::h3::*;
use std::env;
use std::time::{self, Duration};
use std::sync::mpsc;
const MAX_DATAGRAM_SIZE: usize = 8192; //max
use std::fs::File;
use std::fs::OpenOptions;
use std::io::Write;



fn set_config_params(config: &mut quiche::Config){
    config.set_application_protos(quiche::h3::APPLICATION_PROTOCOL).unwrap();
    config.set_max_idle_timeout(15000);
    config.set_max_recv_udp_payload_size(MAX_DATAGRAM_SIZE);
    config.set_max_send_udp_payload_size(MAX_DATAGRAM_SIZE);
    config.set_initial_max_data(10_000_000);
    config.set_initial_max_stream_data_bidi_local(1_000_000);
    config.set_initial_max_stream_data_bidi_remote(1_000_000);
    config.set_initial_max_stream_data_uni(1_000_000);
    config.set_initial_max_streams_bidi(100);
    config.set_initial_max_streams_uni(100);
    // config.set_disable_active_migration(true);
    config.verify_peer(false);
    config.enable_early_data();
    config.enable_hystart(false);
    config.enable_pacing(false);
}

#[allow(unused_mut)]
fn setup_sockets()->(UdpSocket,SocketAddr,SocketAddr){
    let peer_addr = "127.0.0.1:6653".parse().unwrap();
    let bind_addr = match peer_addr {
        std::net::SocketAddr::V4(_) => "0.0.0.0:0",
        std::net::SocketAddr::V6(_) => "[::]:0",
    };
    let mut socket = mio::net::UdpSocket::bind(bind_addr.parse().unwrap()).unwrap();
    let local_addr = socket.local_addr().unwrap();
    return (socket,local_addr,peer_addr);
}

fn create_h3_request(url: Url)-> Vec<Header> {
    let mut path = String::from(url.path());

    if let Some(query) = url.query() {
        path.push('?');
        path.push_str(query);
    }

    let req = vec![
        quiche::h3::Header::new(b":method", b"GET"),
        quiche::h3::Header::new(b":scheme", url.scheme().as_bytes()),
        quiche::h3::Header::new(
            b":authority",
            url.host_str().unwrap().as_bytes(),
        ),
        quiche::h3::Header::new(b":path", path.as_bytes()),
        quiche::h3::Header::new(b"user-agent", b"quiche"),
    ];
    //println!("REQUEST CREATED:{:?}",req);
    return req;
}

fn send_initial_packet(conn: &mut Connection,out: &mut [u8;MAX_DATAGRAM_SIZE],socket: &UdpSocket){
    let (write, send_info) = conn.send(out).expect("initial send failed");
    while let Err(e) = socket.send_to(&out[..write], send_info.to) {
        if e.kind() == std::io::ErrorKind::WouldBlock {
            //println!("No more UDP packets to send...");
            continue;
        }
        panic!("send() failed: {:?}", e);
    }
    //println!("written {}", write);
}
fn main(){
    let args: Vec<String> = env::args().collect();
    let cc_algo = &args[1];


    let mut buf = [0;65535]; //total buffer
    let mut out = [0;MAX_DATAGRAM_SIZE]; //out buffer. Set to 8kB
    //create the config for quiche
    let mut config = quiche::Config::new(quiche::PROTOCOL_VERSION).unwrap();
    set_config_params(&mut config);
    config.set_cc_algorithm_name(cc_algo);
    // Setup connection id
    let mut scid = [0;quiche::MAX_CONN_ID_LEN];
    SystemRandom::new().fill(&mut scid[..]).unwrap();
    let scid = quiche::ConnectionId::from_ref(&scid);

    //setup the event loop
    let mut poll = mio::Poll::new().unwrap();
    let mut events = mio::Events::with_capacity(1024);

    let (tx,rx):(mpsc::Sender<u64>,mpsc::Receiver<u64>) = mpsc::channel();
    thread::spawn(move ||{
        let start = Instant::now();
        let mut last_bytes = 0;
        let mut last_elapsed = 0;
        loop{
            let new_bytes = rx.recv().unwrap();
            let elapsed = start.elapsed().as_millis()+1;
            let mut log_file = OpenOptions::new().append(true).create(true).open("log.txt").expect("File cannot be opened");
            let bytes_diff = (new_bytes-last_bytes) as u64;
            let elapsed_diff = (elapsed-last_elapsed) as u64;
            let bandwidth = ((bytes_diff/elapsed_diff) *8000) as u64;
            let _ = log_file.write((format!("New Bytes: {},t={},bandwidth={} B/s\n",new_bytes,elapsed,bandwidth)).as_bytes());
            last_bytes = new_bytes;
            last_elapsed = elapsed;
            thread::sleep(time::Duration::from_millis(10));
        }
    }); 
    //socket creation and address linking
    let local_addr: SocketAddr;
    let peer_addr: SocketAddr;
    let mut socket: UdpSocket;
    (socket,local_addr,peer_addr) = setup_sockets();

    //register socket with mio events
    poll.registry()
        .register(&mut socket, mio::Token(0), mio::Interest::READABLE)
        .unwrap();
    
    loop{
        let start = time::Instant::now();
        println!("LOOP IT");
        let mut conn = quiche::connect(None, &scid,local_addr, peer_addr, &mut config).unwrap();
        // if let Some(dir) = std::env::var_os("QLOGDIR") {
        //     let id = format!("{scid:?}");
        //     let writer = make_qlog_writer(&dir, "client", &id);
        //     conn.set_qlog(
        //         std::boxed::Box::new(writer),
        //         "quiche-client qlog".to_string(),
        //         format!("{} id={}", "quiche-client qlog", id),
        //     );
        // }
        //establish connection
        let url = Url::parse("http://127.0.0.1/files/rand.csv").unwrap();
        //debug print
        // println!(
        //     "connecting to {:} from {:} with scid {}",
        //     peer_addr,
        //     socket.local_addr().unwrap(),
        //     hex_dump(&scid)
        // );
        
        send_initial_packet(&mut conn, &mut out, &socket);// send initial packet
    
        let h3_config = quiche::h3::Config::new().unwrap(); //create config got http3
        let req = create_h3_request(url); //create http3 request with headers
    
        
        let req_start = std::time::Instant::now();
        let mut req_sent = false;
        let mut http3_conn = None;
        
        loop {
            let stats = conn.stats();
            // println!("RECV BYTES:{}",stats.recv_bytes);
            let _ = tx.send(stats.recv_bytes+(stats.recv as u64)*32).unwrap();
            poll.poll(&mut events, conn.timeout()).unwrap();
            read_from_socket(&mut conn, &mut buf, &socket, &events, local_addr); // read response of initial packet
            if conn.is_closed() {
                println!("connection closed, {:?}, time = {}", conn.stats(),start.elapsed().as_secs());
                break;
            }
            // Create a new HTTP/3 connection once the QUIC connection is established.
            if conn.is_established() && http3_conn.is_none() {
                http3_conn = Some(
                    quiche::h3::Connection::with_transport(&mut conn, &h3_config)
                    .expect("Unable to create HTTP/3 connection, check the server's uni stream limit and window size"),
                );
            }
            // Send HTTP requests once the QUIC connection is established, and until
            // all requests have been sent.
            if let Some(h3_conn) = &mut http3_conn {
                if !req_sent {
                    //println!("sending HTTP request {:?}", req);
                    h3_conn.send_request(&mut conn, &req, false).unwrap();
                    req_sent = true;
                }
            }
    
            process_http3_responses(&mut http3_conn, &mut conn, &mut buf, req_start);
            send_written_packets(&mut conn, &socket,&mut out);
            //println!("{:?}",conn.stats());
            if conn.is_closed() {
                println!("connection closed, {:?}, time = {}", conn.stats(),start.elapsed().as_secs());
                break;
            }
        }
        
    }
    
    
    /*
        Keep reading from socket until there is nothing more left to read.
     */
    fn read_from_socket(conn: &mut Connection,buf: &mut[u8;65535],socket: &UdpSocket,events: &Events,local_addr: std::net::SocketAddr){
        'read: loop {
            if events.is_empty() {
                //println!("timed out");
                conn.on_timeout();
                break 'read;
            }
    
            let (len, from) = match socket.recv_from(buf) {
                Ok(v) => v,
                Err(e) => {
                    // There are no more UDP packets to read, so end the read
                    // loop.
                    if e.kind() == std::io::ErrorKind::WouldBlock {
                        //println!("recv() would block");
                        break 'read;
                    }
    
                    panic!("recv() failed: {:?}", e);
                },
            };
    
            //println!("got {} bytes", len);
    
            let recv_info = quiche::RecvInfo {
                to: local_addr,
                from,
            };
    
            // Process potentially coalesced packets.
            let read = match conn.recv(&mut buf[..len], recv_info) {
                Ok(v) => v,
    
                Err(e) => {
                    println!("recv failed: {:?}", e);
                    continue 'read;
                },
            };
            //println!("processed {} bytes", read);
        }
        //println!("done reading");
    }
    
    /*
        Process responses received to http3 request made.
     */
    fn process_http3_responses(http3_conn: &mut Option<quiche::h3::Connection>,conn: &mut Connection,buf: &mut[u8;65535],req_start: Instant){
        if let Some(http3_conn) = http3_conn {
            // Process HTTP/3 events.
            loop {
                match http3_conn.poll(conn) {
                    Ok((stream_id, quiche::h3::Event::Headers { list, .. })) => {
                        // println!(
                        //     "got response headers {:?} on stream id {}",
                        //     hdrs_to_strings(&list),
                        //     stream_id
                        // );
                    },
    
                    Ok((stream_id, quiche::h3::Event::Data)) => {
                        while let Ok(read) =
                            http3_conn.recv_body(conn, stream_id, buf)
                        {
                            // println!(
                            //     "got {} bytes of response data on stream {}",
                            //     read, stream_id
                            // );
                            // print!("{}", unsafe {std::str::from_utf8_unchecked(&buf[..read])});
                        }
                    },
    
                    Ok((_stream_id, quiche::h3::Event::Finished)) => {
                        // println!(
                        //     "response received in {:?}, closing...",
                        //     req_start.elapsed()
                        // );
    
                        conn.close(true, 0x100, b"kthxbye").unwrap();
                    },
    
                    Ok((_stream_id, quiche::h3::Event::Reset(e))) => {
                        // println!(
                        //     "request was reset by peer with {}, closing...",
                        //     e
                        // );
    
                        conn.close(true, 0x100, b"kthxbye").unwrap();
                    },
    
                    Ok((_, quiche::h3::Event::PriorityUpdate)) => unreachable!(),
    
                    Ok((goaway_id, quiche::h3::Event::GoAway)) => {
                        println!("GOAWAY id={}", goaway_id);
                    },
    
                    Err(quiche::h3::Error::Done) => {
                        break;
                    },
    
                    Err(e) => {
                        println!("HTTP/3 processing failed: {:?}", e);
    
                        break;
                    },
                }
            }
        }
    }
}

/*
    Send all packets written to buffer that wew
 */
fn send_written_packets(conn: &mut Connection,socket: &UdpSocket,out: &mut [u8;MAX_DATAGRAM_SIZE]){
    loop {
        let (write, send_info) = match conn.send(out) {
            Ok(v) => v,

            Err(quiche::Error::Done) => {
                //println!("done writing");
                break;
            },

            Err(e) => {
                println!("send failed: {:?}", e);
                conn.close(false, 0x1, b"fail").ok();
                break;
            },
        };

        if let Err(e) = socket.send_to(&out[..write], send_info.to) {
            if e.kind() == std::io::ErrorKind::WouldBlock {
                //println!("send() would block");
                break;
            }

            panic!("send() failed: {:?}", e);
        }

        //println!("written {}", write);
    }
}



fn hex_dump(buf: &[u8]) -> String {
    let vec: Vec<String> = buf.iter().map(|b| format!("{b:02x}")).collect();

    vec.join("")
}

pub fn hdrs_to_strings(hdrs: &[quiche::h3::Header]) -> Vec<(String, String)> {
    hdrs.iter()
        .map(|h| {
            let name = String::from_utf8_lossy(h.name()).to_string();
            let value = String::from_utf8_lossy(h.value()).to_string();

            (name, value)
        })
        .collect()
}

pub fn make_qlog_writer(
    dir: &std::ffi::OsStr, role: &str, id: &str,
) -> std::io::BufWriter<std::fs::File> {
    let mut path = std::path::PathBuf::from(dir);
    let filename = format!("{role}-{id}.sqlog");
    path.push(filename);

    match std::fs::File::create(&path) {
        Ok(f) => std::io::BufWriter::new(f),

        Err(e) => panic!(
            "Error creating qlog file attempted path was {:?}: {}",
            path, e
        ),
    }
}

