//! Browser runtime for the WASM build of `bridge_cffi`.

use std::{cell::RefCell, sync::Arc};

use bex_project::{Bex, BexArgs, FunctionCallContextBuilder};
use bridge_ctypes::{HANDLE_TABLE, kwargs_to_bex_values};
use prost::Message;

use crate::{BridgeError, call_and_encode, error_to_outbound};

thread_local! {
    static RUNTIME: RefCell<Option<Arc<dyn Bex>>> = RefCell::new(None);
}

pub fn initialize_runtime_from_bytecode(bytecode: &[u8]) -> Result<(), BridgeError> {
    let sys_ops = sys_ops::SysOpsBuilder::new().build();
    initialize_runtime_from_bytecode_with_sys_ops(bytecode, sys_ops)
}

pub fn initialize_runtime_from_bytecode_with_sys_ops(
    bytecode: &[u8],
    sys_ops: sys_ops::SysOps,
) -> Result<(), BridgeError> {
    let runtime = bex_project::new_from_bytecode(bytecode, sys_ops)?;
    RUNTIME.with(|slot| {
        *slot.borrow_mut() = Some(runtime);
    });
    Ok(())
}

fn runtime() -> Result<Arc<dyn Bex>, BridgeError> {
    RUNTIME.with(|slot| slot.borrow().clone().ok_or(BridgeError::NotInitialized))
}

struct DecodedCall {
    runtime: Arc<dyn Bex>,
    function_name: String,
    args: BexArgs,
    context: bex_project::FunctionCallContext,
}

fn decode_call(function_name: &str, encoded_args: &[u8]) -> Result<DecodedCall, BridgeError> {
    use bridge_ctypes::baml_bridge::cffi::CallFunctionArgs;

    let call = CallFunctionArgs::decode(encoded_args).map_err(bridge_ctypes::CtypesError::from)?;
    if call.call_id == 0 {
        return Err(BridgeError::InvalidCallId);
    }
    let type_args = bridge_ctypes::proto_ty_args_to_named(&call.type_args)?;
    let kwargs = kwargs_to_bex_values(call.kwargs, &HANDLE_TABLE)?;
    let context = FunctionCallContextBuilder::new(bex_project::CallId(call.call_id))
        .with_type_args(type_args)
        .build();
    Ok(DecodedCall {
        runtime: runtime()?,
        function_name: function_name.to_string(),
        args: kwargs.into(),
        context,
    })
}

pub async fn call_function_in_wasm(function_name: &str, encoded_args: &[u8]) -> Vec<u8> {
    let call = match decode_call(function_name, encoded_args) {
        Ok(call) => call,
        Err(error) => return error_to_outbound(error),
    };
    call_and_encode(call.runtime, call.function_name, call.args, call.context).await
}

pub fn call_function_in_wasm_sync(function_name: &str, encoded_args: &[u8]) -> Vec<u8> {
    futures::executor::block_on(call_function_in_wasm(function_name, encoded_args))
}

pub fn new_function_call_id() -> u64 {
    bex_project::CallId::next().0
}

pub fn cancel_function_call_by_id(id: u64) -> bool {
    if id == 0 {
        return false;
    }
    runtime()
        .and_then(|runtime| {
            runtime
                .cancel_function_call(bex_project::CallId(id))
                .map_err(BridgeError::from)
        })
        .is_ok()
}
