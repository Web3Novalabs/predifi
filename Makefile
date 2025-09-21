pr:
	scarb fmt
	scarb build
	snforge test

help:
	@echo "make pr   - Run formatting, build, and tests before PR" 