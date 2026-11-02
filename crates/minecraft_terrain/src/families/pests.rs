mod endermite_render;
pub(crate) mod endermites;
mod silverfish;
mod silverfish_render;

terrain_group!(pests {
    silverfish::Silverfish,
    endermites::Endermite,
});
