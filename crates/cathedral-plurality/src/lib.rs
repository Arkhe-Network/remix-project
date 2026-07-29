#![allow(warnings)]
pub mod memory_adapter;
pub mod smart_profile;
pub mod plurality_client;
pub mod plurality_auth;
pub mod plurality_types;
pub mod jwks;

pub trait PluralityClientTrait {
    fn new() -> Self;
}
