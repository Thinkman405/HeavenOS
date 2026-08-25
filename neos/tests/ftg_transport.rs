//! End-to-end transport: a Frame carried along a computed route.
//!
//! The first assertions in NEOS that exercise more than one subsystem at once —
//! `lattice`'s metric chooses the path, `substrate`'s transduction carries the
//! wave across each hop, and `ftg`'s framing decides whether it survives.
//!
//! Doctrine: `_mkb/test-doctrine.md`. **[D]** marks assertions a conventional
//! packet-forwarding stack could not pass.

use ftg::layers_1_2::DISSONANCE_FLOOR;
use ftg::transport::{Delivery, Gateway, Packet};
use ftg::{Frame, FtgError, NetAddress};

fn gateway() -> Gateway {
    Gateway::new(4)
}

/// A 441-cell patch, for the tests that need long routes or large samples.
///
/// Depth 4 reaches only 4 hops from the origin and yields ~29 sampled pairs,
/// which is too thin to claim much. Depth 5 reaches 5 and gives ~180.
fn deep_gateway() -> Gateway {
    Gateway::new(5)
}

/// A route with at least `n` hops, so multi-hop behaviour is actually exercised.
fn multi_hop_packet(g: &Gateway, payload: &[u8], min_hops: usize) -> Packet {
    let cells = g.router().cells();
    let src = cells[0];
    for &dst in cells.iter().rev() {
        if let Ok(path) = g.router().route(src, dst, 200) {
            if path.len() - 1 >= min_hops {
                return Packet::new(Frame::encode(payload), src, dst);
            }
        }
    }
    panic!("no route of at least {min_hops} hops in this patch");
}

// ------------------------------------------------- Group 1: clean delivery

/// 1.1 [D] — a payload survives a multi-hop route **byte-identical**.
///
/// Every hop is a real transduction: phases onto the carrier, sampled, and
/// recovered. Nothing is copied by reference.
#[test]
fn payload_survives_a_multi_hop_route() {
    let g = gateway();
    for payload in [
        b"NEOS".to_vec(),
        vec![0x00, 0xFF, 0xAA, 0x55],
        b"the whole message, end to end".to_vec(),
        (0..=255u8).collect(),
    ] {
        let packet = multi_hop_packet(&g, &payload, 3);
        let delivery = g.deliver(&packet, 200).expect("route exists");
        match delivery {
            Delivery::Arrived { payload: got, hops, .. } => {
                assert!(hops >= 3, "expected a multi-hop route, got {hops}");
                assert_eq!(got, payload, "payload changed in flight");
            }
            other => panic!("clean frame must arrive, got {other:?}"),
        }
    }
}

/// 1.2 — the delivery path is exactly the route the router computes.
///
/// Transport must not quietly take a different path from the one routing
/// reports.
#[test]
fn delivery_path_matches_the_computed_route() {
    let g = gateway();
    let packet = multi_hop_packet(&g, b"path", 3);
    let expected = g
        .router()
        .route(packet.source(), packet.destination(), 200)
        .unwrap();

    match g.deliver(&packet, 200).unwrap() {
        Delivery::Arrived { path, hops, .. } => {
            assert_eq!(path, expected);
            assert_eq!(hops, expected.len() - 1);
        }
        other => panic!("expected arrival, got {other:?}"),
    }
}

/// 1.3 [D] — delivery takes the BFS-optimal number of hops.
///
/// The routing property, now observed end-to-end rather than on paths alone.
#[test]
fn delivery_is_hop_optimal() {
    let g = deep_gateway();
    let cells = g.router().cells();
    let mut checked = 0;
    for i in (0..cells.len()).step_by(31) {
        for j in (0..cells.len()).step_by(37) {
            if cells[i] == cells[j] {
                continue;
            }
            let packet = Packet::new(Frame::encode(b"hop"), cells[i], cells[j]);
            let optimal = g.router().bfs_hops(cells[i], cells[j]).expect("connected");
            let delivery = g.deliver(&packet, 200).unwrap();
            // `.hops()` alone is not enough: it is defined on every `Delivery`
            // variant, including the two failure ones, so a coincidental
            // dissipation at the optimal hop count would pass this assertion
            // without the packet ever having arrived. Arrival is checked
            // explicitly so the test cannot pass vacuously.
            assert!(
                delivery.arrived(),
                "clean delivery from {:?} to {:?} did not arrive: {delivery:?}",
                cells[i],
                cells[j]
            );
            assert_eq!(delivery.hops(), optimal);
            checked += 1;
        }
    }
    assert!(checked > 50, "expected a meaningful sample, checked {checked}");
}

/// 1.3b [D] — **the farthest pair in the patch delivers, byte-exact, at the
/// true diameter — not just the 3-4 hop routes every other test in this file
/// uses.**
///
/// `ftg.rs::diameter_of_the_documented_patch_is_measured` establishes the
/// diameter of this same 441-cell patch is 10 hops; `delivery_is_hop_optimal`
/// above samples across the whole patch but only checks hop *count*, not that
/// the payload actually survived. This is the one test that does both at
/// once, on a route deliberately chosen to be as long as this patch gets: a
/// frame that round-trips through demodulate-and-re-encode ten times, with a
/// clean medium, must still be byte-identical to what was sent.
#[test]
fn farthest_pair_delivers_byte_exact_at_the_true_diameter() {
    let g = deep_gateway();
    let cells = g.router().cells();
    let (mut src, mut dst, mut best) = (cells[0], cells[0], 0);
    for (i, &a) in cells.iter().enumerate().step_by(7) {
        for &b in cells.iter().skip(i + 1).step_by(11) {
            if let Some(h) = g.router().bfs_hops(a, b) {
                if h > best {
                    (src, dst, best) = (a, b, h);
                }
            }
        }
    }
    assert!(best >= 8, "expected to find a near-diameter pair, got {best} hops");

    let payload = b"the farthest cell in the patch still hears this exactly".to_vec();
    let packet = Packet::new(Frame::encode(&payload), src, dst);
    match g.deliver(&packet, 200).unwrap() {
        Delivery::Arrived {
            payload: got,
            hops,
            ..
        } => {
            assert_eq!(hops, best, "delivered hop count must match the measured diameter");
            assert_eq!(got, payload, "payload must survive {hops} real transductions exactly");
        }
        other => panic!("expected arrival at the patch diameter, got {other:?}"),
    }
}

/// 1.4 — delivering to oneself takes no hops.
#[test]
fn delivery_to_self_takes_no_hops() {
    let g = gateway();
    let c = g.router().cells()[5];
    let packet = Packet::new(Frame::encode(b"self"), c, c);
    match g.deliver(&packet, 200).unwrap() {
        Delivery::Arrived { payload, hops, path } => {
            assert_eq!(hops, 0);
            assert_eq!(path, vec![c]);
            assert_eq!(payload, b"self");
        }
        other => panic!("expected arrival, got {other:?}"),
    }
}

/// 1.5 — addressing by network address end-to-end.
#[test]
fn packets_can_be_addressed_by_network_address() {
    let g = gateway();
    let packet = g.packet_for(
        b"addressed",
        NetAddress::v4(10, 0, 0, 1),
        NetAddress::v4(192, 168, 1, 44),
    );
    let delivery = g.deliver(&packet, 200).unwrap();
    assert_eq!(delivery.payload(), Some(&b"addressed"[..]));
}

// --------------------------------------------- Group 2: corruption in flight

/// 2.1 [D] — **a frame corrupted in flight dissipates at that hop.**
///
/// Not carried to the destination and rejected there. The PRD says corrupted
/// frames "collapse into dissonance and are naturally dissipated", and this is
/// that behaviour observed end-to-end: the fault has a *location*.
#[test]
fn corruption_dissipates_at_the_hop_where_it_occurs() {
    let g = gateway();
    let packet = multi_hop_packet(&g, b"corrupt in flight", 4);
    let path = g
        .router()
        .route(packet.source(), packet.destination(), 200)
        .unwrap();

    for fault_hop in 0..3 {
        let delivery = g
            .deliver_through(&packet, 200, |hop, frame| {
                if hop == fault_hop {
                    frame.corrupt(2);
                }
            })
            .unwrap();

        match delivery {
            Delivery::Dissipated { at, hop, amplitude } => {
                assert_eq!(
                    hop,
                    fault_hop + 1,
                    "a fault injected before hop {fault_hop} must dissipate on arrival at that hop"
                );
                assert_eq!(at, path[fault_hop + 1], "dissipation reported at the wrong cell");
                assert!(
                    amplitude > DISSONANCE_FLOOR,
                    "dissipation must report real dissonance, got {amplitude}"
                );
            }
            other => panic!("corrupted frame must dissipate, got {other:?}"),
        }
    }
}

/// 2.2 — a dissipated packet yields no payload. There is no partial delivery.
#[test]
fn dissipated_packets_carry_nothing() {
    let g = gateway();
    let packet = multi_hop_packet(&g, b"lost", 3);
    let delivery = g
        .deliver_through(&packet, 200, |hop, frame| {
            if hop == 0 {
                frame.corrupt(1);
            }
        })
        .unwrap();
    assert!(!delivery.arrived());
    assert_eq!(delivery.payload(), None);
}

/// 2.3 — a benign medium changes nothing.
///
/// Confirms the fault injector is the *only* source of corruption, so 2.1 is
/// measuring the medium rather than damage done by the gateway itself.
#[test]
fn a_quiet_medium_delivers_intact() {
    let g = gateway();
    let packet = multi_hop_packet(&g, b"untouched", 4);
    let delivery = g.deliver_through(&packet, 200, |_, _| {}).unwrap();
    assert_eq!(delivery.payload(), Some(&b"untouched"[..]));
}

/// 2.4 — the transport blind spot mirrors the frame's own.
///
/// Flipping a symbol *and its complement partner* cancels, so the frame reports
/// clean and **arrives with a corrupted payload**. This is the documented limit
/// of interference checking, now visible end-to-end rather than only in a unit
/// test. Asserted so it cannot be quietly claimed away.
#[test]
fn correlated_corruption_survives_the_route_undetected() {
    let g = gateway();
    let payload = b"blindspot";
    let packet = multi_hop_packet(&g, payload, 3);
    let half = packet.frame().payload_bits();

    let delivery = g
        .deliver_through(&packet, 200, |hop, frame| {
            if hop == 0 {
                frame.corrupt(0);
                frame.corrupt(half);
            }
        })
        .unwrap();

    match delivery {
        Delivery::Arrived { payload: got, .. } => {
            assert_ne!(
                got,
                payload.to_vec(),
                "the payload is genuinely corrupted - that is the point"
            );
        }
        other => panic!("correlated flips cancel, so the frame must arrive: {other:?}"),
    }
}

// ------------------------------------------------ Group 3: routing failures

/// 3.1 — a stranded packet reports no descent rather than looping.
#[test]
fn stranded_packet_reports_no_descent() {
    let small = Gateway::new(1);
    let large = Gateway::new(4);
    let outside = large
        .router()
        .cells()
        .iter()
        .find(|c| !small.router().contains(c))
        .copied()
        .expect("a deeper patch has cells the shallow one lacks");

    let packet = Packet::new(Frame::encode(b"stranded"), small.router().cells()[0], outside);
    assert!(matches!(
        small.deliver(&packet, 50),
        Err(FtgError::NoDescent { .. })
    ));
}

/// 3.2 — the hop limit bounds delivery, not just route computation.
#[test]
fn hop_limit_bounds_delivery() {
    let g = gateway();
    let packet = multi_hop_packet(&g, b"limited", 3);
    assert!(matches!(
        g.deliver(&packet, 1),
        Err(FtgError::HopLimit { limit: 1 })
    ));
}

// ------------------------------------------ Group 4: the transduction itself

/// 4.1 [D] — **transduction is lossless across many hops.**
///
/// Each hop synthesises onto the carrier and demodulates back. If that drifted
/// even slightly, a long route would corrupt a clean payload. Over the longest
/// route in the patch it does not.
#[test]
fn transduction_is_lossless_over_the_longest_route() {
    let g = deep_gateway();
    let cells = g.router().cells();
    let (mut best, mut best_hops) = (None, 0);
    for &dst in cells {
        if let Some(h) = g.router().bfs_hops(cells[0], dst) {
            if h > best_hops {
                best_hops = h;
                best = Some(dst);
            }
        }
    }
    let dst = best.expect("some route exists");
    assert!(best_hops >= 5, "expected a long route, longest was {best_hops}");

    let payload: Vec<u8> = (0..=255u8).collect();
    let packet = Packet::new(Frame::encode(&payload), cells[0], dst);
    match g.deliver(&packet, 200).unwrap() {
        Delivery::Arrived { payload: got, hops, .. } => {
            assert_eq!(hops, best_hops);
            assert_eq!(got, payload, "payload drifted over {hops} transductions");
        }
        other => panic!("expected arrival, got {other:?}"),
    }
}

/// 4.2 [D] — every hop samples away from a carrier zero crossing.
///
/// At a zero crossing both bit states read as exactly zero. If transport ever
/// sampled there, delivery would fail with nothing recoverable — so this
/// asserts the instants transport uses are all safe.
#[test]
fn every_hop_samples_at_a_safe_instant() {
    for hop in 0..32u32 {
        let t = substrate::translation::safe_sample_instant(hop);
        assert!(
            !substrate::translation::is_zero_crossing(t),
            "hop {hop} would sample at a zero crossing"
        );
    }
}
