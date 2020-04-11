use std::io::Cursor;
use std::net::SocketAddr;
use std::time::{Duration, Instant};
use std::sync::Arc;

use rand::Rng;
use rand_distr::Pareto;

use aquatic::handlers::*;
use aquatic::common::*;
use aquatic_bench::*;
use bittorrent_udp::converters::*;

use crate::common::*;


const SCRAPE_REQUESTS: usize = 1_000_000;
const SCRAPE_NUM_HASHES: usize = 10;


pub fn bench(
    state: &State,
    requests: Arc<Vec<([u8; MAX_REQUEST_BYTES], SocketAddr)>>
) -> (usize, Duration){
    let mut buffer = [0u8; MAX_PACKET_SIZE];
    let mut cursor = Cursor::new(buffer.as_mut());
    let mut num_responses: usize = 0;
    let mut dummy = 0u8;

    let now = Instant::now();

    let mut requests: Vec<(ScrapeRequest, SocketAddr)> = requests.iter()
        .map(|(request_bytes, src)| {
            if let Request::Scrape(r) = request_from_bytes(request_bytes, 255).unwrap() {
                (r, *src)
            } else {
                unreachable!()
            }
        })
        .collect();
    
    let requests = requests.drain(..);

    handle_scrape_requests(
        &state,
        requests,
    );

    while let Ok((response, _)) = state.response_queue.pop(){
        if let Response::Scrape(_) = response {
            num_responses += 1;
        }

        cursor.set_position(0);

        response_to_bytes(&mut cursor, response, IpVersion::IPv4).unwrap();

        dummy ^= cursor.get_ref()[0];
    }

    let duration = Instant::now() - now;

    assert_eq!(num_responses, SCRAPE_REQUESTS);

    if dummy == 123u8 {
        println!("dummy info");
    }

    (SCRAPE_REQUESTS, duration)
}


pub fn create_requests(
    rng: &mut impl Rng,
    info_hashes: &Vec<InfoHash>
) -> Vec<(ScrapeRequest, SocketAddr)> {
    let pareto = Pareto::new(1., PARETO_SHAPE).unwrap();

    let max_index = info_hashes.len() - 1;

    let mut requests = Vec::new();

    for _ in 0..SCRAPE_REQUESTS {
        let mut request_info_hashes = Vec::new();

        for _ in 0..SCRAPE_NUM_HASHES {
            let info_hash_index = pareto_usize(rng, pareto, max_index);
            request_info_hashes.push(info_hashes[info_hash_index])
        }

        let request = ScrapeRequest {
            connection_id: ConnectionId(rng.gen()),
            transaction_id: TransactionId(rng.gen()),
            info_hashes: request_info_hashes,
        };

        let src = SocketAddr::from(([rng.gen(), rng.gen(), rng.gen(), rng.gen()], rng.gen()));

        requests.push((request, src));
    }

    requests
}