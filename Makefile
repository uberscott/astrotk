all:
	$(MAKE) -C rust/mechtron_core
	$(MAKE) -C wasm/example
