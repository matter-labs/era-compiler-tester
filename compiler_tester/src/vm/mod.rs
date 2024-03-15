//!
//! The VM wrappers.
//!

pub mod eravm;
pub mod evm;
pub mod execution_result;

///
/// The address predictor iterator.
///
pub trait AddressPredictorIterator {
    ///
    /// Returns the next predicted address.
    ///
    fn next(
        &mut self,
        caller: &web3::types::Address,
        increment_nonce: bool,
    ) -> web3::types::Address;
}
