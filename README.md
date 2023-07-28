# **AuditBazaar**

![](https://github-production-user-asset-6210df.s3.amazonaws.com/106224868/255558416-ca967656-dd48-47ac-8d95-d48699ecdd58.svg)


# Cargo Contract Usage : 
You can always use `cargo contract help` to print information on available
commands and their usage.

##### `cargo contract build`

Compiles the contract into optimized WebAssembly bytecode, generates metadata for it,
and bundles both together in a `<name>.contract` file, which you can use for
deploying the contract on-chain.

##### `cargo contract check`

Checks that the code builds as WebAssembly. This command does not output any `<name>.contract`
artifact to the `target/` directory.

##### 'Cargo contract Test'

Runs the unit test cases for the specified contract.
