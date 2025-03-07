use aa_bundler_primitives::{
    consts::rpc_error_codes::{
        ENTITY_BANNED, EXECUTION, EXPIRATION, OPCODE, SANITY_CHECK, SIGNATURE, STAKE_TOO_LOW,
        VALIDATION,
    },
    reputation::ReputationError,
    sanity_check::SanityCheckError,
    simulation::SimulationError,
    uopool::VerificationError,
};
use ethers::abi::AbiEncode;
use jsonrpsee::types::{error::ErrorCode, ErrorObject, ErrorObjectOwned};
use serde_json::json;

pub struct JsonRpcError(pub ErrorObjectOwned);

impl From<JsonRpcError> for ErrorObjectOwned {
    fn from(err: JsonRpcError) -> Self {
        err.0
    }
}

impl From<SanityCheckError> for JsonRpcError {
    fn from(err: SanityCheckError) -> Self {
        JsonRpcError(
        match err {
            SanityCheckError::SenderOrInitCode { sender, init_code } => {
                ErrorObject::owned(
                    SANITY_CHECK,
                    format!(
                        "Either the sender {sender} is an existing contract, or the initCode {init_code} is not empty (but not both)",
                    ),
                    None::<bool>,
                )
            },
            SanityCheckError::FactoryVerification { init_code } => ErrorObject::owned(
                SANITY_CHECK,
                format!("Init code {init_code} is not valid (factory check)",),
                None::<bool>,
            ),
            SanityCheckError::HighVerificationGasLimit {
                verification_gas_limit,
                max_verification_gas,
            } => ErrorObject::owned(
                SANITY_CHECK,
                format!(
                    "Verification gas limit {verification_gas_limit} is higher than max verification gas {max_verification_gas}",
                ),
                None::<bool>,
            ),
            SanityCheckError::LowPreVerificationGas {
                pre_verification_gas,
                pre_verification_gas_expected,
            } => ErrorObject::owned(
                SANITY_CHECK,
                format!(
                    "Pre-verification gas {pre_verification_gas} is lower than calculated pre-verification gas {pre_verification_gas_expected}",
                ),
                None::<bool>,
            ),
            SanityCheckError::PaymasterVerification { paymaster_and_data } => {
                ErrorObject::owned(
                    SANITY_CHECK,
                    format!(
                        "Paymaster and data {paymaster_and_data} is invalid (paymaster check)",
                    ),
                    None::<bool>,
                )
            },
            SanityCheckError::LowCallGasLimit {
                call_gas_limit,
                call_gas_limit_expected,
            } => ErrorObject::owned(
                SANITY_CHECK,
                format!(
                    "Call gas limit {call_gas_limit} is lower than call gas estimation {call_gas_limit_expected}",
                ),
                None::<bool>,
            ),
            SanityCheckError::LowMaxFeePerGas {
                max_fee_per_gas,
                base_fee,
            } => ErrorObject::owned(
                SANITY_CHECK,
                format!(
                    "Max fee per gas {max_fee_per_gas} is lower than base fee {base_fee}",
                ),
                None::<bool>,
            ),
            SanityCheckError::HighMaxPriorityFeePerGas {
                max_priority_fee_per_gas,
                max_fee_per_gas,
            } => ErrorObject::owned(
                SANITY_CHECK,
                format!(
                    "Max priority fee per gas {max_priority_fee_per_gas} is higher than max fee per gas {max_fee_per_gas}",
                ),
                None::<bool>,
            ),
            SanityCheckError::LowMaxPriorityFeePerGas {
                max_priority_fee_per_gas,
                min_priority_fee_per_gas,
            } => ErrorObject::owned(
                SANITY_CHECK,
                format!(
                    "Max priority fee per gas {max_priority_fee_per_gas} is lower than min priority fee per gas {min_priority_fee_per_gas}",
                ),
                None::<bool>,
            ),
            SanityCheckError::SenderVerification { sender, message } => ErrorObject::owned(
                SANITY_CHECK,
                format!("Sender {sender} {message}",),
                None::<bool>,
            ),
            SanityCheckError::Validation { message } => {
                ErrorObject::owned(
                    VALIDATION,
                    message,
                    None::<bool>,
                )
            },
            SanityCheckError::MiddlewareError { message } => {
                ErrorObject::owned(
                    ErrorCode::InternalError.code(),
                    message,
                    None::<bool>,
                )
            },
            SanityCheckError::UnknownError { message } => {
                ErrorObject::owned(
                    SANITY_CHECK,
                    message,
                    None::<bool>,
                )
            },
        }
    )
    }
}

impl From<SimulationError> for JsonRpcError {
    fn from(err: SimulationError) -> Self {
        JsonRpcError(match err {
            SimulationError::Signature {} => ErrorObject::owned(
                SIGNATURE,
                "Invalid UserOp signature or paymaster signature",
                None::<bool>,
            ),
            SimulationError::Expiration {
                valid_after,
                valid_until,
                paymaster,
            } => ErrorObject::owned(
                EXPIRATION,
                "User operation is expired or will expire soon",
                {
                    if let Some(paymaster) = paymaster {
                        Some(json!({
                            "valid_after": valid_after, "valid_until": valid_until, "paymaster": paymaster,
                        }))
                    } else {
                        Some(json!({
                            "valid_after": valid_after, "valid_until": valid_until,
                        }))
                    }
                },
            ),
            SimulationError::Validation { message } => {
                ErrorObject::owned(VALIDATION, message, None::<bool>)
            }
            SimulationError::ForbiddenOpcode { entity, opcode } => ErrorObject::owned(
                OPCODE,
                format!("{entity} uses banned opcode: {opcode}"),
                None::<bool>,
            ),
            SimulationError::Execution { message } => {
                ErrorObject::owned(EXECUTION, message, None::<bool>)
            }
            SimulationError::StorageAccess { slot } => ErrorObject::owned(
                OPCODE,
                format!("Storage access validation failed for slot: {slot}"),
                None::<bool>,
            ),
            SimulationError::Unstaked { entity, message } => {
                ErrorObject::owned(OPCODE, format!("unstaked {entity} {message}"), None::<bool>)
            }
            SimulationError::CallStack { message } => {
                ErrorObject::owned(OPCODE, message, None::<bool>)
            }
            SimulationError::CodeHashes { message } => {
                ErrorObject::owned(OPCODE, message, None::<bool>)
            }
            SimulationError::OutOfGas {} => {
                ErrorObject::owned(OPCODE, "User operation out of gas", None::<bool>)
            }
            SimulationError::MiddlewareError { message } => {
                ErrorObject::owned(ErrorCode::InternalError.code(), message, None::<bool>)
            }
            SimulationError::UnknownError { message } => {
                ErrorObject::owned(ErrorCode::InternalError.code(), message, None::<bool>)
            }
        })
    }
}

impl From<ReputationError> for JsonRpcError {
    fn from(err: ReputationError) -> Self {
        JsonRpcError(
        match err {
            ReputationError::EntityBanned { address, title } => ErrorObject::owned(
                ENTITY_BANNED,
                format!("{title} with address {address} is banned",),
                Some(json!({
                    title: address.to_string(),
                })),
            ),
            ReputationError::StakeTooLow {
                address,
                title,
                min_stake,
                min_unstake_delay,
            } => ErrorObject::owned(
                STAKE_TOO_LOW,
                format!(
                    "{title} with address {address} stake is lower than {min_stake}",
                ),
                Some(json!({
                    title: address.to_string(),
                    "minimumStake": AbiEncode::encode_hex(min_stake),
                    "minimumUnstakeDelay": AbiEncode::encode_hex(min_unstake_delay),
                })),
            ),
            ReputationError::UnstakeDelayTooLow {
                address,
                title,
                min_stake,
                min_unstake_delay,
            } => ErrorObject::owned(
                STAKE_TOO_LOW,
                format!(
                    "{title} with address {address} unstake delay is lower than {min_unstake_delay}",
                ),
                Some(json!({
                    title: address.to_string(),
                    "minimumStake": AbiEncode::encode_hex(min_stake),
                    "minimumUnstakeDelay": AbiEncode::encode_hex(min_unstake_delay),
                })),
            ),
            ReputationError::UnknownError { message } => {
                ErrorObject::owned(ErrorCode::InternalError.code(), message, None::<bool>)
            }
        }
    )
    }
}

impl From<VerificationError> for JsonRpcError {
    fn from(err: VerificationError) -> Self {
        match err {
            VerificationError::SanityCheck(err) => err.into(),
            VerificationError::Simulation(err) => err.into(),
        }
    }
}

impl From<tonic::Status> for JsonRpcError {
    fn from(s: tonic::Status) -> Self {
        JsonRpcError(ErrorObject::owned(
            ErrorCode::InternalError.code(),
            format!("gRPC error: {}", s.message()),
            None::<bool>,
        ))
    }
}

impl From<serde_json::Error> for JsonRpcError {
    fn from(err: serde_json::Error) -> Self {
        JsonRpcError(ErrorObject::owned(
            ErrorCode::ParseError.code(),
            format!("JSON serializing error: {err}"),
            None::<bool>,
        ))
    }
}
