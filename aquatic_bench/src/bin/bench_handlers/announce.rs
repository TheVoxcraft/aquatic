use std::io::Cursor;
use std::time::{Duration, Instant};
use std::net::SocketAddr;
use std::sync::Arc;

use rand::{Rng, SeedableRng, thread_rng, rngs::SmallRng};
use rand_distr::Pareto;

use aquatic::handlers::*;
use aquatic::common::*;
use aquatic::config::Config;
use aquatic_bench::*;
use bittorrent_udp::converters::*;

use crate::common::*;


const ANNOUNCE_REQUESTS: usize = 1_000_000;


pub fn bench(
    state: &State,
    config: &Config,
    requests: Arc<Vec<([u8; MAX_REQUEST_BYTES], SocketAddr)>>
) -> (usize, Duration){
    let mut buffer = [0u8; MAX_PACKET_SIZE];
    let mut cursor = Cursor::new(buffer.as_mut());
    let mut num_responses: usize = 0;
    let mut dummy = 0u8;

    let mut small_rng = SmallRng::from_rng(thread_rng()).unwrap();

    let now = Instant::now();

    let mut requests: Vec<(AnnounceRequest, SocketAddr)> = requests.iter()
        .map(|(request_bytes, src)| {
            if let Request::Announce(r) = request_from_bytes(request_bytes, 255).unwrap() {
                (r, *src)
            } else {
                unreachable!()
            }
        })
        .collect();
    
    let requests = requests.drain(..);

    handle_announce_requests(
        &state,
        config,
        &mut small_rng,
        requests,
    );
    
    while let Ok((response, src)) = state.response_queue.pop(){
        if let Response::Announce(_) = response {
            num_responses += 1;
        }

        cursor.set_position(0);

        response_to_bytes(&mut cursor, response, IpVersion::IPv4).unwrap();

        dummy ^= cursor.get_ref()[0];
    }

    let duration = Instant::now() - now;

    assert_eq!(num_responses, ANNOUNCE_REQUESTS);

    if dummy == 123u8 {
        println!("dummy info");
    }

    (ANNOUNCE_REQUESTS, duration)
}



pub fn create_requests(
    rng: &mut impl Rng,
    info_hashes: &Vec<InfoHash>
) -> Vec<(AnnounceRequest, SocketAddr)> {
    let pareto = Pareto::new(1., PARETO_SHAPE).unwrap();

    let max_index = info_hashes.len() - 1;

    let mut requests = Vec::new();

    for _ in 0..ANNOUNCE_REQUESTS {
        let info_hash_index = pareto_usize(rng, pareto, max_index);

        let request = AnnounceRequest {
            connection_id: ConnectionId(rng.gen()),
            transaction_id: TransactionId(rng.gen()),
            info_hash: info_hashes[info_hash_index],
            peer_id: PeerId(rng.gen()),
            bytes_downloaded: NumberOfBytes(rng.gen()),
            bytes_uploaded: NumberOfBytes(rng.gen()),
            bytes_left: NumberOfBytes(rng.gen()),
            event: AnnounceEvent::Started,
            ip_address: None, 
            key: PeerKey(rng.gen()),
            peers_wanted: NumberOfPeers(rng.gen()),
            port: Port(rng.gen())
        };

        let src = SocketAddr::from(([rng.gen(), rng.gen(), rng.gen(), rng.gen()], rng.gen()));

        requests.push((request, src));
    }

    requests
}