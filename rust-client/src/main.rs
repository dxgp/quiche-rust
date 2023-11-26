use quiche;
use ring::rand::*;
const MAX_DATAGRAM_SIZE: usize = 1350;
fn main(){
    // let mut config = quiche::Config::new(quiche::PROTOCOL_VERSION)?;
    // config.set_application_protos(&[b"example-proto"]);
    let mut buf = [0;65535];
    let mut out = [5;MAX_DATAGRAM_SIZE];

    
    
    let mut config = quiche::Config::new(quiche::PROTOCOL_VERSION).unwrap();
    config.set_application_protos(&[b"example-proto"]);
    /*
        The config object is what we use to control QUIC Version, ALPN IDs, flow control, congestion control
        idle timeout etc.
     */
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
    
    let (write,send_info) = conn.send(&mut out).expect("initial send failed.");

    while let Err(e) = socket.send_to(&out[..write], send_info.to) {
        if e.kind() == std::io::ErrorKind::WouldBlock {
            println!("send() would block");
            continue;
        }
        panic!("send() failed: {:?}", e);
    }
    println!("initial packet sent");

    let req_start = std::time::Instant::now();
    let mut req_sent = false;

    loop{
        poll.poll(&mut events, conn.timeout()).unwrap();
        'read: loop{
            if events.is_empty() {
                println!("Timed out");
                conn.on_timeout();
                break 'read;
            }
            let (len,from) = match socket.recv_from(&mut buf){
                Ok(v) => v,
                Err(e) => {
                    if e.kind() == std::io::ErrorKind::WouldBlock {
                        println!("recv() would block");
                        break 'read;
                    }
                    panic!("RECV FAILED: {:?}",e);
                },
            };
            println!("Got {} bytes",len);

            let recv_info = quiche::RecvInfo{
                to: socket.local_addr().unwrap(),
                from,
            };

            let read = match conn.recv(&mut buf[..len], recv_info){
                Ok(v)=>v,
                Err(e) => {
                    println!("RECV FAILED: {:?}",e);
                    continue 'read;
                },
            };

            println!("Processed {} bytes",read);
        }

        println!("Done reading");
        if conn.is_closed() {
            println!("Connection closed, {:?}",conn.stats());
            break;
        }
        println!("INITIAL CONN ESTABLISHED:{}",conn.is_established());
        println!("Request not sent = {}",!req_sent);
        if conn.is_established() && !req_sent {
            conn.send(&mut out);
            req_sent = true;
            println!("Conn established and req not send. Sent request");
        }

        loop{
            println!("Beg. of this loop. Conn established:{}",conn.is_established());
            let (write,send_info) = match conn.send(&mut out){
                Ok(v)=>v,
                Err(quiche::Error::Done)=>{
                    println!("Done writing");
                    break;
                },
                Err(e) => {
                    println!("send failed: {:?}",e);
                    conn.close(false, 0x1, b"fail").ok();
                    break;
                },
            };
            if let Err(e) = socket.send_to(&out[..write], send_info.to){
                if e.kind() == std::io::ErrorKind::WouldBlock {
                    println!("send() would block");
                    break;
                }
                panic!("send() failed: {:?}", e);
            }
            println!("written {}", write);
            if conn.is_closed() {
                println!("connection closed, {:?}", conn.stats());
                break;
            }
        }
    }
    
}