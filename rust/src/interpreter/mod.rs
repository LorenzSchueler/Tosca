use std::{cmp::min, mem, slice};

use bnum::types::U256;
use evmc_vm::{
    ExecutionContext, ExecutionMessage, Revision, StatusCode, StepResult, StepStatusCode, Uint256,
};

use crate::{
    interpreter::jumpdest::get_jump_destinations,
    types::{opcode, u256},
};

mod jumpdest;

pub fn run(
    revision: Revision,
    code: &[u8],
    message: &ExecutionMessage,
    context: &mut ExecutionContext,
    mut step_status_code: StepStatusCode,
    mut pc: usize,
    mut gas_refund: i64,
    mut stack: Vec<u256>,
    mut memory: Vec<u8>,
    mut last_call_return_data: Option<Vec<u8>>,
    steps: Option<i32>,
) -> Result<StepResult, (StepStatusCode, StatusCode)> {
    let mut gas_left = message.gas();
    let mut status_code = StatusCode::EVMC_SUCCESS;
    let mut output = None;
    let jump_destinations = get_jump_destinations(code);

    println!("running test");
    for _ in 0..steps.unwrap_or(i32::MAX) {
        if pc >= code.len() {
            return Err((StepStatusCode::EVMC_STEP_FAILED, StatusCode::EVMC_FAILURE));
        }
        match code[pc] {
            //} unsafe { mem::transmute::<u8, Opcode>(code[pc]) } {
            opcode::STOP => {
                step_status_code = StepStatusCode::EVMC_STEP_STOPPED;
                status_code = StatusCode::EVMC_SUCCESS;
                break;
            }
            opcode::ADD => {
                consume_gas::<3>(&mut gas_left)?;
                let [top, top2] = pop_from_stack(&mut stack)?;
                stack.push(top + top2);
                pc += 1;
            }
            opcode::MUL => {
                consume_gas::<5>(&mut gas_left)?;
                let [top, top2] = pop_from_stack(&mut stack)?;
                stack.push(top * top2);
                pc += 1;
            }
            opcode::SUB => {
                consume_gas::<3>(&mut gas_left)?;
                let [top, top2] = pop_from_stack(&mut stack)?;
                stack.push(top - top2);
                pc += 1;
            }
            opcode::DIV => {
                consume_gas::<5>(&mut gas_left)?;
                let [top, top2] = pop_from_stack(&mut stack)?;
                stack.push(top / top2);
                pc += 1;
            }
            opcode::SDIV => {
                consume_gas::<5>(&mut gas_left)?;
                let [top, top2] = pop_from_stack(&mut stack)?;
                stack.push(top.sdiv(top2));
                pc += 1;
            }
            opcode::MOD => {
                consume_gas::<5>(&mut gas_left)?;
                let [top, top2] = pop_from_stack(&mut stack)?;
                stack.push(top % top2);
                pc += 1;
            }
            opcode::SMOD => {
                consume_gas::<5>(&mut gas_left)?;
                let [top, top2] = pop_from_stack(&mut stack)?;
                stack.push(top.srem(top2));
                pc += 1;
            }
            opcode::ADDMOD => {
                consume_gas::<8>(&mut gas_left)?;
                let [top, top2, top3] = pop_from_stack(&mut stack)?;
                stack.push(u256::addmod(top, top2, top3));
                pc += 1;
            }
            opcode::MULMOD => {
                consume_gas::<8>(&mut gas_left)?;
                let [top, top2, top3] = pop_from_stack(&mut stack)?;
                stack.push(u256::mulmod(top, top2, top3));
                pc += 1;
            }
            opcode::EXP => {
                consume_gas::<10>(&mut gas_left)?;
                let [top, top2] = pop_from_stack(&mut stack)?;
                let top2_bytes: [u8; 32] = *top2;
                let mut cost_multiplier = 32;
                for byte in top2_bytes.into_iter() {
                    if byte == 0 {
                        cost_multiplier -= 1;
                    } else {
                        break;
                    }
                }
                let dyn_gas = 50 * cost_multiplier;
                consume_dyn_gas(&mut gas_left, dyn_gas)?;
                stack.push(top.pow(top2));
                pc += 1;
            }
            opcode::SIGNEXTEND => {
                consume_gas::<5>(&mut gas_left)?;
                let [top, top2] = pop_from_stack(&mut stack)?;
                stack.push(u256::signextend(top, top2));
                pc += 1;
            }
            opcode::LT => {
                consume_gas::<3>(&mut gas_left)?;
                let [top, top2] = pop_from_stack(&mut stack)?;
                stack.push(((top < top2) as u8).into());
                pc += 1;
            }
            opcode::GT => {
                consume_gas::<3>(&mut gas_left)?;
                let [top, top2] = pop_from_stack(&mut stack)?;
                stack.push(((top > top2) as u8).into());
                pc += 1;
            }
            opcode::SLT => {
                consume_gas::<3>(&mut gas_left)?;
                let [top, top2] = pop_from_stack(&mut stack)?;
                stack.push(((top.slt(&top2)) as u8).into());
                pc += 1;
            }
            opcode::SGT => {
                consume_gas::<3>(&mut gas_left)?;
                let [top, top2] = pop_from_stack(&mut stack)?;
                stack.push(((top.sgt(&top2)) as u8).into());
                pc += 1;
            }
            opcode::EQ => {
                consume_gas::<3>(&mut gas_left)?;
                let [top, top2] = pop_from_stack(&mut stack)?;
                stack.push(((top == top2) as u8).into());
                pc += 1;
            }
            opcode::ISZERO => {
                consume_gas::<3>(&mut gas_left)?;
                let [top] = pop_from_stack(&mut stack)?;
                stack.push(((top == u256::ZERO) as u8).into());
                pc += 1;
            }
            opcode::AND => {
                consume_gas::<3>(&mut gas_left)?;
                let [top, top2] = pop_from_stack(&mut stack)?;
                stack.push(top & top2);
                pc += 1;
            }
            opcode::OR => {
                consume_gas::<3>(&mut gas_left)?;
                let [top, top2] = pop_from_stack(&mut stack)?;
                stack.push(top | top2);
                pc += 1;
            }
            opcode::XOR => {
                consume_gas::<3>(&mut gas_left)?;
                let [top, top2] = pop_from_stack(&mut stack)?;
                stack.push(top ^ top2);
                pc += 1;
            }
            opcode::NOT => {
                consume_gas::<3>(&mut gas_left)?;
                let [top] = pop_from_stack(&mut stack)?;
                stack.push(!top);
                pc += 1;
            }
            opcode::BYTE => {
                consume_gas::<3>(&mut gas_left)?;
                let [top, top2] = pop_from_stack(&mut stack)?;
                stack.push(top2.byte(top));
                pc += 1;
            }
            opcode::SHL => {
                consume_gas::<3>(&mut gas_left)?;
                let [top, top2] = pop_from_stack(&mut stack)?;
                stack.push(top2 << top);
                pc += 1;
            }
            opcode::SHR => {
                consume_gas::<3>(&mut gas_left)?;
                let [top, top2] = pop_from_stack(&mut stack)?;
                stack.push(top2 >> top);
                pc += 1;
            }
            opcode::SAR => {
                consume_gas::<3>(&mut gas_left)?;
                let [top, top2] = pop_from_stack(&mut stack)?;
                stack.push(top2.sar(top));
                pc += 1;
            }
            opcode::SHA3 => unimplemented!(),
            opcode::ADDRESS => {
                consume_gas::<2>(&mut gas_left)?;
                check_stack_overflow::<1>(&stack)?;
                stack.push(message.recipient().into());
                pc += 1;
            }
            opcode::BALANCE => unimplemented!(),
            opcode::ORIGIN => {
                consume_gas::<2>(&mut gas_left)?;
                check_stack_overflow::<1>(&stack)?;
                stack.push(context.get_tx_context().tx_origin.into());
                pc += 1;
            }
            opcode::CALLER => {
                consume_gas::<2>(&mut gas_left)?;
                check_stack_overflow::<1>(&stack)?;
                stack.push(message.sender().into());
                pc += 1;
            }
            opcode::CALLVALUE => {
                consume_gas::<2>(&mut gas_left)?;
                check_stack_overflow::<1>(&stack)?;
                stack.push((*message.value()).into());
                pc += 1;
            }
            opcode::CALLDATALOAD => {
                consume_gas::<3>(&mut gas_left)?;
                let [offset] = pop_from_stack(&mut stack)?;
                let offset: U256 = offset.into();
                let call_data = message.input().unwrap();
                if offset >= U256::from(call_data.len()) {
                    stack.push(u256::ZERO);
                } else {
                    let start = offset.digits()[0] as usize;
                    let end = min(call_data.len(), start + 32);
                    let mut bytes = [0; 32];
                    bytes[..end - start].copy_from_slice(&call_data[start..end]);
                    stack.push(bytes.into());
                }
                pc += 1;
            }
            opcode::CALLDATASIZE => {
                consume_gas::<2>(&mut gas_left)?;
                check_stack_overflow::<1>(&stack)?;
                let call_data = message.input().unwrap();
                stack.push((call_data.len() as u64).into());
                pc += 1;
            }
            opcode::PUSH0 => {
                check_min_revision(Revision::EVMC_SHANGHAI, revision)?;
                consume_gas::<2>(&mut gas_left)?;
                check_stack_overflow::<1>(&stack)?;
                stack.push(u256::ZERO);
                pc += 1;
            }
            opcode::CALLDATACOPY => unimplemented!(),
            opcode::CODESIZE => {
                consume_gas::<2>(&mut gas_left)?;
                check_stack_overflow::<1>(&stack)?;
                stack.push((code.len() as u64).into());
                pc += 1;
            }
            opcode::CODECOPY => unimplemented!(),
            opcode::GASPRICE => {
                consume_gas::<2>(&mut gas_left)?;
                check_stack_overflow::<1>(&stack)?;
                stack.push(context.get_tx_context().tx_gas_price.into());
                pc += 1;
            }
            opcode::EXTCODESIZE => unimplemented!(),
            opcode::EXTCODECOPY => unimplemented!(),
            opcode::RETURNDATASIZE => unimplemented!(),
            opcode::RETURNDATACOPY => unimplemented!(),
            opcode::EXTCODEHASH => unimplemented!(),
            opcode::BLOCKHASH => {
                consume_gas::<20>(&mut gas_left)?;
                let [block_number] = pop_from_stack(&mut stack)?;
                let current_block_number = context.get_tx_context().block_number;
                let idx = U256::from(block_number);
                if idx > U256::from_digit((current_block_number + 255) as u64) {
                    stack.push(u256::ZERO);
                } else {
                    let idx = idx.digits()[0] as i64;
                    stack.push(context.get_block_hash(idx).into());
                }
                pc += 1;
            }
            opcode::COINBASE => {
                consume_gas::<2>(&mut gas_left)?;
                check_stack_overflow::<1>(&stack)?;
                stack.push(context.get_tx_context().block_coinbase.into());
                pc += 1;
            }
            opcode::TIMESTAMP => {
                consume_gas::<2>(&mut gas_left)?;
                check_stack_overflow::<1>(&stack)?;
                stack.push((context.get_tx_context().block_timestamp as u64).into());
                pc += 1;
            }
            opcode::NUMBER => {
                consume_gas::<2>(&mut gas_left)?;
                check_stack_overflow::<1>(&stack)?;
                stack.push((context.get_tx_context().block_number as u64).into());
                pc += 1;
            }
            opcode::PREVRANDAO => {
                consume_gas::<2>(&mut gas_left)?;
                check_stack_overflow::<1>(&stack)?;
                stack.push(context.get_tx_context().block_prev_randao.into());
                pc += 1;
            }
            opcode::GASLIMIT => {
                consume_gas::<2>(&mut gas_left)?;
                check_stack_overflow::<1>(&stack)?;
                stack.push((context.get_tx_context().block_gas_limit as u64).into());
                pc += 1;
            }
            opcode::CHAINID => {
                consume_gas::<2>(&mut gas_left)?;
                check_stack_overflow::<1>(&stack)?;
                stack.push(context.get_tx_context().chain_id.into());
                pc += 1;
            }
            opcode::SELFBALANCE => {
                consume_gas::<5>(&mut gas_left)?;
                check_stack_overflow::<1>(&stack)?;
                stack.push(context.get_balance(message.recipient()).into());
                pc += 1;
            }
            opcode::BASEFEE => {
                check_min_revision(Revision::EVMC_LONDON, revision)?;
                consume_gas::<2>(&mut gas_left)?;
                check_stack_overflow::<1>(&stack)?;
                stack.push(context.get_tx_context().block_base_fee.into());
                pc += 1;
            }
            opcode::BLOBHASH => {
                check_min_revision(Revision::EVMC_CANCUN, revision)?;
                consume_gas::<3>(&mut gas_left)?;
                let [idx] = pop_from_stack(&mut stack)?;
                let idx = U256::from(idx);
                let count = context.get_tx_context().blob_hashes_count;
                if idx < U256::from(count) {
                    let idx = idx.digits()[0] as usize;

                    // TODO create new ExecutionTxContext type and do this conversion in mod ffi
                    let hashes = context.get_tx_context().blob_hashes;
                    let hashes: &[Uint256] = if hashes.is_null() {
                        assert_eq!(count, 0);
                        &[]
                    } else {
                        // SAFETY:
                        // hashes is not null and count > 0
                        unsafe { slice::from_raw_parts(hashes, count) }
                    };

                    let hash = hashes[idx];

                    stack.push(hash.into());
                } else {
                    stack.push(u256::ZERO);
                }
                pc += 1;
            }
            opcode::BLOBBASEFEE => {
                check_min_revision(Revision::EVMC_CANCUN, revision)?;
                consume_gas::<2>(&mut gas_left)?;
                check_stack_overflow::<1>(&stack)?;
                stack.push(context.get_tx_context().blob_base_fee.into());
                pc += 1;
            }
            opcode::POP => {
                consume_gas::<2>(&mut gas_left)?;
                let [_] = pop_from_stack(&mut stack)?;
                pc += 1;
            }
            opcode::MLOAD => unimplemented!(),
            opcode::MSTORE => unimplemented!(),
            opcode::MSTORE8 => unimplemented!(),
            opcode::SLOAD => unimplemented!(),
            opcode::SSTORE => unimplemented!(),
            opcode::JUMP => {
                consume_gas::<8>(&mut gas_left)?;
                let [dest] = pop_from_stack(&mut stack)?;
                jump(dest, &mut pc, &jump_destinations)?;
            }
            opcode::JUMPI => {
                consume_gas::<10>(&mut gas_left)?;
                let [dest, cond] = pop_from_stack(&mut stack)?;
                if cond == u256::ZERO {
                    pc += 1;
                } else {
                    jump(dest, &mut pc, &jump_destinations)?;
                }
            }
            opcode::PC => {
                consume_gas::<2>(&mut gas_left)?;
                check_stack_overflow::<1>(&stack)?;
                stack.push((pc as u64).into());
                pc += 1;
            }
            opcode::MSIZE => {
                consume_gas::<2>(&mut gas_left)?;
                check_stack_overflow::<1>(&stack)?;
                stack.push((memory.len() as u64).into());
                pc += 1;
            }
            opcode::GAS => {
                consume_gas::<2>(&mut gas_left)?;
                check_stack_overflow::<1>(&stack)?;
                stack.push((gas_left as u64).into());
                pc += 1;
            }
            opcode::JUMPDEST => {
                consume_gas::<1>(&mut gas_left)?;
                pc += 1;
            }
            opcode::TLOAD => unimplemented!(),
            opcode::TSTORE => unimplemented!(),
            opcode::MCOPY => unimplemented!(),
            opcode::PUSH1 => push::<1>(code, &mut pc, &mut stack, &mut gas_left)?,
            opcode::PUSH2 => push::<2>(code, &mut pc, &mut stack, &mut gas_left)?,
            opcode::PUSH3 => push::<3>(code, &mut pc, &mut stack, &mut gas_left)?,
            opcode::PUSH4 => push::<4>(code, &mut pc, &mut stack, &mut gas_left)?,
            opcode::PUSH5 => push::<5>(code, &mut pc, &mut stack, &mut gas_left)?,
            opcode::PUSH6 => push::<6>(code, &mut pc, &mut stack, &mut gas_left)?,
            opcode::PUSH7 => push::<7>(code, &mut pc, &mut stack, &mut gas_left)?,
            opcode::PUSH8 => push::<8>(code, &mut pc, &mut stack, &mut gas_left)?,
            opcode::PUSH9 => push::<9>(code, &mut pc, &mut stack, &mut gas_left)?,
            opcode::PUSH10 => push::<10>(code, &mut pc, &mut stack, &mut gas_left)?,
            opcode::PUSH11 => push::<11>(code, &mut pc, &mut stack, &mut gas_left)?,
            opcode::PUSH12 => push::<12>(code, &mut pc, &mut stack, &mut gas_left)?,
            opcode::PUSH13 => push::<13>(code, &mut pc, &mut stack, &mut gas_left)?,
            opcode::PUSH14 => push::<14>(code, &mut pc, &mut stack, &mut gas_left)?,
            opcode::PUSH15 => push::<15>(code, &mut pc, &mut stack, &mut gas_left)?,
            opcode::PUSH16 => push::<16>(code, &mut pc, &mut stack, &mut gas_left)?,
            opcode::PUSH17 => push::<17>(code, &mut pc, &mut stack, &mut gas_left)?,
            opcode::PUSH18 => push::<18>(code, &mut pc, &mut stack, &mut gas_left)?,
            opcode::PUSH19 => push::<19>(code, &mut pc, &mut stack, &mut gas_left)?,
            opcode::PUSH20 => push::<20>(code, &mut pc, &mut stack, &mut gas_left)?,
            opcode::PUSH21 => push::<21>(code, &mut pc, &mut stack, &mut gas_left)?,
            opcode::PUSH22 => push::<22>(code, &mut pc, &mut stack, &mut gas_left)?,
            opcode::PUSH23 => push::<23>(code, &mut pc, &mut stack, &mut gas_left)?,
            opcode::PUSH24 => push::<24>(code, &mut pc, &mut stack, &mut gas_left)?,
            opcode::PUSH25 => push::<25>(code, &mut pc, &mut stack, &mut gas_left)?,
            opcode::PUSH26 => push::<26>(code, &mut pc, &mut stack, &mut gas_left)?,
            opcode::PUSH27 => push::<27>(code, &mut pc, &mut stack, &mut gas_left)?,
            opcode::PUSH28 => push::<28>(code, &mut pc, &mut stack, &mut gas_left)?,
            opcode::PUSH29 => push::<29>(code, &mut pc, &mut stack, &mut gas_left)?,
            opcode::PUSH30 => push::<30>(code, &mut pc, &mut stack, &mut gas_left)?,
            opcode::PUSH31 => push::<31>(code, &mut pc, &mut stack, &mut gas_left)?,
            opcode::PUSH32 => push::<32>(code, &mut pc, &mut stack, &mut gas_left)?,
            opcode::DUP1 => dup::<1>(&mut pc, &mut stack, &mut gas_left)?,
            opcode::DUP2 => dup::<2>(&mut pc, &mut stack, &mut gas_left)?,
            opcode::DUP3 => dup::<3>(&mut pc, &mut stack, &mut gas_left)?,
            opcode::DUP4 => dup::<4>(&mut pc, &mut stack, &mut gas_left)?,
            opcode::DUP5 => dup::<5>(&mut pc, &mut stack, &mut gas_left)?,
            opcode::DUP6 => dup::<6>(&mut pc, &mut stack, &mut gas_left)?,
            opcode::DUP7 => dup::<7>(&mut pc, &mut stack, &mut gas_left)?,
            opcode::DUP8 => dup::<8>(&mut pc, &mut stack, &mut gas_left)?,
            opcode::DUP9 => dup::<9>(&mut pc, &mut stack, &mut gas_left)?,
            opcode::DUP10 => dup::<10>(&mut pc, &mut stack, &mut gas_left)?,
            opcode::DUP11 => dup::<11>(&mut pc, &mut stack, &mut gas_left)?,
            opcode::DUP12 => dup::<12>(&mut pc, &mut stack, &mut gas_left)?,
            opcode::DUP13 => dup::<13>(&mut pc, &mut stack, &mut gas_left)?,
            opcode::DUP14 => dup::<14>(&mut pc, &mut stack, &mut gas_left)?,
            opcode::DUP15 => dup::<15>(&mut pc, &mut stack, &mut gas_left)?,
            opcode::DUP16 => dup::<16>(&mut pc, &mut stack, &mut gas_left)?,
            opcode::SWAP1 => swap::<1>(&mut pc, &mut stack, &mut gas_left)?,
            opcode::SWAP2 => swap::<2>(&mut pc, &mut stack, &mut gas_left)?,
            opcode::SWAP3 => swap::<3>(&mut pc, &mut stack, &mut gas_left)?,
            opcode::SWAP4 => swap::<4>(&mut pc, &mut stack, &mut gas_left)?,
            opcode::SWAP5 => swap::<5>(&mut pc, &mut stack, &mut gas_left)?,
            opcode::SWAP6 => swap::<6>(&mut pc, &mut stack, &mut gas_left)?,
            opcode::SWAP7 => swap::<7>(&mut pc, &mut stack, &mut gas_left)?,
            opcode::SWAP8 => swap::<8>(&mut pc, &mut stack, &mut gas_left)?,
            opcode::SWAP9 => swap::<9>(&mut pc, &mut stack, &mut gas_left)?,
            opcode::SWAP10 => swap::<10>(&mut pc, &mut stack, &mut gas_left)?,
            opcode::SWAP11 => swap::<11>(&mut pc, &mut stack, &mut gas_left)?,
            opcode::SWAP12 => swap::<12>(&mut pc, &mut stack, &mut gas_left)?,
            opcode::SWAP13 => swap::<13>(&mut pc, &mut stack, &mut gas_left)?,
            opcode::SWAP14 => swap::<14>(&mut pc, &mut stack, &mut gas_left)?,
            opcode::SWAP15 => swap::<15>(&mut pc, &mut stack, &mut gas_left)?,
            opcode::SWAP16 => swap::<16>(&mut pc, &mut stack, &mut gas_left)?,
            opcode::LOG0 => unimplemented!(),
            opcode::LOG1 => unimplemented!(),
            opcode::LOG2 => unimplemented!(),
            opcode::LOG3 => unimplemented!(),
            opcode::LOG4 => unimplemented!(),
            opcode::CREATE => unimplemented!(),
            opcode::CALL => unimplemented!(),
            opcode::CALLCODE => unimplemented!(),
            opcode::RETURN => unimplemented!(),
            opcode::DELEGATECALL => unimplemented!(),
            opcode::CREATE2 => unimplemented!(),
            opcode::STATICCALL => unimplemented!(),
            opcode::REVERT => unimplemented!(),
            opcode::INVALID => unimplemented!(),
            opcode::SELFDESTRUCT => unimplemented!(),
            op => {
                println!("invalid opcode 0x{op:x?}");
                step_status_code = StepStatusCode::EVMC_STEP_FAILED;
                status_code = StatusCode::EVMC_BAD_JUMP_DESTINATION;
                break;
            }
        }
    }

    stack.reverse();
    Ok(StepResult::new(
        step_status_code,
        status_code,
        revision,
        pc as u64,
        gas_left,
        gas_refund,
        output,
        // SAFETY
        // u256 is a newtype of Uint256 with repr(transparent) which guarantees the same memory
        // layout.
        unsafe { mem::transmute::<Vec<u256>, Vec<Uint256>>(stack) },
        memory,
        last_call_return_data,
    ))
}

fn push<const N: usize>(
    code: &[u8],
    pc: &mut usize,
    stack: &mut Vec<u256>,
    gas_left: &mut i64,
) -> Result<(), (StepStatusCode, StatusCode)> {
    consume_gas::<3>(gas_left)?;
    check_stack_overflow::<1>(stack)?;
    // Note: not tested by ct
    if code.len() < *pc + N {
        return Err((
            StepStatusCode::EVMC_STEP_FAILED,
            StatusCode::EVMC_INTERNAL_ERROR,
        ));
    }

    *pc += 1;
    stack.push(code[*pc..*pc + N].try_into().unwrap());
    *pc += N;

    Ok(())
}

fn dup<const N: usize>(
    pc: &mut usize,
    stack: &mut Vec<u256>,
    gas_left: &mut i64,
) -> Result<(), (StepStatusCode, StatusCode)> {
    consume_gas::<3>(gas_left)?;
    check_stack_overflow::<1>(stack)?;
    let nth = nth_ref_from_stack::<N>(stack)?;
    let nth = *nth;

    stack.push(nth);
    *pc += 1;

    Ok(())
}

fn swap<const N: usize>(
    pc: &mut usize,
    stack: &mut Vec<u256>,
    gas_left: &mut i64,
) -> Result<(), (StepStatusCode, StatusCode)> {
    consume_gas::<3>(gas_left)?;
    let len = stack.len();
    if len < N + 1 {
        return Err((
            StepStatusCode::EVMC_STEP_FAILED,
            StatusCode::EVMC_STACK_UNDERFLOW,
        ));
    }

    stack.swap(len - 1, len - 1 - N);
    *pc += 1;

    Ok(())
}

fn jump(
    dest: u256,
    pc: &mut usize,
    jump_destinations: &[usize],
) -> Result<(), (StepStatusCode, StatusCode)> {
    let dest_full = U256::from(dest);
    let dest = dest_full.digits()[0] as usize;
    // If the destination does not fit into u64 it is definitely to large, otherwise check if lowest
    // 64 bit are in jump_destinations.
    if dest_full > u64::MAX.into() || !jump_destinations.contains(&dest) {
        return Err((
            StepStatusCode::EVMC_STEP_FAILED,
            StatusCode::EVMC_BAD_JUMP_DESTINATION,
        ));
    }
    *pc = dest;

    Ok(())
}

#[inline(always)]
fn check_min_revision(
    min_revision: Revision,
    revision: Revision,
) -> Result<(), (StepStatusCode, StatusCode)> {
    if revision < min_revision {
        return Err((
            StepStatusCode::EVMC_STEP_FAILED,
            StatusCode::EVMC_INTERNAL_ERROR,
        ));
    }
    Ok(())
}

#[inline(always)]
fn consume_gas<const GAS: u64>(gas_left: &mut i64) -> Result<(), (StepStatusCode, StatusCode)> {
    if *gas_left < (GAS as i64) {
        return Err((
            StepStatusCode::EVMC_STEP_FAILED,
            StatusCode::EVMC_OUT_OF_GAS,
        ));
    }
    *gas_left -= GAS as i64;
    Ok(())
}

#[inline(always)]
fn consume_dyn_gas(gas_left: &mut i64, needed: u64) -> Result<(), (StepStatusCode, StatusCode)> {
    if *gas_left < (needed as i64) {
        return Err((
            StepStatusCode::EVMC_STEP_FAILED,
            StatusCode::EVMC_OUT_OF_GAS,
        ));
    }
    *gas_left -= needed as i64;
    Ok(())
}

#[inline(always)]
fn check_stack_overflow<const N: usize>(
    stack: &[u256],
) -> Result<(), (StepStatusCode, StatusCode)> {
    if stack.len() + N > 1024 {
        return Err((
            StepStatusCode::EVMC_STEP_FAILED,
            StatusCode::EVMC_STACK_OVERFLOW,
        ));
    }
    Ok(())
}

#[inline(always)]
fn pop_from_stack<const N: usize>(
    stack: &mut Vec<u256>,
) -> Result<[u256; N], (StepStatusCode, StatusCode)> {
    if stack.len() < N {
        return Err((
            StepStatusCode::EVMC_STEP_FAILED,
            StatusCode::EVMC_STACK_UNDERFLOW,
        ));
    }
    let mut array = [u256::ZERO; N];
    for element in &mut array {
        *element = stack.pop().unwrap();
    }

    Ok(array)
}

#[inline(always)]
fn nth_ref_from_stack<const N: usize>(
    stack: &[u256],
) -> Result<&u256, (StepStatusCode, StatusCode)> {
    if stack.len() < N {
        return Err((
            StepStatusCode::EVMC_STEP_FAILED,
            StatusCode::EVMC_STACK_UNDERFLOW,
        ));
    }

    Ok(&stack[stack.len() - N])
}
