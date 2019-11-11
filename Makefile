.PHONY: all

all:
	mdbook build
	cp -f ./book/singlepage/README.md .
