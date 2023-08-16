make sure that you have cargo contract installed, 
do: cargo contract build --skip-wasm-validation --release

we're skipping wasm validations for now since it throws:
    ERROR: An unexpected panic function import was found in the contract Wasm.
    This typically goes back to a known bug in the Rust compiler:
    https://github.com/rust-lang/rust/issues/78744

while testing the contracts, make sure to do:
    cargo test -- --nocapture
to get the println! outputs in console as well.



cargo contract instantiate --constructor new --args "{Token Address}" --suri //Alice
$(date +%s) --execute
Use this to instantiate.


cargo contract call --contract "{Token address}" --message get_current_audit_id --suri //Alice -x