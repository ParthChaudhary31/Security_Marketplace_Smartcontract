make sure that you have cargo contract installed, 
do: cargo contract build --skip-wasm-validation --release

we're skipping wasm validations for now since it throws:
    ERROR: An unexpected panic function import was found in the contract Wasm.
    This typically goes back to a known bug in the Rust compiler:
    https://github.com/rust-lang/rust/issues/78744

while testing the contracts, make sure to do:
    cargo test -- --nocapture
to get the println! outputs in console as well.



cargo contract instantiate     --constructor new --args "5FT5xGwGYQvosDfBoRvKJXfwBZzkYnDqxWUG319xBgMLZX43" --suri //Alice
$(date +%s) --execute
Used this to instantiate, maybe it worked, maybe it didn't. Let's find out.


cargo contract call --contract "5H8zt44h7wih2CSmUv9g7fPNM8YvUizSrKijHquKLgGK2RCg" --message get_current_audit_id --suri //Alice -x

