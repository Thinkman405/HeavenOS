//! Hypervisor entry point and virtual machine bootstrap.
//!
//! A thin binary over the library. Everything testable lives in `lib.rs`.

use substrate::translation;
use substrate::Hypervisor;

fn main() {
    let mut hv = Hypervisor::boot(31, 4096);

    println!("NEOS substrate");
    println!(
        "  carrier        {:.6e} rad/s (angular)",
        hv.carrier().get()
    );
    println!("  quarter period {:.6e} s", hv.carrier().quarter_period());
    println!(
        "  memory         {} cells x {} bytes = {} total",
        hv.pool().cell_count(),
        hv.pool().cell_capacity(),
        hv.pool().total_capacity()
    );

    let message = b"NEOS";
    let phases = translation::bits_to_phases(message);
    let t = translation::safe_sample_instant(0);
    match translation::demodulate(&phases, t) {
        Ok(bits) => println!(
            "  translation    {:?} -> {} phases -> {:?} (round trip {})",
            String::from_utf8_lossy(message),
            phases.len(),
            String::from_utf8_lossy(&bits),
            if bits == message { "ok" } else { "FAILED" }
        ),
        Err(e) => println!("  translation    error: {e}"),
    }

    let alloc = hv
        .pool_mut()
        .allocate(8192)
        .expect("8 KiB fits in 31 x 4096");
    println!(
        "  allocation     {} bytes across {} adjacent cells",
        alloc.len(),
        alloc.cells().len()
    );

    println!("  extent         {}", hv.pool().extent());
    let split = hv.pool_mut().split().expect("unit split is in domain");
    println!("  after split    {split}  (axiom A1: 1 (x) 1 = 2)");

    for _ in 0..4 {
        hv.tick();
    }
    println!("  uptime         {:.6e} s after {} ticks", hv.uptime_seconds(), hv.ticks());
}
