use std::net::{Ipv4Addr, SocketAddr};
use std::net::{UdpSocket, TcpStream, TcpListener};

use std::sync::{Arc, Mutex};
use std::env::args;
use std::io::{self, Write, Read};
use std::time::Duration;
use std::thread::{self, sleep};
use std::str::FromStr;

fn main() {
    println!("Network Multicast Tester");
    println!("-----------------------------");

    println!("Please enter this systems in-use IPv4 address (Come on winapi...): ");
    let (bind_addr, local_ip) = loop {
        let mut local_ip_addr = String::new();
        io::stdin().read_line(&mut local_ip_addr).unwrap();
        let local_ip_addr = String::from(local_ip_addr.trim_end_matches(|c| c == '\n' || c == '\r'));
        match Ipv4Addr::from_str(&local_ip_addr) {
            Ok(addr) => {
                break (SocketAddr::from((addr, 14000)), addr);
            }
            Err(_) => {
                println!("An improper IP address was entered, please try again!");
            }
        }
    };

    loop {
        if args().len() == 0 | 1 {
            println!("No mode specified, falling back to selection!");
            println!("Enter 'S' for broadcaster mode or 'R' for receiver mode: ");

            let mut mode_choice = String::new();
            io::stdin().read_line(&mut mode_choice).unwrap();
            let mode_choice = String::from(mode_choice.trim_end_matches(|c| c == '\n' || c == '\r'));

            match mode_choice.to_uppercase().as_str() {
                "S" => launch_broadcaster(bind_addr),
                "R" => launch_receiver(bind_addr, local_ip),
                "" => {
                    println!("Exiting!"); 
                    break
                }
                _ => {}
            }

        } else { break }
    }

    for argument in args() {
        // Skip the default executable path
        if args().nth(0) == Some(argument.to_owned()) { continue };
        if argument == "--sender" {
            launch_broadcaster(bind_addr);
        } else if argument == "--receiver" {
            launch_receiver(bind_addr, local_ip);
        }
        else {
            println!("Improper arguments received, try again!")
        }
    }
}

fn launch_broadcaster(bind_addr: SocketAddr) {
    let response_counter: Arc<Mutex<[u8; 10]>> = Arc::new(Mutex::from([0; 10]));

    println!("-----------------------------");
    println!("Testing as the broadcaster!");
    println!("Make sure the listener is ready before starting");

    let listener_rcounter = response_counter.clone();
    thread::Builder::new().name("response_listener".to_string()).spawn(move || {
        response_listener(bind_addr, response_counter)
    }).ok();

    multi_broadcaster(bind_addr, listener_rcounter);
}

fn launch_receiver(bind_addr: SocketAddr, local_ip: Ipv4Addr) {
    println!("-----------------------------");
    println!("Testing as the receiver!");
    println!("-----------------------------");
    multi_receiver(bind_addr, local_ip);
}

fn multi_broadcaster(bind_addr: SocketAddr, response_counter: Arc<Mutex<[u8; 10]>>) {
    println!("Press any key to start tests: ");
    let mut test_start = String::new();
    io::stdin().read_line(&mut test_start).unwrap();

    println!("Creating announcement socket...");
    let announcement_socket = UdpSocket::bind(bind_addr).expect("Failed to bind to port!");

    println!("Sending a set of 10 multicast packets...");
    println!("-----------------------------");

    let test_packetdata = "General Kenobi!".as_bytes();
    for number in 0..10 {
        if number != 9 {
            announcement_socket.send_to(test_packetdata, "239.0.0.3:14000").expect("Failed to multicast packet!");
        } else {
            // Let the caster know this is the last packet
            announcement_socket.send_to("Until next time!".as_bytes(), "239.0.0.3:14000").expect("Failed to multicast final packet!");
        }
        
        println!("Sending packet {}...", number + 1);
        sleep(Duration::new(1, 0)); // Little bit of a delay to avoid sending them all under exact conditions
    }

    println!("-----------------------------");
    let mut packet_counter = 0;
    let mut total_valid_packets = 0;
    for packet_response in response_counter.lock().unwrap().iter() {
        let packet_is_valid = match packet_response {
            0 => {
                packet_counter += 1;
                "No Response"
            }
            1 => {
                packet_counter += 1;
                total_valid_packets += 1;
                "Had Response"
            }
            &_ => "" // This should never trigger
        };
        println!("Packet {}: {}", packet_counter, packet_is_valid)
    }
    println!("-----------------------------");
    println!("We saw responses to {}/10 of the multicast packets", total_valid_packets);
    match total_valid_packets {
        10 => println!("Multicast across the two devices is working!"),

        0 => {
            println!("Multicast across the two devices is not working...");
            println!("If you're trying to multicast from a wireless device to Ethernet, then this is probably to be expected, sadly.");
        }

        _ => {
            println!("Multicast appears to be partially working, though there is some packet loss.");
            println!("If you're testing multicasts between wireless and wired devices, this is a common occurance.");
        }
    }

    println!("-----------------------------");
    println!("Test completed, press any key to exit");
    let mut exit_now = String::new();
    io::stdin().read_line(&mut exit_now).unwrap();
}

fn multi_receiver(bind_addr: SocketAddr, local_ip: Ipv4Addr) {
    let receiver_socket = UdpSocket::bind(bind_addr).expect("Failed to bind to port!");
    receiver_socket.join_multicast_v4(&Ipv4Addr::new(239, 0, 0, 3), &local_ip).unwrap();

    let mut recv_buffer: Vec<u8> = vec![0; 65536];
    let mut packet_counter = 0;
    loop {
         if packet_counter >= 10 {
            // We got the last packet, exit now
            println!("-----------------------------");
            println!("Test completed, press any key to exit!");
            let mut exit_now = String::new();
            io::stdin().read_line(&mut exit_now).unwrap();
            break 
        }
        let (_, caster_ip) = receiver_socket.recv_from(&mut recv_buffer).unwrap();
        packet_counter += 1;
        println!("Received a packet from the broadcaster!");
        
        let mut responder = TcpStream::connect(caster_ip).expect("Failed to conenct to caster!");
        match responder.write("Hello There!".as_bytes()) {
            Ok(_) => {}
            Err(_) => println!("Failed to respond to caster, please check your connection!")
        }  
    }
}

fn response_listener(bind_addr: SocketAddr, response_counter: Arc<Mutex<[u8; 10]>>) {
    let listener = TcpListener::bind(bind_addr).expect("Failed to bind TCP listener!");

    let mut packet_iterator_count = 0;
    loop {
        if packet_iterator_count >= 10  {
            break // Shutdown this thread only
        }
        match listener.accept() {
            Ok((mut stream, _)) => {
                let mut response = String::new();
                stream.read_to_string(&mut response).unwrap();

                // Mark the packet's received status to true
                response_counter.lock().unwrap()[packet_iterator_count] = 1;
                packet_iterator_count += 1;
            }
            Err(e) => println!("{:?}", e)
        }
    }
}
