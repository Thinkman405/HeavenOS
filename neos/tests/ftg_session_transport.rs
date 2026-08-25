//! Session-gated delivery — §6 transport joined to §7 connection lifecycle.
//!
//! A connection in NEOS is a shared standing wave, not a state record. These
//! assertions hold transport to that: a packet cannot travel without a resonant
//! link, and a session that collapses mid-transfer strands the packet where the
//! carrier stopped.
//!
//! **[D]** marks assertions a conventional stack could not pass — one where a
//! "connection" is a struct that survives whatever the physics does.

use ftg::session::{Link, LinkState, Oscillator};
use ftg::transport::{Delivery, Gateway, Packet};
use ftg::{constants::*, Frame, FtgError};

fn gateway() -> Gateway {
    Gateway::new(4)
}

/// The documented 441-cell patch, whose true diameter is 10 hops — see
/// `ftg.rs::diameter_of_the_documented_patch_is_measured`. Every other test
/// in this file uses `gateway()`'s smaller, 166-cell patch at 3-4 hops.
fn deep_gateway() -> Gateway {
    Gateway::new(5)
}

fn resonant_link() -> Link {
    Link::attempt_handshake(Oscillator::new(0.0, 1.0), Oscillator::new(0.1, 1.0))
        .expect("variance 0.1 is well inside the lock bound")
}

/// A route of at least `min_hops`, so mid-transfer behaviour is exercised.
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
    panic!("no route of at least {min_hops} hops");
}

// ---------------------------------------------------- Group 1: admission

/// 1.1 — a resonant session carries the payload intact.
#[test]
fn resonant_session_delivers() {
    let g = gateway();
    let link = resonant_link();
    let packet = multi_hop_packet(&g, b"over a session", 3);

    match g.deliver_over(&link, &packet, 200).unwrap() {
        Delivery::Arrived { payload, hops, .. } => {
            assert!(hops >= 3);
            assert_eq!(payload, b"over a session");
        }
        other => panic!("expected arrival, got {other:?}"),
    }
}

/// 1.1b [D] — **a stable, undisturbed session survives the patch's true
/// diameter**, not just the 3-4 hop routes every other test here uses.
///
/// `still_locked` is re-checked every hop (see `session_is_rechecked_...`
/// below), but nothing about a resonant link's phase variance changes on its
/// own — it only moves when something calls `drift_to`. So a session that
/// starts locked and is never perturbed should carry a packet exactly as far
/// as a bare `deliver_through` would, including at ten real hops. This is
/// the session-gated counterpart to
/// `ftg_transport.rs::farthest_pair_delivers_byte_exact_at_the_true_diameter`.
#[test]
fn undisturbed_session_survives_the_true_diameter() {
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

    let link = resonant_link();
    let payload = b"still locked after ten real hops".to_vec();
    let packet = Packet::new(Frame::encode(&payload), src, dst);

    match g.deliver_over(&link, &packet, 200).unwrap() {
        Delivery::Arrived {
            payload: got,
            hops,
            ..
        } => {
            assert_eq!(hops, best, "delivered hop count must match the measured diameter");
            assert_eq!(got, payload, "payload must survive an undisturbed session at {hops} hops");
        }
        other => panic!("expected arrival at the patch diameter, got {other:?}"),
    }
}

/// 1.2 [D] — **a collapsed session cannot carry anything.**
///
/// Teardown drove amplitude to zero. There is no medium, so there is no
/// delivery — not a degraded one, none.
#[test]
fn collapsed_session_refuses_delivery() {
    let g = gateway();
    let mut link = resonant_link();
    link.teardown(CARRIER_RAD_PER_SEC, 1e-10).unwrap();
    assert_eq!(link.state(), LinkState::Collapsed);

    let packet = multi_hop_packet(&g, b"no carrier", 3);
    assert!(matches!(
        g.deliver_over(&link, &packet, 200),
        Err(FtgError::Collapsed)
    ));
}

/// 1.3 [D] — a session that has drifted out of resonance refuses delivery.
///
/// Built by hand at variance ≥ π/4: `attempt_handshake` would not have created
/// it, which is itself the point — the only way in is through the gate.
#[test]
fn out_of_resonance_session_refuses_delivery() {
    let g = gateway();
    let packet = multi_hop_packet(&g, b"drifted", 3);

    // The handshake refuses to build one at all.
    assert!(matches!(
        Link::attempt_handshake(
            Oscillator::new(0.0, 1.0),
            Oscillator::new(LINK_LOCK_BOUND, 1.0)
        ),
        Err(FtgError::NoLock { .. })
    ));

    // And a link that drifts after locking is refused at admission.
    let mut link = resonant_link();
    link.drift_to(LINK_LOCK_BOUND + 0.1);
    assert!(!link.still_locked());
    assert!(matches!(
        g.deliver_over(&link, &packet, 200),
        Err(FtgError::NoLock { .. })
    ));
}

/// 1.4 — admission does not consume or alter the caller's link.
#[test]
fn delivery_does_not_mutate_the_callers_link() {
    let g = gateway();
    let link = resonant_link();
    let before = link.state();
    let packet = multi_hop_packet(&g, b"immutable", 3);
    g.deliver_over(&link, &packet, 200).unwrap();
    assert_eq!(link.state(), before);
    assert!(link.still_locked());
}

// ------------------------------------------- Group 2: collapse mid-transfer

/// 2.1 [D] — **a session torn down mid-transfer strands the packet there.**
///
/// The frame is intact; the carrier is gone. This is the assertion that makes
/// "a connection is a standing wave, not a record" operational rather than
/// rhetorical — a conventional stack would deliver, because its connection
/// object still exists.
#[test]
fn teardown_mid_transfer_strands_the_packet() {
    let g = gateway();
    let packet = multi_hop_packet(&g, b"in flight when it died", 4);
    let path = g
        .router()
        .route(packet.source(), packet.destination(), 200)
        .unwrap();

    for collapse_hop in 0..3 {
        let mut link = resonant_link();
        let delivery = g
            .deliver_over_with(&mut link, &packet, 200, |hop, _frame, l| {
                if hop == collapse_hop {
                    l.teardown(CARRIER_RAD_PER_SEC, 1e-10).ok();
                }
            })
            .unwrap();

        match delivery {
            Delivery::LinkLost {
                at,
                hop,
                carrier_amplitude,
            } => {
                assert_eq!(hop, collapse_hop + 1, "stranded at the wrong hop");
                assert_eq!(at, path[collapse_hop + 1], "stranded at the wrong cell");
                assert!(
                    carrier_amplitude < 1e-9,
                    "the carrier must actually be gone, measured {carrier_amplitude}"
                );
            }
            other => panic!("expected LinkLost, got {other:?}"),
        }
    }
}

/// 2.2 [D] — the link is re-checked **every** hop, not only at admission.
///
/// Collapsing at the last possible hop must still strand the packet. If the
/// gate were admission-only, this would arrive.
#[test]
fn link_is_rechecked_at_every_hop() {
    let g = gateway();
    let packet = multi_hop_packet(&g, b"late collapse", 4);
    let hops = g
        .router()
        .route(packet.source(), packet.destination(), 200)
        .unwrap()
        .len()
        - 1;

    let mut link = resonant_link();
    let last = hops - 1;
    let delivery = g
        .deliver_over_with(&mut link, &packet, 200, |hop, _f, l| {
            if hop == last {
                l.teardown(CARRIER_RAD_PER_SEC, 1e-10).ok();
            }
        })
        .unwrap();

    assert!(
        matches!(delivery, Delivery::LinkLost { hop, .. } if hop == hops),
        "collapse on the final hop must still strand, got {delivery:?}"
    );
}

/// 2.3 — a stranded packet carries nothing. No partial delivery.
#[test]
fn stranded_packet_carries_nothing() {
    let g = gateway();
    let packet = multi_hop_packet(&g, b"lost", 3);
    let mut link = resonant_link();
    let delivery = g
        .deliver_over_with(&mut link, &packet, 200, |hop, _f, l| {
            if hop == 0 {
                l.teardown(CARRIER_RAD_PER_SEC, 1e-10).ok();
            }
        })
        .unwrap();
    assert!(!delivery.arrived());
    assert_eq!(delivery.payload(), None);
}

/// 2.4 — a quiet session delivers, so 2.1 measures the teardown and not the
/// harness.
#[test]
fn untouched_session_delivers_intact() {
    let g = gateway();
    let packet = multi_hop_packet(&g, b"undisturbed", 4);
    let mut link = resonant_link();
    let delivery = g
        .deliver_over_with(&mut link, &packet, 200, |_, _, _| {})
        .unwrap();
    assert_eq!(delivery.payload(), Some(&b"undisturbed"[..]));
    assert!(link.still_locked());
}

/// 2.5 [D] — **drift discovered mid-transfer now actually collapses the
/// link**, not merely reports it lost while it stays nominally `Resonant`.
///
/// Every test above this one that strands a packet does so by calling
/// `l.teardown(...)` directly in the medium closure — the link is already
/// `Collapsed` by the time `deliver_over_with` notices. This test instead
/// drifts the link (`drift_to`, which — by design — does not touch `state`
/// on its own) and checks the *caller's* `link` afterward: before
/// `Link::enforce_lock` existed, `link.state()` here would still read
/// `Resonant { .. }` even after a `LinkLost` delivery, because nothing had
/// ever called `teardown` on it.
#[test]
fn drift_discovered_mid_transfer_actually_collapses_the_link() {
    let g = gateway();
    let packet = multi_hop_packet(&g, b"drifted in flight", 4);
    let mut link = resonant_link();

    let delivery = g
        .deliver_over_with(&mut link, &packet, 200, |hop, _frame, l| {
            if hop == 1 {
                l.drift_to(LINK_LOCK_BOUND + 0.1);
            }
        })
        .unwrap();

    assert!(
        matches!(delivery, Delivery::LinkLost { hop: 2, .. }),
        "expected stranding at hop 2, got {delivery:?}"
    );
    assert_eq!(
        link.state(),
        LinkState::Collapsed,
        "a drifted link discovered mid-transfer must actually collapse, \
         not remain Resonant while merely being reported as lost"
    );
}

/// 2.6 [D] — the same policy applies **at admission**, before any hop runs.
///
/// `out_of_resonance_session_refuses_delivery` already covers the refusal
/// itself, through `deliver_over` (which clones, so it cannot observe the
/// caller's own link afterward). This uses `deliver_over_with` directly on a
/// merely-drifted (never manually torn down) link, so the automatic-teardown
/// side effect is actually visible on the value the caller holds.
#[test]
fn drift_discovered_at_admission_also_collapses_the_link() {
    let g = gateway();
    let packet = multi_hop_packet(&g, b"already drifted", 3);
    let mut link = resonant_link();
    link.drift_to(LINK_LOCK_BOUND + 0.2);
    assert_eq!(
        link.state(),
        LinkState::Resonant { sync_phase: 0.05 },
        "premise: drift_to alone must not touch state"
    );

    let result = g.deliver_over_with(&mut link, &packet, 200, |_, _, _| {});
    assert!(matches!(result, Err(FtgError::NoLock { .. })));
    assert_eq!(
        link.state(),
        LinkState::Collapsed,
        "admission-time drift must be enforced immediately, same as mid-transfer"
    );
}

// -------------------------------------- Group 3: the two failures are distinct

/// 3.1 [D] — frame corruption and session loss are different outcomes.
///
/// Both stop the packet, but for different reasons and with different remedies.
/// Collapsing them into one "failed" would lose that.
#[test]
fn dissipation_and_link_loss_are_distinguishable() {
    let g = gateway();
    let packet = multi_hop_packet(&g, b"two failure modes", 4);

    let mut l1 = resonant_link();
    let corrupted = g
        .deliver_over_with(&mut l1, &packet, 200, |hop, frame, _| {
            if hop == 1 {
                frame.corrupt(3);
            }
        })
        .unwrap();

    let mut l2 = resonant_link();
    let lost = g
        .deliver_over_with(&mut l2, &packet, 200, |hop, _f, l| {
            if hop == 1 {
                l.teardown(CARRIER_RAD_PER_SEC, 1e-10).ok();
            }
        })
        .unwrap();

    assert!(
        matches!(corrupted, Delivery::Dissipated { .. }),
        "a corrupted frame dissipates, got {corrupted:?}"
    );
    assert!(
        matches!(lost, Delivery::LinkLost { .. }),
        "a collapsed session strands, got {lost:?}"
    );
    assert_ne!(
        std::mem::discriminant(&corrupted),
        std::mem::discriminant(&lost)
    );
}

/// 3.2 — an intact frame survives session loss; only the medium failed.
///
/// Confirms `LinkLost` really is about the carrier. The frame was never
/// touched, so re-sending it over a fresh session must work.
#[test]
fn a_stranded_frame_is_still_intact() {
    let g = gateway();
    let payload = b"intact but stranded";
    let packet = multi_hop_packet(&g, payload, 4);

    let mut dying = resonant_link();
    let lost = g
        .deliver_over_with(&mut dying, &packet, 200, |hop, _f, l| {
            if hop == 1 {
                l.teardown(CARRIER_RAD_PER_SEC, 1e-10).ok();
            }
        })
        .unwrap();
    assert!(matches!(lost, Delivery::LinkLost { .. }));

    // Same packet, fresh session: it arrives untouched.
    let fresh = resonant_link();
    assert_eq!(
        g.deliver_over(&fresh, &packet, 200).unwrap().payload(),
        Some(&payload[..])
    );
}

// --------------------------------------------- Group 4: ungated path unchanged

/// 4.1 — `deliver` still works without a session.
///
/// Session gating is an added guarantee on `deliver_over`, not a change to the
/// existing path. Slice 2's behaviour must be untouched.
#[test]
fn ungated_delivery_is_unaffected() {
    let g = gateway();
    let packet = multi_hop_packet(&g, b"no session needed", 3);
    assert_eq!(
        g.deliver(&packet, 200).unwrap().payload(),
        Some(&b"no session needed"[..])
    );
}
