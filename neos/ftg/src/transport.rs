//! End-to-end transport: carrying a [`Frame`] along a computed route.
//!
//! This is where Layer 1/2 and Layer 3/4 meet. Routing computes a path;
//! framing survives — or fails to survive — each hop along it.
//!
//! ## A hop is a transmission, not a pointer move
//!
//! At every hop the frame is synthesised onto the carrier and demodulated at
//! the far end, using `substrate`'s transduction. That is what makes this
//! end-to-end rather than a list traversal: a frame that cannot be recovered
//! from the carrier does not arrive.
//!
//! ## Two ways a packet fails to arrive, and they are not the same
//!
//! - [`Delivery::Dissipated`] — the **frame** collapsed into dissonance.
//! - [`Delivery::LinkLost`] — the **session** collapsed; the frame is intact
//!   but the standing wave carrying it is gone.
//!
//! Keeping them apart matters because the remedies differ: a dissipated frame
//! was corrupted in transit, while a lost link means the connection ended
//! underneath a healthy packet.
//!
//! ## Corruption dissipates where it happens
//!
//! The contract forbids repair. A frame that picks up dissonance is dropped at
//! **the hop where it was detected**, not carried to the destination and
//! rejected there. `Delivery::Dissipated` reports which hop and how much net
//! amplitude, so a fault has a location rather than just an outcome.

use crate::constants::CARRIER_RAD_PER_SEC;
use crate::layers_1_2::Frame;
use crate::layers_3_4::{NetAddress, Router};
use crate::session::{Link, LinkState};
use crate::FtgError;
use lattice::tessellation::CellId;
use substrate::translation;

/// A frame with somewhere to go.
#[derive(Debug, Clone, PartialEq)]
pub struct Packet {
    frame: Frame,
    src: CellId,
    dst: CellId,
}

impl Packet {
    pub const fn new(frame: Frame, src: CellId, dst: CellId) -> Self {
        Self { frame, src, dst }
    }

    pub fn frame(&self) -> &Frame {
        &self.frame
    }

    pub const fn source(&self) -> CellId {
        self.src
    }

    pub const fn destination(&self) -> CellId {
        self.dst
    }
}

/// What became of a packet.
#[derive(Debug, Clone, PartialEq)]
pub enum Delivery {
    /// Reached the destination with its payload intact.
    Arrived {
        payload: Vec<u8>,
        path: Vec<CellId>,
        hops: usize,
    },
    /// Collapsed into dissonance in transit. Not an error to be handled but the
    /// designed outcome for a corrupted frame: it dissipates.
    Dissipated {
        at: CellId,
        hop: usize,
        amplitude: f64,
    },
    /// The session collapsed while the packet was in flight.
    ///
    /// Distinct from [`Delivery::Dissipated`]: the frame is **intact**, but the
    /// standing wave carrying it is gone. A connection in NEOS is not a state
    /// record that can outlive its physics - when amplitude reaches zero there
    /// is no medium, so the frame simply stops.
    ///
    /// `carrier_amplitude` is the measured combined amplitude at the moment of
    /// loss, so a caller can see the medium really did vanish rather than
    /// trusting the label.
    LinkLost {
        at: CellId,
        hop: usize,
        carrier_amplitude: f64,
    },
}

impl Delivery {
    pub fn arrived(&self) -> bool {
        matches!(self, Self::Arrived { .. })
    }

    pub fn payload(&self) -> Option<&[u8]> {
        match self {
            Self::Arrived { payload, .. } => Some(payload),
            Self::Dissipated { .. } | Self::LinkLost { .. } => None,
        }
    }

    pub fn hops(&self) -> usize {
        match self {
            Self::Arrived { hops, .. } => *hops,
            Self::Dissipated { hop, .. } | Self::LinkLost { hop, .. } => *hop,
        }
    }
}

/// Joins routing and framing into a delivery path.
#[derive(Debug, Clone)]
pub struct Gateway {
    router: Router,
}

impl Gateway {
    pub fn new(depth: usize) -> Self {
        Self {
            router: Router::new(depth),
        }
    }

    pub fn router(&self) -> &Router {
        &self.router
    }

    /// Build a packet addressed by network address rather than by cell.
    pub fn packet_for(&self, payload: &[u8], src: NetAddress, dst: NetAddress) -> Packet {
        Packet::new(
            Frame::encode(payload),
            self.router.cell_for(src),
            self.router.cell_for(dst),
        )
    }

    /// Carry a packet to its destination.
    ///
    /// # Errors
    /// Propagates [`FtgError::NoDescent`] or [`FtgError::HopLimit`] from
    /// routing. A *corrupted* frame is not an error — it returns
    /// [`Delivery::Dissipated`], because dissipation is the designed behaviour
    /// rather than a failure of the gateway.
    pub fn deliver(&self, packet: &Packet, max_hops: usize) -> Result<Delivery, FtgError> {
        self.deliver_through(packet, max_hops, |_, _| {})
    }

    /// Carry a packet, letting a `medium` perturb the frame at each hop.
    ///
    /// The medium is how corruption enters. Production passes a no-op; tests
    /// pass a fault injector, which is the only way to exercise dissipation
    /// without pretending the gateway itself damages frames.
    pub fn deliver_through<F>(
        &self,
        packet: &Packet,
        max_hops: usize,
        mut medium: F,
    ) -> Result<Delivery, FtgError>
    where
        F: FnMut(usize, &mut Frame),
    {
        let path = self.router.route(packet.src, packet.dst, max_hops)?;
        let mut frame = packet.frame.clone();

        for (hop, window) in path.windows(2).enumerate() {
            let arriving_at = window[1];

            // The medium acts on the wave in flight.
            medium(hop, &mut frame);

            // Transduce across the hop. Sampling at a safe instant is not
            // optional: at a carrier zero crossing both bit states read as
            // zero and nothing is recoverable.
            let t = translation::safe_sample_instant(hop as u32);
            let recovered = translation::demodulate(frame.phases(), t)
                .map_err(|_| FtgError::Dissonant { amplitude: f64::NAN })?;
            frame = Frame::from_phases(translation::bits_to_phases(&recovered));

            let dissonance = frame.dissonance();
            if dissonance > crate::layers_1_2::DISSONANCE_FLOOR {
                return Ok(Delivery::Dissipated {
                    at: arriving_at,
                    hop: hop + 1,
                    amplitude: dissonance,
                });
            }
        }

        let payload = frame.decode()?;
        Ok(Delivery::Arrived {
            hops: path.len() - 1,
            path,
            payload,
        })
    }

    /// Carry a packet **over an established session**.
    ///
    /// Unlike [`Gateway::deliver`], this requires a resonant [`Link`]. §7 says a
    /// connection is a shared standing wave rather than a state record, so a
    /// packet cannot travel without one: there is no medium.
    ///
    /// # Errors
    /// - [`FtgError::Collapsed`] if the link has already been torn down.
    /// - [`FtgError::NoLock`] if the link is not resonant, or has drifted to
    ///   phase variance at or above the lock bound.
    /// - Routing errors as for [`Gateway::deliver`].
    pub fn deliver_over(
        &self,
        link: &Link,
        packet: &Packet,
        max_hops: usize,
    ) -> Result<Delivery, FtgError> {
        self.deliver_over_with(&mut link.clone(), packet, max_hops, |_, _, _| {})
    }

    /// Session-gated delivery, letting a `medium` act on both the frame **and
    /// the link** at each hop.
    ///
    /// The link is re-checked before every hop, not merely at admission. That
    /// is the point: a session can collapse mid-transfer, and a packet already
    /// in flight must stop where the carrier stopped rather than completing on
    /// a connection that no longer exists.
    pub fn deliver_over_with<F>(
        &self,
        link: &mut Link,
        packet: &Packet,
        max_hops: usize,
        mut medium: F,
    ) -> Result<Delivery, FtgError>
    where
        F: FnMut(usize, &mut Frame, &mut Link),
    {
        // Admission: the session must exist before anything is sent.
        if link.state() == LinkState::Collapsed {
            return Err(FtgError::Collapsed);
        }
        // Measure before enforcing: `enforce_lock` tears a drifted link down
        // as a side effect, and teardown's forced pi-shift would make a
        // variance read afterward describe the shift, not the drift that
        // actually caused the loss.
        let variance = link.phase_variance();
        if !link.enforce_lock(CARRIER_RAD_PER_SEC, translation::safe_sample_instant(0)) {
            // Automatic teardown-on-drift (`equations.md`'s Standing Wave
            // Superposition rule): a link found already drifted past the
            // lock bound is torn down here, not merely refused while
            // remaining nominally Resonant. `enforce_lock` just did that -
            // this is not a second failure to react to, it is what already
            // happened.
            return Err(FtgError::NoLock { variance });
        }

        let path = self.router.route(packet.src, packet.dst, max_hops)?;
        let mut frame = packet.frame.clone();

        for (hop, window) in path.windows(2).enumerate() {
            let arriving_at = window[1];
            let t = translation::safe_sample_instant(hop as u32);

            // The medium acts on the wave, and may collapse the session.
            medium(hop, &mut frame, link);

            // Re-check per hop. A session lost mid-transfer strands the packet
            // here; the frame is intact but there is nothing to carry it.
            //
            // Measure before enforcing, for the same reason as admission:
            // `enforce_lock` may tear the link down as a side effect, and the
            // reported amplitude must describe the medium at the moment of
            // loss, not whatever teardown's forced shift leaves behind.
            let carrier_amplitude = link.superposition(CARRIER_RAD_PER_SEC, t).abs();
            if !link.enforce_lock(CARRIER_RAD_PER_SEC, t) {
                // A link that merely reported LinkLost while remaining
                // Resonant would be exactly the "state record that outlives
                // its physics" the module docs rule out. `enforce_lock`
                // makes this real: `link.state()` is now genuinely
                // Collapsed, unless it already was.
                return Ok(Delivery::LinkLost {
                    at: arriving_at,
                    hop: hop + 1,
                    carrier_amplitude,
                });
            }

            let recovered = translation::demodulate(frame.phases(), t)
                .map_err(|_| FtgError::Dissonant { amplitude: f64::NAN })?;
            frame = Frame::from_phases(translation::bits_to_phases(&recovered));

            let dissonance = frame.dissonance();
            if dissonance > crate::layers_1_2::DISSONANCE_FLOOR {
                return Ok(Delivery::Dissipated {
                    at: arriving_at,
                    hop: hop + 1,
                    amplitude: dissonance,
                });
            }
        }

        let payload = frame.decode()?;
        Ok(Delivery::Arrived {
            hops: path.len() - 1,
            path,
            payload,
        })
    }
}
