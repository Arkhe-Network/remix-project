#[cfg(test)]
#[path = "../verification/kani/substrate.rs"]
mod kani_substrate;

#[cfg(test)]
#[path = "../verification/kani/projection.rs"]
mod kani_projection;

#[cfg(test)]
#[path = "../verification/proptest/substrate_prop.rs"]
mod proptest_substrate;

#[cfg(test)]
#[path = "../verification/proptest/projection_prop.rs"]
mod proptest_projection;
