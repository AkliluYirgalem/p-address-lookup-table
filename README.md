<h1 align="center">
  <code>p-address-lookup-table</code>
</h1>
<p align="center">
  <img width="400" alt="p-address-lookup-table" src="https://github.com/user-attachments/assets/a5562235-261b-4c73-990c-d18437f9249b" />
</p>

<p align="center">
  A <code>pinocchio</code>-based address lookup table program.
</p>

## Overview

A re-implementation of the native [Address Lookup Table program](https://github.com/solana-program/address-lookup-table) using [`pinocchio`](https://github.com/anza-xyz/pinocchio) inspired by febo's [p-token](https://github.com/febo/p-token/) program.

## Features

- `no_std` crate
- Same instruction and account layout as the [native implementation](https://github.com/solana-program/address-lookup-table)
- Minimal CU usage
- Optimized binary size: ~155 KB → ~25 KB (≈83% reduction)

## Instructions

- [x] CreateLookupTable
- [x] FreezeLookupTable
- [x] ExtendLookupTable
- [x] DeactivateLookupTable
- [x] CloseLookupTable

## Compute Units

| Instruction             | Completed | CU (`p-address-lookup-table`) | CU (`native-address-lookup-table`) |
| ----------------------- | --------- | ----------------------------- | ---------------------------------- |
| `CreateLookupTable`     | ✅        | 3368                          | 10459                              |
| `FreezeLookupTable`     | ✅        | 207                           | 1762                               |
| `ExtendLookupTable`     | ✅        | 1986                          | 6331                               |
| `DeactivateLookupTable` | ✅        | 364                           | 2873                               |
| `CloseLookupTable`      | ✅        | 639                           | 2890                               |

## Building

To build the programs from the root directory of the repository:

```bash
cargo build-sbf --sbf-out-dir ./tests/fixtures/
```

## Testing

To run the tests:

```bash
cargo test -- --test-threads=1
```

## License

The code is licensed under the [Apache License Version 2.0](LICENSE)
