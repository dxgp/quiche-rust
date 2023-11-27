use quiche;
use ring::rand::*;
use qlog::Qlog;
use qlog::*;
const MAX_DATAGRAM_SIZE: usize = 5000; //max
use rand::distributions::{Alphanumeric, DistString};

fn main(){
    let mut buf = [0;65535];
    let mut out = [0;MAX_DATAGRAM_SIZE];
    
    let mut config = quiche::Config::new(quiche::PROTOCOL_VERSION).unwrap();
    config.set_application_protos(&[b"http/0.9"]);
    config.set_max_idle_timeout(5000);
    config.set_max_recv_udp_payload_size(MAX_DATAGRAM_SIZE);
    config.set_max_send_udp_payload_size(MAX_DATAGRAM_SIZE);
    config.set_initial_max_data(10_000_000);
    config.set_initial_max_stream_data_bidi_local(1_000_000);
    config.set_initial_max_stream_data_bidi_remote(1_000_000);
    config.set_initial_max_stream_data_uni(1_000_000);
    config.set_initial_max_streams_bidi(100);
    config.set_initial_max_streams_uni(100);
    config.set_disable_active_migration(true);
    config.verify_peer(false);

    // Setup connection id
    let mut scid = [0;quiche::MAX_CONN_ID_LEN];
    SystemRandom::new().fill(&mut scid[..]).unwrap();
    let scid = quiche::ConnectionId::from_ref(&scid);

    //setup the event loop
    let mut poll = mio::Poll::new().unwrap();
    let mut events = mio::Events::with_capacity(1024);

    //Setup the socket
    let peer_addr = "127.0.0.1:6653".parse().unwrap();
    let bind_addr = match peer_addr {
        std::net::SocketAddr::V4(_) => "0.0.0.0:0",
        std::net::SocketAddr::V6(_) => "[::]:0",
    };
    let mut socket = mio::net::UdpSocket::bind(bind_addr.parse().unwrap()).unwrap();
    let local_addr = socket.local_addr().unwrap();
    let mut conn = quiche::connect(None, &scid,local_addr, peer_addr, &mut config).unwrap();

    poll.registry()
        .register(&mut socket, mio::Token(0), mio::Interest::READABLE)
        .unwrap();
    
    // if let Some(dir) = std::env::var_os("QLOGDIR") {
    //     let id = format!("{scid:?}");
    //     let writer = make_qlog_writer(&dir, "client", &id);
    //     conn.set_qlog(
    //         std::boxed::Box::new(writer),
    //         "quiche-client qlog".to_string(),
    //         format!("{} id={}", "quiche-client qlog", id),
    //     );
    // }

    // if let Some(session_file) = &args.session_file {
    //     if let Ok(session) = std::fs::read(session_file) {
    //         conn.set_session(&session).ok();
    //     }
    // }

    println!(
        "connecting to {:} from {:} with scid {}",
        peer_addr,
        socket.local_addr().unwrap(),
        hex_dump(&scid)
    );

    let (write, send_info) = conn.send(&mut out).expect("initial send failed");

    while let Err(e) = socket.send_to(&out[..write], send_info.to) {
        if e.kind() == std::io::ErrorKind::WouldBlock {
            println!("No more UDP packets to send...");
            continue;
        }

        panic!("send() failed: {:?}", e);
    }

    println!("written {}", write);

    let req_start = std::time::Instant::now();

    let mut req_sent = false;
    loop {
        poll.poll(&mut events, conn.timeout()).unwrap();

        // Read incoming UDP packets from the socket and feed them to quiche,
        // until there are no more packets to read.
        'read: loop {
            // If the event loop reported no events, it means that the timeout
            // has expired, so handle it without attempting to read packets. We
            // will then proceed with the send loop.
            if events.is_empty() {
                println!("timed out");
                conn.on_timeout();
                break 'read;
            }

            let (len, from) = match socket.recv_from(&mut buf) {
                Ok(v) => v,
                Err(e) => {
                    if e.kind() == std::io::ErrorKind::WouldBlock {
                        println!("No more UDP packets to read...");
                        break 'read;
                    }

                    panic!("recv() failed: {:?}", e);
                },
            };

            println!("got {} bytes", len);

            let recv_info = quiche::RecvInfo {
                to: socket.local_addr().unwrap(),
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
        }
        println!("BUFFER: {}",std::string::String::from_utf8_lossy(&buf));
        println!("done reading");

        if conn.is_closed() {
            println!("connection closed, {:?}", conn.stats());
            break;
        }

        // Process all readable streams.
        for s in conn.readable() {
            while let Ok((read, fin)) = conn.stream_recv(s, &mut buf) {
                println!("received {} bytes", read);

                let stream_buf = &buf[..read];

                println!(
                    "stream {} has {} bytes (fin? {})",
                    s,
                    stream_buf.len(),
                    fin
                );

                print!("{}", unsafe {
                    std::str::from_utf8_unchecked(stream_buf)
                });

                // The server reported that it has no more data to send, which
                // we got the full response. Close the connection.
                if s == 4 && fin {
                    println!(
                        "response received in {:?}, closing...",
                        req_start.elapsed()
                    );

                    conn.close(true, 0x00, b"kthxbye").unwrap();
                }
            }
        }

        // Send an HTTP request as soon as the connection is established.
        if conn.is_established() && !req_sent {
            let mut ss_str: String = Alphanumeric.sample_string(&mut rand::thread_rng(), MAX_DATAGRAM_SIZE - 4).to_owned();
            ss_str.push_str("GET ");
            conn.stream_send(4, ss_str.as_bytes(), true)
                .unwrap();

            req_sent = true;
        }

        

        // Generate outgoing QUIC packets and send them on the UDP socket, until
        // quiche reports that there are no more packets to be sent.
        loop {
            let (write, send_info) = match conn.send(&mut out) {
                Ok(v) => v,

                Err(quiche::Error::Done) => {
                    println!("done writing");
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
                    println!("No more UDP Packets to send...");
                    break;
                }

                panic!("send() failed: {:?}", e);
            }

            println!("written {}", write);
        }

        if conn.is_closed() {
            println!("connection closed, {:?}", conn.stats());
            break;
        }
    }
}

fn hex_dump(buf: &[u8]) -> String {
    let vec: Vec<String> = buf.iter().map(|b| format!("{b:02x}")).collect();

    vec.join("")
}