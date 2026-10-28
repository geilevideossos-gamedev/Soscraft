#[cfg(test)]
pub mod probe;

family_group!(pests 36..=39 {
    #[cfg(test)]
    probe: probe::Probe,
});
