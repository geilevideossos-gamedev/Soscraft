pub mod buckets;
mod squids;
#[cfg(test)]
mod tests;

terrain_group!(water {
    squids::Squid,
});
