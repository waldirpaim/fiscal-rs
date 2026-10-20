#![doc = "Node.js native binding for fiscal-rs via napi-rs."]

mod builder;
mod certificate;
mod client;
mod contingency;
mod utils;

// O napi desativa o registro dos exports em cfg(test). Mantém os pontos de entrada
// alcançáveis nesse build, sem mudar a API Rust de produção.
#[cfg(test)]
pub use {builder::*, certificate::*, client::*, contingency::*, utils::*};
