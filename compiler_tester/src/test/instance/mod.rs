//!
//! The test contract instance used for building.
//!

pub mod eravm;
pub mod evm;

use self::eravm::Instance as EraVMInstance;
use self::evm::Instance as EVMInstance;

///
/// The test contract instance used for building.
///
#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Clone)]
pub enum Instance {
    /// The EraVM instance.
    EraVM(EraVMInstance),
    /// The EVM instance.
    EVM(EVMInstance),
}

impl Instance {
    ///
    /// A shortcut constructor for the EraVM instance.
    ///
    pub fn eravm(
        path: String,
        address: Option<web3::types::Address>,
        is_main: bool,
        is_library: bool,
        code_hash: web3::types::U256,
    ) -> Self {
        Self::EraVM(EraVMInstance::new(
            path, address, is_main, is_library, code_hash,
        ))
    }

    ///
    /// A shortcut constructor for the EVM instance.
    ///
    pub fn evm(
        path: String,
        address: Option<web3::types::Address>,
        is_main: bool,
        is_library: bool,
        deploy_code: Vec<u8>,
        runtime_code_size: usize,
    ) -> Self {
        Self::EVM(EVMInstance::new(
            path,
            address,
            is_main,
            is_library,
            deploy_code,
            runtime_code_size,
        ))
    }

    ///
    /// Sets the address of the instance.
    ///
    pub fn set_address(&mut self, address: web3::types::Address) {
        match self {
            Self::EraVM(instance) => instance.address = Some(address),
            Self::EVM(instance) => instance.address = Some(address),
        }
    }

    ///
    /// Returns the instance path if applicable.
    ///
    pub fn path(&self) -> &str {
        match self {
            Self::EraVM(instance) => instance.path.as_str(),
            Self::EVM(instance) => instance.path.as_str(),
        }
    }

    ///
    /// Whether the instance is main.
    ///
    pub fn is_main(&self) -> bool {
        match self {
            Self::EraVM(instance) => instance.is_main,
            Self::EVM(instance) => instance.is_main,
        }
    }

    ///
    /// Whether the instance is a library.
    ///
    pub fn is_library(&self) -> bool {
        match self {
            Self::EraVM(instance) => instance.is_library,
            Self::EVM(instance) => instance.is_library,
        }
    }

    ///
    /// Returns the instance address if applicable.
    ///
    pub fn address(&self) -> Option<&web3::types::Address> {
        match self {
            Self::EraVM(instance) => instance.address.as_ref(),
            Self::EVM(instance) => instance.address.as_ref(),
        }
    }
}
