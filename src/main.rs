use std::net::{Ipv4Addr, SocketAddr};
use std::net::{TcpListener, TcpStream, UdpSocket};

use std::env;
use std::fmt;
use std::io::{self, Read, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

const SENDER_MODE_FLAG: &str = "--sender";
const RECEIVER_MODE_FLAG: &str = "--receiver";

const GREETER_TEST_PACKET_DATA: &[u8] = b"General Kenobi!";
const ACK_TEST_PACKET_DATA: &[u8] = b"Hello there!";
const FINALE_TEST_PACKET_DATA: &[u8] = b"Until next time!";
const MULTICAST_IP: &str = "239.0.0.3:14000";

#[derive(Debug)]
enum Error {
    SocketBind(io::Error),
    Connection(io::Error),
    ReadData,
    SendData,
    BadArguments,
    StdIn,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::SocketBind(e) => write!(f, "Failed to bind to a socket: {}", e),
            Error::Connection(e) => write!(f, "Failed to connect to a peer: {}", e),
            Error::ReadData => write!(f, "Failed to read data from a peer!"),
            Error::SendData => write!(
                f,
                "Failed to respond with data to a caster, check your connection!"
            ),
            Error::BadArguments => write!(f, "Invalid CLI arguments were supplied!"),
            Error::StdIn => write!(f, "Failed to read data from the command line!"),
        }
    }
}

fn main() {
    if let Err(e) = app_main() {
        eprintln!("An error occured: {}", e)
    }
}

fn app_main() -> Result<(), Error> {
    println!("Network Multicast Tester");
    terminal_seperator();

    println!("Please enter this systems in-use IPv4 address (Come on winapi...): ");
    let bind_addr = loop {
        let local_ip_addr = read_user_string()?;

        let local_ip_addr = local_ip_addr.trim_end_matches(|c| c == '\n' || c == '\r');
        if let Ok(addr) = local_ip_addr.parse::<Ipv4Addr>() {
            break SocketAddr::from((addr, 14000));
        } else {
            eprintln!("An improper IP address was entered, please try again!");
        }
    };

    let cmd_args = env::args();
    if cmd_args.len() < 2 {
        loop {
            println!("No mode specified, falling back to selection!");
            println!("Enter 'S' for broadcaster mode or 'R' for receiver mode: ");

            let mode_choice = read_user_string()?;
            let mode_choice = mode_choice.trim_end_matches(|c| c == '\n' || c == '\r');
            match mode_choice.to_uppercase().as_str() {
                "S" => return launch_broadcaster(bind_addr),
                "R" => return launch_receiver(bind_addr),
                _ => continue,
            }
        }
    }

    let mode_arg = env::args().nth(1).ok_or(Error::BadArguments)?;
    match mode_arg.as_str() {
        SENDER_MODE_FLAG => launch_broadcaster(bind_addr),
        RECEIVER_MODE_FLAG => launch_receiver(bind_addr),
        _ => Err(Error::BadArguments),
    }
}

fn launch_broadcaster(bind_addr: SocketAddr) -> Result<(), Error> {
    let response_counter = Arc::new(Mutex::from([0; 10]));

    terminal_seperator();
    println!("Testing as the broadcaster!");
    println!("Make sure the listener is ready before starting");

    let listener_rcounter = response_counter.clone();
    let res = thread::Builder::new()
        .name("response_listener".to_string())
        .spawn(move || {
            let listener = TcpListener::bind(bind_addr).map_err(Error::SocketBind)?;

            for pc in 0..10 {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        let mut response = String::new();
                        stream
                            .read_to_string(&mut response)
                            .map_err(|_| Error::ReadData)?;

                        // Mark the packet's received status to true
                        response_counter.lock().unwrap()[pc] = 1;
                    }
                    Err(_) => return Err(Error::ReadData),
                }
            }

            Ok(())
        })
        .unwrap();

    multi_broadcaster(bind_addr, listener_rcounter)?;

    res.join().unwrap()
}

fn launch_receiver(bind_addr: SocketAddr) -> Result<(), Error> {
    terminal_seperator();
    println!("Testing as the receiver!");
    terminal_seperator();

    let receiver_socket = UdpSocket::bind(bind_addr).map_err(Error::SocketBind)?;

    let recv_address = match bind_addr.ip() {
        std::net::IpAddr::V4(addr) => addr,
        _ => unreachable!(),
    };

    receiver_socket
        .join_multicast_v4(&Ipv4Addr::new(239, 0, 0, 3), &recv_address)
        .map_err(Error::SocketBind)?;

    let mut recv_buffer: Vec<u8> = vec![0; 65536];
    for _ in 0..10 {
        let (_, caster_ip) = receiver_socket
            .recv_from(&mut recv_buffer)
            .map_err(|_| Error::ReadData)?;

        println!("Received a packet from the broadcaster!");

        let mut responder = TcpStream::connect(caster_ip).map_err(Error::Connection)?;
        responder
            .write(ACK_TEST_PACKET_DATA)
            .map_err(|_| Error::SendData)?;
    }

    // We got the last packet, exit now
    terminal_seperator();
    println!("Test completed, press any key to exit!");
    read_user_string()?;
    Ok(())
}

fn multi_broadcaster(
    bind_addr: SocketAddr,
    response_counter: Arc<Mutex<[u8; 10]>>,
) -> Result<(), Error> {
    println!("Press any key to start tests: ");
    read_user_string()?;

    println!("Creating announcement socket...");
    let announcement_socket = UdpSocket::bind(bind_addr).map_err(Error::SocketBind)?;

    println!("Sending a set of 10 multicast packets...");
    terminal_seperator();

    for number in 0..10 {
        if number != 9 {
            announcement_socket
                .send_to(GREETER_TEST_PACKET_DATA, MULTICAST_IP)
                .map_err(|_| Error::SendData)?;
        } else {
            // Let the caster know this is the last packet
            announcement_socket
                .send_to(FINALE_TEST_PACKET_DATA, MULTICAST_IP)
                .map_err(|_| Error::SendData)?;
        }

        println!("Sending packet {}...", number + 1);
        thread::sleep(Duration::from_secs(1)); // Little bit of a delay to avoid sending them all under exact conditions
    }

    terminal_seperator();
    let mut total_valid_packets = 0;
    for (p_count, packet_response) in response_counter.lock().unwrap().iter().enumerate() {
        let packet_is_valid = match packet_response {
            0 => "No Response",
            1 => {
                total_valid_packets += 1;
                "Had Response"
            }
            &_ => unreachable!(),
        };

        println!("Packet {}: {}", p_count, packet_is_valid)
    }

    println!(
        "We saw responses to {}/10 of the multicast packets",
        total_valid_packets
    );

    match total_valid_packets {
        10 => println!("Multicast across the two devices is working!"),

        0 => {
            println!("Multicast across the two devices is not working...");
            println!("If you're trying to multicast from a wireless device to Ethernet, then this is probably to be expected, sadly.");
        }

        _ => {
            println!(
                "Multicast appears to be partially working, though there is some packet loss."
            );
            println!("If you're testing multicasts between wireless and wired devices, this is a common occurance.");
        }
    }

    terminal_seperator();
    println!("Test completed, press any key to exit...");
    read_user_string()?;
    Ok(())
}

fn read_user_string() -> Result<String, Error> {
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|_| Error::StdIn)?;
    Ok(input)
}

fn terminal_seperator() {
    println!("------------------------");
}
