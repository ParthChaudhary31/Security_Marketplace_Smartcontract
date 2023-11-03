make sure that you have cargo contract installed, 
do: cargo contract build --release

while testing the contracts, make sure to do:
    cargo test -- --nocapture
to get the println! outputs in console as well.



cargo contract instantiate --constructor new --args "{Token Address}" --suri //Alice
$(date +%s) --execute
Use this to instantiate.


cargo contract call --contract "{Token address}" --message get_current_audit_id --suri //Alice -x