use pnet::packet::icmp::echo_reply::EchoReplyPacket;
use pnet::packet::icmp::echo_request::{EchoRequestPacket, MutableEchoRequestPacket};
use pnet::packet::icmp::time_exceeded::TimeExceededPacket;
use pnet::packet::icmp::{self, IcmpPacket, IcmpTypes};
use pnet::packet::ip::IpNextHeaderProtocols;
use pnet::packet::ipv4::Ipv4Packet;
use pnet::packet::Packet;

const ECHO_PAYLOAD_LEN: usize = 16;
const ECHO_HEADER_LEN: usize = 8;

#[derive(Debug, PartialEq)]
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

/// Parses a received IP datagram and checks it's a response to *our*
/// probe. Returns `None` when the packet isn't related to us (wrong
/// ICMP type, or an identifier/sequence mismatch), so the caller can
/// keep waiting for the real response. A raw ICMP socket receives every
/// ICMP message on the host, so without this correlation a stray packet
/// could be misattributed to a hop.
pub fn parse_icmp_response(bytes: &[u8], identifier: u16, sequence: u16) -> Option<IcmpResponse> {
    let ipv4 = Ipv4Packet::new(bytes)?;
    if ipv4.get_next_level_protocol() != IpNextHeaderProtocols::Icmp {
        return None;
    }

    let icmp = IcmpPacket::new(ipv4.payload())?;
    match icmp.get_icmp_type() {
        IcmpTypes::EchoReply => {
            let reply = EchoReplyPacket::new(ipv4.payload())?;
            if reply.get_identifier() == identifier && reply.get_sequence_number() == sequence {
                Some(IcmpResponse::EchoReply)
            } else {
                None
            }
        }
        IcmpTypes::TimeExceeded => {
            // The message embeds the original datagram: the IPv4 header
            // we sent followed by the first 8 bytes of our echo request —
            // exactly where the identifier and sequence live. pnet's
            // TimeExceededPacket handles the error header layout (including
            // the 4-byte unused field) and exposes the original datagram
            // via payload().
            let time_exceeded = TimeExceededPacket::new(ipv4.payload())?;
            let embedded = time_exceeded.payload();
            let embedded_ip = Ipv4Packet::new(embedded)?;
            let ip_header_len = embedded_ip.get_header_length() as usize * 4;
            let request = EchoRequestPacket::new(embedded.get(ip_header_len..)?)?;
            if request.get_identifier() == identifier && request.get_sequence_number() == sequence {
                Some(IcmpResponse::TimeExceeded)
            } else {
                None
            }
        }
        _ => Some(IcmpResponse::Other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal valid IPv4 header, big-endian. `payload` is the ICMP
    /// message; total length accounts for header + payload.
    fn ipv4_wrap(payload: &[u8], source: [u8; 4], dest: [u8; 4], protocol: u8) -> Vec<u8> {
        let total_len = (20 + payload.len()) as u16;
        let mut hdr = vec![0u8; 20];
        hdr[0] = 0x45; // version 4, ihl 5 words
        hdr[2..4].copy_from_slice(&total_len.to_be_bytes());
        hdr[8] = 64; // ttl
        hdr[9] = protocol;
        hdr[12..16].copy_from_slice(&source);
        hdr[16..20].copy_from_slice(&dest);
        hdr.extend_from_slice(payload);
        hdr
    }

    /// The ICMP TimeExceeded error header, then the original datagram
    /// (embedded IPv4 header + first 8 bytes of our echo request).
    fn time_exceeded_msg(orig_echo: &[u8]) -> Vec<u8> {
        let mut msg = vec![0u8; 8];
        msg[0] = IcmpTypes::TimeExceeded.0;
        msg.extend_from_slice(&ipv4_wrap(orig_echo, [8, 8, 8, 8], [10, 0, 0, 1], IpNextHeaderProtocols::Icmp.0));
        msg
    }

    #[test]
    fn echo_reply_matching_probe_recognized() {
        let bytes = ipv4_wrap(
            &[0, 0, 0, 0, 0x12, 0x34, 0x00, 0x01], // EchoReply hdr: id 0x1234 seq 1
            [8, 8, 8, 8],
            [10, 0, 0, 1],
            IpNextHeaderProtocols::Icmp.0,
        );
        assert!(matches!(
            parse_icmp_response(&bytes, 0x1234, 1),
            Some(IcmpResponse::EchoReply)
        ));
    }

    #[test]
    fn echo_reply_wrong_identifier_rejected() {
        let bytes = ipv4_wrap(
            &[0, 0, 0, 0, 0x12, 0x34, 0x00, 0x01],
            [8, 8, 8, 8],
            [10, 0, 0, 1],
            IpNextHeaderProtocols::Icmp.0,
        );
        assert_eq!(parse_icmp_response(&bytes, 0xFFFF, 1), None);
    }

    #[test]
    fn echo_reply_wrong_sequence_rejected() {
        let bytes = ipv4_wrap(
            &[0, 0, 0, 0, 0x12, 0x34, 0x00, 0x01],
            [8, 8, 8, 8],
            [10, 0, 0, 1],
            IpNextHeaderProtocols::Icmp.0,
        );
        assert_eq!(parse_icmp_response(&bytes, 0x1234, 7), None);
    }

    #[test]
    fn time_exceeded_matching_probe_recognized() {
        let echo = build_echo_request(0x1234, 3);
        let te = time_exceeded_msg(&echo);
        let bytes = ipv4_wrap(
            &te,
            [192, 168, 1, 1], // the router that sent the error
            [10, 0, 0, 1],
            IpNextHeaderProtocols::Icmp.0,
        );
        assert!(matches!(
            parse_icmp_response(&bytes, 0x1234, 3),
            Some(IcmpResponse::TimeExceeded)
        ));
    }

    #[test]
    fn time_exceeded_wrong_identifier_rejected() {
        let echo = build_echo_request(0x1234, 3);
        let te = time_exceeded_msg(&echo);
        let bytes = ipv4_wrap(
            &te,
            [192, 168, 1, 1],
            [10, 0, 0, 1],
            IpNextHeaderProtocols::Icmp.0,
        );
        assert_eq!(parse_icmp_response(&bytes, 0x5678, 3), None);
    }

    #[test]
    fn non_icmp_packet_rejected() {
        let bytes = ipv4_wrap(&[0u8; 8], [8, 8, 8, 8], [10, 0, 0, 1], IpNextHeaderProtocols::Udp.0);
        assert_eq!(parse_icmp_response(&bytes, 0x1234, 1), None);
    }
}