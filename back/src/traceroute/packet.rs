use pnet::packet::icmp::echo_request::MutableEchoRequestPacket;
use pnet::packet::icmp::{self, IcmpPacket, IcmpTypes};
use pnet::packet::ip::IpNextHeaderProtocols;
use pnet::packet::ipv4::Ipv4Packet;
use pnet::packet::Packet;

const ECHO_PAYLOAD_LEN: usize = 16;
const ECHO_HEADER_LEN: usize = 8;

#[derive(Debug)]
pub enum IcmpResponse {
    TimeExceeded,
    EchoReply,
    Other,
}

pub fn build_echo_request(identifier: u16, sequence: u16) -> Vec<u8> {
    let mut buf = vec![0u8; ECHO_HEADER_LEN + ECHO_PAYLOAD_LEN];
    let mut packet = MutableEchoRequestPacket::new(&mut buf)
        .expect("buffer sized correctly for echo request");

    packet.set_icmp_type(IcmpTypes::EchoRequest);
    packet.set_icmp_code(icmp::echo_request::IcmpCodes::NoCode);
    packet.set_identifier(identifier);
    packet.set_sequence_number(sequence);

    let checksum = icmp::checksum(&IcmpPacket::new(packet.packet()).unwrap());
    packet.set_checksum(checksum);

    buf
}

pub fn parse_icmp_response(bytes: &[u8]) -> Option<IcmpResponse> {
    let ipv4 = Ipv4Packet::new(bytes)?;
    if ipv4.get_next_level_protocol() != IpNextHeaderProtocols::Icmp {
        return None;
    }

    let icmp = IcmpPacket::new(ipv4.payload())?;
    match icmp.get_icmp_type() {
        IcmpTypes::TimeExceeded => Some(IcmpResponse::TimeExceeded),
        IcmpTypes::EchoReply => Some(IcmpResponse::EchoReply),
        _ => Some(IcmpResponse::Other),
    }
}