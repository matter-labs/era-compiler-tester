//!
//! The address iterator trait.
//!

///
/// The address iterator trait.
///
pub trait AddressIterator {
    ///
    /// Returns the next address.
    ///
    fn next(
        &mut self,
        caller: &web3::types::Address,
        increment_nonce: bool,
    ) -> web3::types::Address;

    ///
    /// Increments the nonce for the caller.
    ///
    fn increment_nonce(&mut self, caller: &web3::types::Address);

    ///
    /// Returns the nonce for the caller.
    ///
    /// If the nonce for the `caller` does not exist, it will be created.
    ///
    fn nonce(&mut self, caller: &web3::types::Address) -> usize;
}
