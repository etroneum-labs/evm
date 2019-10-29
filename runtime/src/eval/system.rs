use alloc::vec::Vec;
use primitive_types::{H256, U256};
use sha3::{Keccak256, Digest};
use crate::{Runtime, ExitError, Handler, Capture, Transfer,
			CreateScheme, CallScheme, Context, ExitSucceed};
use super::Control;

pub fn sha3<H: Handler>(runtime: &mut Runtime) -> Control<H> {
	pop_u256!(runtime, from, len);
	let from = as_usize_or_fail!(from);
	let len = as_usize_or_fail!(len);
	let data = runtime.machine.memory().get(from, len);
	let ret = Keccak256::digest(data.as_slice());
	push!(runtime, H256::from_slice(ret.as_slice()));

	Control::Continue
}

pub fn chainid<H: Handler>(runtime: &mut Runtime, handler: &H) -> Control<H> {
	push_u256!(runtime, handler.chain_id());

	Control::Continue
}

pub fn address<H: Handler>(runtime: &mut Runtime) -> Control<H> {
	let ret = H256::from(runtime.context.address);
	push!(runtime, ret);

	Control::Continue
}

pub fn balance<H: Handler>(runtime: &mut Runtime, handler: &H) -> Control<H> {
	pop!(runtime, address);
	push_u256!(runtime, handler.balance(address.into()));

	Control::Continue
}

pub fn selfbalance<H: Handler>(runtime: &mut Runtime, handler: &H) -> Control<H> {
	push_u256!(runtime, handler.balance(runtime.context.address));

	Control::Continue
}

pub fn origin<H: Handler>(runtime: &mut Runtime, handler: &H) -> Control<H> {
	let ret = H256::from(handler.origin());
	push!(runtime, ret);

	Control::Continue
}

pub fn caller<H: Handler>(runtime: &mut Runtime) -> Control<H> {
	let ret = H256::from(runtime.context.caller);
	push!(runtime, ret);

	Control::Continue
}

pub fn callvalue<H: Handler>(runtime: &mut Runtime) -> Control<H> {
	let mut ret = H256::default();
	runtime.context.apparent_value.to_big_endian(&mut ret[..]);
	push!(runtime, ret);

	Control::Continue
}

pub fn gasprice<H: Handler>(runtime: &mut Runtime, handler: &H) -> Control<H> {
	let mut ret = H256::default();
	handler.gas_price().to_big_endian(&mut ret[..]);
	push!(runtime, ret);

	Control::Continue
}

pub fn extcodesize<H: Handler>(runtime: &mut Runtime, handler: &H) -> Control<H> {
	pop!(runtime, address);
	push_u256!(runtime, handler.code_size(address.into()));

	Control::Continue
}

pub fn extcodehash<H: Handler>(runtime: &mut Runtime, handler: &H) -> Control<H> {
	pop!(runtime, address);
	push!(runtime, handler.code_hash(address.into()));

	Control::Continue
}

pub fn extcodecopy<H: Handler>(runtime: &mut Runtime, handler: &H) -> Control<H> {
	pop!(runtime, address);
	pop_u256!(runtime, memory_offset, code_offset, len);

	match runtime.machine.memory_mut().copy_large(
		memory_offset,
		code_offset,
		len,
		&handler.code(address.into())
	) {
		Ok(()) => (),
		Err(e) => return Control::Exit(Err(e)),
	};

	Control::Continue
}

pub fn returndatasize<H: Handler>(runtime: &mut Runtime) -> Control<H> {
	let size = U256::from(runtime.return_data_buffer.len());
	push_u256!(runtime, size);

	Control::Continue
}

pub fn returndatacopy<H: Handler>(runtime: &mut Runtime) -> Control<H> {
	pop_u256!(runtime, memory_offset, data_offset, len);

	match runtime.machine.memory_mut().copy_large(memory_offset, data_offset, len, &runtime.return_data_buffer) {
		Ok(()) => Control::Continue,
		Err(e) => Control::Exit(Err(e)),
	}
}

pub fn blockhash<H: Handler>(runtime: &mut Runtime, handler: &H) -> Control<H> {
	pop_u256!(runtime, number);
	push!(runtime, handler.block_hash(number));

	Control::Continue
}

pub fn coinbase<H: Handler>(runtime: &mut Runtime, handler: &H) -> Control<H> {
	push!(runtime, handler.block_coinbase().into());
	Control::Continue
}

pub fn timestamp<H: Handler>(runtime: &mut Runtime, handler: &H) -> Control<H> {
	push_u256!(runtime, handler.block_timestamp());
	Control::Continue
}

pub fn number<H: Handler>(runtime: &mut Runtime, handler: &H) -> Control<H> {
	push_u256!(runtime, handler.block_number());
	Control::Continue
}

pub fn difficulty<H: Handler>(runtime: &mut Runtime, handler: &H) -> Control<H> {
	push_u256!(runtime, handler.block_difficulty());
	Control::Continue
}

pub fn gaslimit<H: Handler>(runtime: &mut Runtime, handler: &H) -> Control<H> {
	push_u256!(runtime, handler.block_gas_limit());
	Control::Continue
}

pub fn sload<H: Handler>(runtime: &mut Runtime, handler: &H) -> Control<H> {
	pop!(runtime, index);
	push!(runtime, handler.storage(runtime.context.address, index));

	Control::Continue
}

pub fn sstore<H: Handler>(runtime: &mut Runtime, handler: &mut H) -> Control<H> {
	pop!(runtime, index, value);
	match handler.set_storage(runtime.context.address, index, value) {
		Ok(()) => Control::Continue,
		Err(e) => Control::Exit(Err(e)),
	}
}

pub fn gas<H: Handler>(runtime: &mut Runtime, handler: &H) -> Control<H> {
	push_u256!(runtime, handler.gas_left());

	Control::Continue
}

pub fn log<H: Handler>(runtime: &mut Runtime, n: u8, handler: &mut H) -> Control<H> {
	pop_u256!(runtime, offset, len);
	let offset = as_usize_or_fail!(offset);
	let len = as_usize_or_fail!(len);
	let data = runtime.machine.memory().get(offset, len);

	let mut topics = Vec::new();
	for _ in 0..(n as usize) {
		match runtime.machine.stack_mut().pop() {
			Ok(value) => { topics.push(value); }
			Err(e) => return Control::Exit(Err(e)),
		}
	}

	match handler.log(runtime.context.address, topics, data) {
		Ok(()) => Control::Continue,
		Err(e) => Control::Exit(Err(e)),
	}
}

pub fn suicide<H: Handler>(runtime: &mut Runtime, handler: &mut H) -> Control<H> {
	pop!(runtime, target);

	let balance = handler.balance(runtime.context.address);
	match handler.transfer(Transfer {
		source: runtime.context.address,
		target: target.into(),
		value: balance
	}) {
		Ok(()) => (),
		Err(e) => return Control::Exit(Err(e)),
	}

	match handler.mark_delete(runtime.context.address) {
		Ok(()) => (),
		Err(e) => return Control::Exit(Err(e)),
	}

	Control::Exit(Ok(ExitSucceed::Suicided))
}

pub fn create<H: Handler>(
	runtime: &mut Runtime,
	is_create2: bool,
	handler: &mut H,
) -> Control<H> {
	pop_u256!(runtime, value, code_offset, len);

	let code_offset = as_usize_or_fail!(code_offset);
	let len = as_usize_or_fail!(len);
	let code = runtime.machine.memory().get(code_offset, len);

	let scheme = if is_create2 {
		pop!(runtime, salt);
		let code_hash = H256::from_slice(Keccak256::digest(&code).as_slice());

		let mut hasher = Keccak256::new();
		hasher.input(&[0xff]);
		hasher.input(&runtime.context.address[..]);
		hasher.input(&salt[..]);
		hasher.input(&code_hash[..]);

		let target = H256::from_slice(hasher.result().as_slice());
		CreateScheme::Fixed(target.into())
	} else {
		CreateScheme::Dynamic
	};

	let create_address = match handler.create_address(runtime.context.address, scheme) {
		Ok(address) => address,
		Err(e) => {
			push!(runtime, H256::default());

			return if handler.is_recoverable() {
				Control::Continue
			} else {
				Control::Exit(Err(e))
			}
		},
	};

	let context = Context {
		address: create_address,
		caller: runtime.context.address,
		apparent_value: value,
	};

	let transfer = Some(Transfer {
		source: runtime.context.address,
		target: create_address,
		value
	});

	match handler.create(create_address, transfer, code, None, context) {
		Ok(Capture::Exit(_)) => {
			push!(runtime, create_address.into());
			Control::Continue
		},
		Ok(Capture::Trap(interrupt)) => {
			push!(runtime, H256::default());
			Control::CreateInterrupt(interrupt)
		},
		Err(e) => {
			push!(runtime, H256::default());

			if handler.is_recoverable() {
				Control::Continue
			} else {
				Control::Exit(Err(e))
			}
		},
	}
}

pub fn call<'config, H: Handler>(
	runtime: &mut Runtime,
	scheme: CallScheme,
	handler: &mut H,
) -> Control<H> {
	pop_u256!(runtime, gas);
	pop!(runtime, to);
	let mut gas = as_usize_or_fail!(gas);

	let value = match scheme {
		CallScheme::Call | CallScheme::CallCode => {
			pop_u256!(runtime, value);
			value
		},
		CallScheme::DelegateCall | CallScheme::StaticCall => {
			U256::zero()
		},
	};

	if value != U256::zero() {
		gas += runtime.config.call_stipend;
	}

	pop_u256!(runtime, in_offset, in_len, out_offset, out_len);

	let in_offset = as_usize_or_fail!(in_offset);
	let in_len = as_usize_or_fail!(in_len);
	let out_offset = as_usize_or_fail!(out_offset);
	let out_len = as_usize_or_fail!(out_len);

	let input = runtime.machine.memory().get(in_offset, in_len);
	let context = match scheme {
		CallScheme::Call | CallScheme::StaticCall => Context {
			address: to.into(),
			caller: runtime.context.address,
			apparent_value: value,
		},
		CallScheme::CallCode => Context {
			address: runtime.context.address,
			caller: runtime.context.address,
			apparent_value: value,
		},
		CallScheme::DelegateCall => Context {
			address: runtime.context.address,
			caller: runtime.context.caller,
			apparent_value: runtime.context.apparent_value,
		},
	};

	let transfer = if scheme == CallScheme::Call {
		Some(Transfer {
			source: runtime.context.address,
			target: to.into(),
			value: value.into()
		})
	} else if scheme == CallScheme::CallCode {
		Some(Transfer {
			source: runtime.context.address,
			target: runtime.context.address,
			value: value.into()
		})
	} else {
		None
	};

	match handler.call(to.into(), transfer, input, Some(gas), scheme == CallScheme::StaticCall, context) {
		Ok(Capture::Exit((_, return_data))) => {
			runtime.return_data_buffer = return_data;

			match runtime.machine.memory_mut().set(
				out_offset, &runtime.return_data_buffer[..], Some(out_len)
			) {
				Ok(()) => {
					push_u256!(runtime, U256::one());
					Control::Continue
				},
				Err(e) => {
					push_u256!(runtime, U256::zero());

					if handler.is_recoverable() {
						Control::Continue
					} else {
						Control::Exit(Err(e))
					}
				},
			}

		},
		Ok(Capture::Trap(interrupt)) => {
			push!(runtime, H256::default());
			Control::CallInterrupt(interrupt)
		},
		Err(e) => {
			push_u256!(runtime, U256::zero());

			if handler.is_recoverable() {
				Control::Continue
			} else {
				Control::Exit(Err(e))
			}
		},
	}
}