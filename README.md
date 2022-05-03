# Operate this repo:

1. From the project root run
`npm i`

2. Use `cargo check` and `cargo clipply` to check your code changes if you make ones.
   
3. Compile and optimize your contract:
   Use `cargo wasm` to compile your contract
   Then run
   `docker run --rm -v "$(pwd)":/contract --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry enigmampc/secret-contract-optimizer:1.0.6`
   in order to optimize and get the file `contract.wasm.gz`
   
4. Once you have `contract.wasm.gz` in your root folder, run `npm run test` from the project root directory, in order to run the integration tests that are in the file: `__tests__/integration.ts`
