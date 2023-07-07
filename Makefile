RELBIN:=target/release/cairo-vm-run
DBGBIN:=target/debug/cairo-vm-run
STARKNET_COMPILE:=cairo/target/release/starknet-compile
STARKNET_SIERRA_COMPILE:=cairo/target/release/starknet-sierra-compile

.PHONY: build-cairo-1-compiler deps deps-macos cargo-deps build run check test clippy coverage benchmark flamegraph \
	compare_benchmarks_deps compare_benchmarks docs clean \
	compare_vm_output compare_trace_memory compare_trace compare_memory \
	compare_trace_memory_proof compare_trace_proof compare_memory_proof \
	cairo_bench_programs cairo_proof_programs cairo_test_programs \
	cairo_trace cairo-vm_trace cairo_proof_trace cairo-vm_proof_trace \
	$(RELBIN) $(DBGBIN) $(STARKNET_COMPILE) $(STARKNET_SIERRA_COMPILE) \
	example_programs

# Proof mode consumes too much memory with cairo-lang to execute
# two instances at the same time in the CI without getting killed
.NOTPARALLEL: $(CAIRO_TRACE_PROOF) $(CAIRO_MEM_PROOF)

# ===================
# Run with proof mode
# ===================

TEST_PROOF_DIR=cairo_programs/proof_programs
TEST_PROOF_FILES:=$(wildcard $(TEST_PROOF_DIR)/*.cairo)
COMPILED_PROOF_TESTS:=$(patsubst $(TEST_PROOF_DIR)/%.cairo, $(TEST_PROOF_DIR)/%.json, $(TEST_PROOF_FILES))
CAIRO_MEM_PROOF:=$(patsubst $(TEST_PROOF_DIR)/%.json, $(TEST_PROOF_DIR)/%.memory, $(COMPILED_PROOF_TESTS))
CAIRO_TRACE_PROOF:=$(patsubst $(TEST_PROOF_DIR)/%.json, $(TEST_PROOF_DIR)/%.trace, $(COMPILED_PROOF_TESTS))
CAIRO_RS_MEM_PROOF:=$(patsubst $(TEST_PROOF_DIR)/%.json, $(TEST_PROOF_DIR)/%.rs.memory, $(COMPILED_PROOF_TESTS))
CAIRO_RS_TRACE_PROOF:=$(patsubst $(TEST_PROOF_DIR)/%.json, $(TEST_PROOF_DIR)/%.rs.trace, $(COMPILED_PROOF_TESTS))

PROOF_BENCH_DIR=cairo_programs/benchmarks
PROOF_BENCH_FILES:=$(wildcard $(PROOF_BENCH_DIR)/*.cairo)
PROOF_COMPILED_BENCHES:=$(patsubst $(PROOF_BENCH_DIR)/%.cairo, $(PROOF_BENCH_DIR)/%.json, $(PROOF_BENCH_FILES))

$(TEST_PROOF_DIR)/%.json: $(TEST_PROOF_DIR)/%.cairo
	cairo-compile --cairo_path="$(TEST_PROOF_DIR):$(PROOF_BENCH_DIR)" $< --output $@ --proof_mode

$(TEST_PROOF_DIR)/%.rs.trace $(TEST_PROOF_DIR)/%.rs.memory: $(TEST_PROOF_DIR)/%.json $(RELBIN)
	cargo llvm-cov run -p cairo-vm-cli --release --no-report -- --layout starknet_with_keccak --proof_mode $< --trace_file $@ --memory_file $(@D)/$(*F).rs.memory

$(TEST_PROOF_DIR)/%.trace $(TEST_PROOF_DIR)/%.memory: $(TEST_PROOF_DIR)/%.json
	cairo-run --layout starknet_with_keccak --proof_mode --program $< --trace_file $@ --memory_file $(@D)/$(*F).memory

$(PROOF_BENCH_DIR)/%.json: $(PROOF_BENCH_DIR)/%.cairo
	cairo-compile --cairo_path="$(TEST_PROOF_DIR):$(PROOF_BENCH_DIR)" $< --output $@ --proof_mode

# ======================
# Run without proof mode
# ======================

TEST_DIR=cairo_programs
TEST_FILES:=$(wildcard $(TEST_DIR)/*.cairo)
COMPILED_TESTS:=$(patsubst $(TEST_DIR)/%.cairo, $(TEST_DIR)/%.json, $(TEST_FILES))
CAIRO_MEM:=$(patsubst $(TEST_DIR)/%.json, $(TEST_DIR)/%.memory, $(COMPILED_TESTS))
CAIRO_TRACE:=$(patsubst $(TEST_DIR)/%.json, $(TEST_DIR)/%.trace, $(COMPILED_TESTS))
CAIRO_RS_MEM:=$(patsubst $(TEST_DIR)/%.json, $(TEST_DIR)/%.rs.memory, $(COMPILED_TESTS))
CAIRO_RS_TRACE:=$(patsubst $(TEST_DIR)/%.json, $(TEST_DIR)/%.rs.trace, $(COMPILED_TESTS))

BENCH_DIR=cairo_programs/benchmarks
BENCH_FILES:=$(wildcard $(BENCH_DIR)/*.cairo)
COMPILED_BENCHES:=$(patsubst $(BENCH_DIR)/%.cairo, $(BENCH_DIR)/%.json, $(BENCH_FILES))

BAD_TEST_DIR=cairo_programs/bad_programs
BAD_TEST_FILES:=$(wildcard $(BAD_TEST_DIR)/*.cairo)
COMPILED_BAD_TESTS:=$(patsubst $(BAD_TEST_DIR)/%.cairo, $(BAD_TEST_DIR)/%.json, $(BAD_TEST_FILES))

NORETROCOMPAT_DIR:=cairo_programs/noretrocompat
NORETROCOMPAT_FILES:=$(wildcard $(NORETROCOMPAT_DIR)/*.cairo)
COMPILED_NORETROCOMPAT_TESTS:=$(patsubst $(NORETROCOMPAT_DIR)/%.cairo, $(NORETROCOMPAT_DIR)/%.json, $(NORETROCOMPAT_FILES))

$(BENCH_DIR)/%.json: $(BENCH_DIR)/%.cairo
	cairo-compile --cairo_path="$(TEST_DIR):$(BENCH_DIR)" $< --output $@ --proof_mode

$(TEST_DIR)/%.json: $(TEST_DIR)/%.cairo
	cairo-compile --cairo_path="$(TEST_DIR):$(BENCH_DIR)" $< --output $@

$(TEST_DIR)/%.rs.trace $(TEST_DIR)/%.rs.memory: $(TEST_DIR)/%.json $(RELBIN)
	cargo llvm-cov run -p cairo-vm-cli --release --no-report -- --layout all_cairo $< --trace_file $@ --memory_file $(@D)/$(*F).rs.memory

$(TEST_DIR)/%.trace $(TEST_DIR)/%.memory: $(TEST_DIR)/%.json
	cairo-run --layout starknet_with_keccak --program $< --trace_file $@ --memory_file $(@D)/$(*F).memory

$(NORETROCOMPAT_DIR)/%.json: $(NORETROCOMPAT_DIR)/%.cairo
	cairo-compile --cairo_path="$(TEST_DIR):$(BENCH_DIR):$(NORETROCOMPAT_DIR)" $< --output $@


BAD_TEST_DIR=cairo_programs/bad_programs
BAD_TEST_FILES:=$(wildcard $(BAD_TEST_DIR)/*.cairo)
COMPILED_BAD_TESTS:=$(patsubst $(BAD_TEST_DIR)/%.cairo, $(BAD_TEST_DIR)/%.json, $(BAD_TEST_FILES))


$(BAD_TEST_DIR)/%.json: $(BAD_TEST_DIR)/%.cairo
	cairo-compile $< --output $@

# ======================
# Test Cairo 1 Contracts
# ======================

CAIRO_1_CONTRACTS_TEST_DIR=cairo_programs/cairo-1-contracts
CAIRO_1_CONTRACTS_TEST_CAIRO_FILES:=$(wildcard $(CAIRO_1_CONTRACTS_TEST_DIR)/*.cairo)
COMPILED_SIERRA_CONTRACTS:=$(patsubst $(CAIRO_1_CONTRACTS_TEST_DIR)/%.cairo, $(CAIRO_1_CONTRACTS_TEST_DIR)/%.sierra, $(CAIRO_1_CONTRACTS_TEST_CAIRO_FILES))
COMPILED_CASM_CONTRACTS:= $(patsubst $(CAIRO_1_CONTRACTS_TEST_DIR)/%.sierra, $(CAIRO_1_CONTRACTS_TEST_DIR)/%.casm, $(COMPILED_SIERRA_CONTRACTS))

$(CAIRO_1_CONTRACTS_TEST_DIR)/%.sierra: $(CAIRO_1_CONTRACTS_TEST_DIR)/%.cairo
	$(STARKNET_COMPILE) --allowed-libfuncs-list-name experimental_v0.1.0 $< $@

$(CAIRO_1_CONTRACTS_TEST_DIR)/%.casm: $(CAIRO_1_CONTRACTS_TEST_DIR)/%.sierra
	$(STARKNET_SIERRA_COMPILE) --allowed-libfuncs-list-name experimental_v0.1.0 $< $@

cairo-repo-dir = cairo

build-cairo-1-compiler: | $(cairo-repo-dir)

$(cairo-repo-dir):
	git clone --depth 1 -b v1.1.0 https://github.com/starkware-libs/cairo.git
	cd cairo; cargo b --release --bin starknet-compile --bin starknet-sierra-compile

cargo-deps:
	cargo install --version 0.3.1 iai-callgrind-runner
	cargo install --version 1.1.0 cargo-criterion
	cargo install --version 0.6.1 flamegraph
	cargo install --version 1.14.0 hyperfine
	cargo install --version 0.9.49 cargo-nextest
	cargo install --version 0.5.9 cargo-llvm-cov
	cargo install --version 0.11.0 wasm-pack

deps: cargo-deps build-cairo-1-compiler
	pyenv install -s pypy3.9-7.3.9
	PYENV_VERSION=pypy3.9-7.3.9 python -m venv cairo-vm-pypy-env
	. cairo-vm-pypy-env/bin/activate ; \
	pip install -r requirements.txt ; \
	pyenv install -s 3.9.15
	PYENV_VERSION=3.9.15 python -m venv cairo-vm-env
	. cairo-vm-env/bin/activate ; \
	pip install -r requirements.txt ; \

deps-macos: cargo-deps build-cairo-1-compiler
	arch -x86_64 pyenv install -s pypy3.9-7.3.9
	PYENV_VERSION=pypy3.9-7.3.9 python -m venv cairo-vm-pypy-env
	. cairo-vm-pypy-env/bin/activate ; \
	CFLAGS=-I/opt/homebrew/opt/gmp/include LDFLAGS=-L/opt/homebrew/opt/gmp/lib pip install -r requirements.txt ; \
	pyenv install -s 3.9.15
	PYENV_VERSION=3.9.15 python -m venv cairo-vm-env
	. cairo-vm-env/bin/activate ; \
	CFLAGS=-I/opt/homebrew/opt/gmp/include LDFLAGS=-L/opt/homebrew/opt/gmp/lib pip install -r requirements.txt ; \

$(RELBIN):
	cargo build --release

build: $(RELBIN)

run:
	cargo run -p cairo-vm-cli

check:
	cargo check

examples/wasm-demo/src/array_sum.json: examples/wasm-demo/src/array_sum.cairo
	cairo-compile --no_debug_info examples/wasm-demo/src/array_sum.cairo \
		--output examples/wasm-demo/src/array_sum.json

example_programs: examples/wasm-demo/src/array_sum.json

cairo_test_programs: $(COMPILED_TESTS) $(COMPILED_BAD_TESTS) $(COMPILED_NORETROCOMPAT_TESTS) example_programs
cairo_proof_programs: $(COMPILED_PROOF_TESTS)
cairo_bench_programs: $(COMPILED_BENCHES)
cairo_1_test_contracts: $(COMPILED_CASM_CONTRACTS)

cairo_proof_trace: $(CAIRO_TRACE_PROOF) $(CAIRO_MEM_PROOF)
cairo-vm_proof_trace: $(CAIRO_RS_TRACE_PROOF) $(CAIRO_RS_MEM_PROOF)

cairo_trace: $(CAIRO_TRACE) $(CAIRO_MEM)
cairo-vm_trace: $(CAIRO_RS_TRACE) $(CAIRO_RS_MEM)

test: $(COMPILED_PROOF_TESTS) $(COMPILED_TESTS) $(COMPILED_BAD_TESTS) $(COMPILED_NORETROCOMPAT_TESTS) $(COMPILED_CASM_CONTRACTS)
	cargo llvm-cov nextest --no-report --workspace --features "test_utils, cairo-1-hints"
test-no_std: $(COMPILED_PROOF_TESTS) $(COMPILED_TESTS) $(COMPILED_BAD_TESTS) $(COMPILED_NORETROCOMPAT_TESTS)
	cargo llvm-cov nextest --no-report --workspace --features test_utils --no-default-features
test-wasm: $(COMPILED_PROOF_TESTS) $(COMPILED_TESTS) $(COMPILED_BAD_TESTS) $(COMPILED_NORETROCOMPAT_TESTS)
	# NOTE: release mode is needed to avoid "too many locals" error
	wasm-pack test --release --node vm --no-default-features

clippy: example_programs
	cargo clippy --workspace --all-features --benches --examples --tests -- -D warnings

coverage:
	cargo llvm-cov report --lcov --output-path lcov.info

coverage-clean:
	cargo llvm-cov clean

benchmark: $(COMPILED_BENCHES)
	cargo criterion --bench criterion_benchmark
	@echo 'Report: target/criterion/reports/index.html'

benchmark-action: $(COMPILED_BENCHES)
	cargo bench --bench criterion_benchmark -- --output-format bencher |sed 1d | tee output.txt

iai-benchmark-action: $(COMPILED_BENCHES)
	cargo bench --bench iai_benchmark

flamegraph:
	cargo flamegraph --root --bench criterion_benchmark -- --bench

compare_benchmarks: $(COMPILED_BENCHES)
	cd bench && ./run_benchmarks.sh

compare_trace_memory: $(CAIRO_RS_TRACE) $(CAIRO_TRACE) $(CAIRO_RS_MEM) $(CAIRO_MEM)
	cd src/tests; ./compare_vm_state.sh trace memory

compare_trace: $(CAIRO_RS_TRACE) $(CAIRO_TRACE)
	cd src/tests; ./compare_vm_state.sh trace

compare_memory: $(CAIRO_RS_MEM) $(CAIRO_MEM)
	cd src/tests; ./compare_vm_state.sh memory

compare_trace_memory_proof: $(COMPILED_PROOF_TESTS) $(CAIRO_RS_TRACE_PROOF) $(CAIRO_TRACE_PROOF) $(CAIRO_RS_MEM_PROOF) $(CAIRO_MEM_PROOF)
	cd src/tests; ./compare_vm_state.sh trace memory proof_mode

compare_trace_proof: $(CAIRO_RS_TRACE_PROOF) $(CAIRO_TRACE_PROOF)
	cd src/tests; ./compare_vm_state.sh trace proof_mode

compare_memory_proof: $(CAIRO_RS_MEM_PROOF) $(CAIRO_MEM_PROOF)
	cd src/tests; ./compare_vm_state.sh memory proof_mode

# Run with nightly enable the `doc_cfg` feature wich let us provide clear explaination about which parts of the code are behind a feature flag
docs:
	RUSTDOCFLAGS="--cfg docsrs" cargo +nightly doc --verbose --release --locked --no-deps --all-features --open

clean:
	rm -f $(TEST_DIR)/*.json
	rm -f $(TEST_DIR)/*.memory
	rm -f $(TEST_DIR)/*.trace
	rm -f $(BENCH_DIR)/*.json
	rm -f $(BAD_TEST_DIR)/*.json
	rm -f $(CAIRO_1_CONTRACTS_TEST_DIR)/*.sierra
	rm -f $(CAIRO_1_CONTRACTS_TEST_DIR)/*.casm
	rm -f $(TEST_PROOF_DIR)/*.json
	rm -f $(TEST_PROOF_DIR)/*.memory
	rm -f $(TEST_PROOF_DIR)/*.trace
	rm -rf cairo-vm-env
	rm -rf cairo-vm-pypy-env
	rm -rf cairo
