#[crate_id="pcapfe#0.0"];
#[crate_type="lib"];
#[desc = "A Rust package wrapping libpcap (tested on Linux only)."];
#[license = "MIT"]; // sure, why not?(

#[feature(globs)];

extern mod std;

use std::cast;
use std::io::net::ip;
use std::io::net::ip::Ipv4Addr;
use std::io::util;
use std::libc::{c_uint,c_schar,c_void};
use std::ptr;
use std::str;
use std::u16;
use std::vec;

use pcap::*;
mod pcap;

pub static SIZE_ETHERNET_HEADER: uint = 14;
pub static SIZE_IP_HEADER_MIN: uint = 20;
pub static SIZE_IP_HEADER_MAX: uint = 80;
pub static SIZE_TCP_HEADER: uint = 1;
pub static SIZE_UDP_HEADER: uint = 8;

// Todo: probably a better way to design several pieces of this in general
// some way to chain these together or sumthin

pub enum PcapNextExError {
    BadState,
    ReadError,
    Timeout,
    EndOfCaptureFile,
    Unknown
}

pub enum PcapFilterError {
    DeviceClosed, // FIX: this is a dup of the above error. Consolidate these enums?
    CompileError,
    SetError
}

/*
pub struct MacAddress {
    // there must be a better way!
    Address: [u8, ..6]
}

pub fn mac_from_slice(sl: &[u8]) -> MacAddress {
    MacAddress{
        Address: [ 0x00, 0x01, 0x02, 0x03, 0x04, 0x05 ],
    }
}
*/

pub struct EthernetHeader {
    DstMac:     ~[u8],
    SrcMac:     ~[u8],
    Kind:       EthernetType,
}

pub enum EthernetType {
    EthernetType_IPv4,
    EthernetType_IPv6,
    EthernetType_Unknown,
}

pub struct Ipv4Header {
    Version:      uint,
    Ihl:          uint,
    DiffServices: uint,
    HeaderLength: uint,
    TotalLength:  uint,
    Id:           uint,
    Flags:        uint,
    FragOffset:   uint,
    Ttl:          uint,
    Protocol:     InternetProtocolNumbers,
    Checksum:     uint,
    SrcIp:        ip::IpAddr,
    DstIp:        ip::IpAddr,
}

pub struct Ipv6Header {
    Temp:        uint,
}

pub enum InternetProtocolNumbers {
    ICMP = 1,
    TCP = 6,
    UserDatagram = 17,
}

pub struct TcpHeader {
    SrcPort:     ip::Port,
    DstPort:     ip::Port,
    Seq:         uint,
    Ack:         uint,
    DataOffset:  uint,
    Flags:       uint,
    Window:      uint,
    Checksum:    uint,
    Urgent:      uint,
}

pub struct UdpHeader {
    SrcPort:     ip::Port,
    DstPort:     ip::Port,
    Length:      uint,
    Checksum:    uint,
}

pub enum DecodedPacket<'r> {
    InvalidPacket,
    EthernetPacket(EthernetHeader, &'r [u8]),
    IpPacket(EthernetHeader, Ipv4Header, &'r [u8]),
    TcpPacket(EthernetHeader, Ipv4Header, TcpHeader, &'r [u8]),
    UdpPacket(EthernetHeader, Ipv4Header, UdpHeader, &'r [u8]),
}

pub fn decode_ethernet_header(header_plus_payload: &[u8]) -> Option<(EthernetHeader, uint)> {
    
    println!("decode ethernet header");
    // TODO: Check size

    let dst_mac = header_plus_payload.slice(0, 6).to_owned();
    let src_mac = header_plus_payload.slice(6, 12).to_owned();
    //let type_ = unsafe { header_plus_payload.slice(13, 14).to_owned() as uint }; // TODO: Fix
    let type_ = 0x0400;
    let type_ = match type_ {
        // for lower ranges, it maps to a size, ugh. something EthernetHeader_Size<uint> ?
        0x0400 => { EthernetType_IPv4 }
        0x0800 => { EthernetType_IPv6 }
        _ => { EthernetType_Unknown }
    };
    let ether_hdr = EthernetHeader{
        DstMac: dst_mac,
        SrcMac: src_mac,
        Kind: type_,
    };
    Some((ether_hdr, 14))
}

// should these decodes just return their payload in the first place?
// wouldn't need to reslice them, the main nested parsing code gets cleaner.........

pub fn decode_ipv4_header(h: &[u8]) -> Option<(Ipv4Header, uint)> {

    println!("decode ipv4 header (len: {:?})", h.len());

    // TODO: Check size
    let byte1 = h[0];
    let byte2 = h[1];

    let version = 4;
    let ihl = (byte1 & 0b00001111) as uint;

    println!("decode ipv4 header (len: {:?})", ihl);


    let dscp = byte2 >> 2;
    let ecn = byte2 & 0b00000011;

    let total_len: u16 = h[2] as u16 << 8 | h[3] as u16;
    let id: u16 = h[4] as u16 << 8 | h[5] as u16;

    let flags = h[6] >> 5 & 0b00000111;
    //let frag_offset = 
    let frag_offset: u16 = h[6] as u16 << 8 | h[7] as u16;
    let frag_offset = 0b0001111111111111;
    let frag_offset = 0;

    let ttl = h[8];
    let proto = h[9];

    let checksum: u16 = h[10] as u16 << 8 | h[11] as u16;

    let src_ip = Ipv4Addr(h[12], h[13], h[14], h[15]);
    let dst_ip = Ipv4Addr(h[16], h[17], h[18], h[19]);


    if ihl > 15 {
            fail!("too many ihls");
    }

    if ihl > 5 {
        let options = h.slice(24, 4*(ihl-5));
    }

    let res = Ipv4Header{
        Version:      version as uint,
        Ihl:          ihl as uint,
        DiffServices: 0x0000, // WTF IS THIS EVEN?
        HeaderLength: ihl * 4 as uint,
        TotalLength:  total_len as uint,
        Id:           id as uint,
        Flags:        flags as uint,
        FragOffset:   frag_offset as uint,
        Ttl:          ttl as uint,
        Protocol:     match(proto) { 0x8000 => { TCP }, _ => { UserDatagram } },
        Checksum:     checksum as uint,
        SrcIp:        src_ip,
        DstIp:        dst_ip,
        // copy the remaining payload pointer here?
        // is that valuable?
        // honestly, might be nicer, in case you match a TCP Packet, but want the
        // Ethernet header and the rest of the raw bytes
        // ... otherwise, there really wouldn't be a way to access that easily through my API...
    };

    Some((res, ihl*4))
}

pub fn decode_ipv6_header(header_plus_payload: &[u8]) -> Option<(Ipv6Header, uint)> {
    None
}

pub fn decode_tcp_header(header_plus_payload: &[u8]) -> Option<(TcpHeader, uint)> {
    // TODO: Check length

    println!("decode tcp header");

    let res = TcpHeader{
        SrcPort:     80,
        DstPort:     80,
        Seq:         80,
        Ack:         80,
        DataOffset:  80,
        Flags:       80,
        Window:      80,
        Checksum:    80,
        Urgent:      80,
    };

    // TODO: Check checksum

    Some((res, 20))
}

pub fn decode_udp_header(h: &[u8]) -> Option<(UdpHeader, uint)> {
    if h.len() < 8 {
        return None
    }

    println!("---------------------");
    pp(h);
    println!("---------------------");

    let src_port: u16 = h[0] as u16 << 8 | h[1] as u16;
    let dst_port: u16 = h[2] as u16 << 8 | h[3] as u16;
    let length: u16   = h[4] as u16 << 8 | h[5] as u16;
    let checksum: u16 = h[6] as u16 << 8 | h[7] as u16;

    let res = UdpHeader{
        SrcPort:  src_port as ip::Port,
        DstPort:  dst_port as ip::Port,
        Length:   length   as uint,
        Checksum: checksum as uint,

        //Payload: h.slice_from(8),
    };

    // TODO: Check checksum

    Some((res, 8))
}

pub fn pp(p: &[u8]) {
    let p2 = p.map(|&e| if e > 31 && e < 127 { e } else { '.' as u8 });
    println!("print ascii-mapped bytes: {:?}", p2);
    let temp_payload = str::from_utf8_owned(p2);
    println!("print utf8 bytes        : {:?}", temp_payload);
}

pub fn DecodePacket<'r>(payload: &'r [u8]) -> DecodedPacket<'r> {
    let mut payload = payload;

    match decode_ethernet_header(payload) {
        Some((ether_hdr, ether_hdr_len)) => {
            payload = payload.slice_from(ether_hdr_len);
            match ether_hdr.Kind {
                EthernetType_IPv4 => match decode_ipv4_header(payload) {
                    Some((ip_hdr, ip_hdr_len)) => {
                        payload = payload.slice_from(ip_hdr_len);
                        match ip_hdr.Protocol {
                            TCP => match decode_tcp_header(payload) {
                                Some((tcp_hdr, tcp_hdr_len)) => {
                                    // tcp_hdr -> tcp_hdr.len()
                                    // return payload with it. attempt decode, etc
                                    payload = payload.slice_from(tcp_hdr_len);
                                    TcpPacket(ether_hdr, ip_hdr, tcp_hdr, payload)
                                }
                                None => { InvalidPacket }
                            },
                            UserDatagram => match decode_udp_header(payload) {
                                Some((udp_hdr, udp_hdr_len)) => {
                                    payload = payload.slice_from(udp_hdr_len);
                                    UdpPacket(ether_hdr, ip_hdr, udp_hdr, payload)
                                }
                                None => { InvalidPacket } // could let it fall through? does it matter?
                            },
                            _ => { InvalidPacket }
                        }
                    },
                    None => { InvalidPacket }
                },
                EthernetType_IPv6 => match decode_ipv6_header(payload) {
                    Some(_te) => { InvalidPacket }
                    None => { InvalidPacket }
                },
                _ => { return InvalidPacket; }
            }
        },
        None => {
            InvalidPacket // change the return type to opt or result
        }
    }
}

// pcap wrapper

pub struct PcapDevice {
    pcap_dev: *mut pcap_t,
    closed: bool
}

pub struct PcapPacket {
    timestamp: Struct_timeval,
    len: uint,
    payload: ~[u8]
}

pub fn PcapOpenDevice(dev: &str) -> Option<PcapDevice> {
    PcapOpenDeviceAdv(dev, 65536, 0, 1000)
}

pub fn PcapOpenDeviceAdv(dev: &str, size: int, flag: int, mtu: int) -> Option<PcapDevice> {
    unsafe {
        let mut errbuf: ~[c_schar] = vec::with_capacity(256);
        let c_dev = dev.to_c_str().unwrap();
        let handle = pcap_open_live(c_dev, size as i32, flag as i32, mtu as i32, errbuf.as_mut_ptr());
        // should probably do something with error buffer?
        if handle == ptr::mut_null() {
            None
        } else {
            let pd: PcapDevice = PcapDevice { pcap_dev: handle, closed: false };
            Some(pd)
        }
    }
}

pub fn PcapOpenOffline(filename: &str) -> Option<PcapDevice> {
    // do shit, return handle
    None
}

impl PcapDevice {
    pub fn SetFilter(&self, dev: &str, filter_str: &str) -> Result<(), PcapFilterError> {
        unsafe {
            if self.closed {
                return Err(DeviceClosed)
            }
            let mut errbuf: ~[c_schar] = vec::with_capacity(256);
            let mut netp: c_uint = 0;
            let mut maskp: c_uint = 0;
            let mut filter_program: Struct_bpf_program = std::unstable::intrinsics::uninit();

            let c_dev = dev.to_c_str().unwrap();
            let c_filter_str = filter_str.to_c_str().unwrap();
            
            pcap_lookupnet(c_dev, &mut netp, &mut maskp, errbuf.as_mut_ptr());
            let res = pcap_compile(self.pcap_dev, &mut filter_program, c_filter_str, 0, netp);
            if res != 0 {
                Err(CompileError)
            } else {
                let res = pcap_setfilter(self.pcap_dev, &mut filter_program);
                if res != 0 {
                    Err(SetError) // how to set the errorbuf msg in the SetError somehow?
                } else {
                    Ok(())
                }
            }
        }
    }

    pub fn NextPacketEx(&self) -> Result<PcapPacket, PcapNextExError> {
        if self.closed {
            Err(BadState)
        } else {
            unsafe {
                let mut pkthdr_ptr: *mut Struct_pcap_pkthdr = std::unstable::intrinsics::uninit();
                let mut pkt_data_ptr: *u8 = std::unstable::intrinsics::uninit();

                let result = pcap_next_ex(self.pcap_dev, &mut pkthdr_ptr, &mut pkt_data_ptr);

                let pkt_len: uint = (*pkthdr_ptr).len as uint;
                match result {
                    -2 => { Err(EndOfCaptureFile) }
                    -1 => { Err(ReadError) } // call pcap_geterr() or pcap_perror()
                    0 => { Err(Timeout) }
                    1 => {
                        if pkt_len == 0 {
                            println!("ignoring zero length packet"); 
                            Err(Unknown)
                        } else {
                            let payload = std::vec::from_buf(pkt_data_ptr, pkt_len); // does this copy? pkt_data_ptr's location is reused
                            let pkt = PcapPacket{
                                timestamp: (*pkthdr_ptr).ts,
                                len: pkt_len,
                                payload: payload
                            };
                            Ok(pkt)
                        }
                    }
                    _ => { Err(Unknown) }
                }
            }
        }
    }

    pub fn Inject(&self, pkt: DecodedPacket) -> Result<(), ()> {
        unsafe {
            match pkt {
                TcpPacket(ehdr, ihdr, thdr, pkt) => {
                    println!("tcp, {:?}", pkt);
                    let data: ~[u8] = ~[0x00, 0x01, 0x02]; // TODO: FIX
                    let data1 = unsafe { data.as_ptr() as *c_void };
                    let size1 = unsafe { data.len() as u64 };
                    let result = pcap_inject(self.pcap_dev, data1, size1);
                    match result {
                        -1 => {}
                        0 => {}
                        1 => {}
                        _ => { fail!("stop touching yourself pcap"); }
                    }
                }
                _ => {

                }
            };
        }
        Ok(())
    }

    // Should this be impl Drop for PcapDevice?
    pub fn Close(&mut self) {
        unsafe {
            self.closed = true;
            pcap_close(self.pcap_dev);
        }
    }
}