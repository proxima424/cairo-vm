#!/usr/bin/env sh

set -e

	
hyperfine \
	-n "cairo-rs (proof mode)" "target/release/cairo-vm-cli cairo_programs/benchmarks/fib.json --proof_mode --memory_file /dev/null --trace_file /dev/null --layout starknet_with_keccak" \
	-n "cairo-rs (NO proof mode)" "target/release/cairo-vm-cli cairo_programs/benchmarks/fib.json --layout starknet_with_keccak" \
	-n "Rust" "feb/target/release/feb" -w 10 -r 200



	# -n "Cairo VM (CPython)" "PYENV_VERSION=pypy3.9-7.3.9 cairo-run --proof_mode --memory_file /dev/null --trace_file /dev/null --layout starknet_with_keccak --program cairo_programs/benchmarks/fibonacci_1000_multirun.json" \
	# -n "Cairo VM (PyPy)" "PYENV_VERSION=pypy3.9-7.3.9 cairo-run --proof_mode --memory_file /dev/null --trace_file /dev/null --layout all --program cairo_programs/benchmarks/factorial_multirun.json" -w 1 -r 5