pub mod endermites;
#[cfg(test)]
pub mod probe;
pub mod silverfish;

family_group!(pests 36..=39 {
    silverfish: silverfish::Silverfish,
    endermites: endermites::Endermite,
    #[cfg(test)]
    probe: probe::Probe,
});
