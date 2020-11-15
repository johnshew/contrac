use std::net::IpAddr;
use winping::{Buffer, Pinger};

fn main() {
    let ip_arg = std::env::args().nth(1);
    let ip_text = ip_arg.unwrap_or(String::from("8.8.8.8"));
    let ip_addr = ip_text.parse::<IpAddr>().expect("Could not parse IP Address");

    let dst = std::env::args()
        .nth(1)
        .unwrap_or(String::from("1.1.1.1"))
        .parse::<IpAddr>()
        .expect("Could not parse IP Address");

    println!("{}",dst);

    let pinger = Pinger::new().unwrap();
    let mut buffer = Buffer::new();

    for _ in 0..4 {
        match pinger.send(dst, &mut buffer) {
            Ok(rtt) => {
                println!("Response time {} ms.", rtt);
            }
            Err(err) => println!("{}.", err),
        }
    }
    println!("Hello, world!");
}
