//!
//! The EVM invoker.
//!

use crate::vm::evm::runtime::Runtime as EVMRuntime;

///
/// The EVM resolver type.
///
pub type Resolver<'evm> = evm::standard::EtableResolver<
    'evm,
    'evm,
    'evm,
    (),
    evm::Etable<evm::standard::State<'evm>, EVMRuntime, evm::trap::CallCreateTrap>,
>;

///
/// The EVM wrapped invoker type.
///
pub type Invoker<'evm> = evm::standard::Invoker<'evm, 'evm, Resolver<'evm>>;
