//! Lógica compartida entre la app de escritorio y la web.
//!
//! Cada fuente de datos se traduce a [`game::Game`] y desde ahí salen los mismos
//! resúmenes, marcadores y líneas de tiempo.

pub mod game;
pub mod lcu;
pub mod lp;
pub mod match_v5;
