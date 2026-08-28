CARGO ?= cargo
PYTHON ?= python3
INSTALL ?= install

PREFIX ?= /usr
DESTDIR ?=
BINDIR ?= $(PREFIX)/bin
LIBDIR ?= $(PREFIX)/lib/emane
DATADIR ?= $(PREFIX)/share/emane
DTDDIR ?= $(DATADIR)/dtd
SCHEMADIR ?= $(DATADIR)/schema
MANIFESTDIR ?= $(DATADIR)/manifest

TARGET_DIR ?= target
RELEASE_DIR := $(TARGET_DIR)/release
MANIFEST_BUILD_DIR ?= $(TARGET_DIR)/manifests
WHEEL_BUILD_DIR ?= $(TARGET_DIR)/wheels
PYTHON_STAGE_DIR ?= $(TARGET_DIR)/python-install
PYTHON_WHEEL = $(firstword $(wildcard $(WHEEL_BUILD_DIR)/emane-*.whl))
PYTHON_SITE_DIR ?= $(shell $(PYTHON) -c 'import sysconfig; print(sysconfig.get_path("purelib", vars={"base": "$(PREFIX)", "platbase": "$(PREFIX)"}))')

RUST_BINS := emane emaneeventd emaneeventservice emaneinfo emanetransportd
RUST_PLUGINS := \
	libbentpipe.so \
	libbypassmaclayer.so \
	libbypassphylayer.so \
	libcommeffectshim.so \
	libdummy_mac.so \
	libieee80211abg.so \
	libphyapitestshim.so \
	librfpipe.so \
	libtdma.so \
	libtiminganalysisshim.so

.PHONY: all build release check test clean manifests python-wheel python-stage \
	prepare-install check-install-artifacts install install-rust install-data \
	install-manifests install-python installcheck

all: build

build:
	$(CARGO) build --workspace

release:
	$(CARGO) build --release --workspace

check:
	$(CARGO) check --workspace

test:
	$(CARGO) test --workspace

clean:
	$(CARGO) clean

manifests: release
	EMANEINFO="$(abspath $(RELEASE_DIR)/emaneinfo)" \
	EMANE_PLUGIN_PATH="$(abspath $(RELEASE_DIR))" \
	EMANE_MANIFEST_SCHEMA="$(abspath schema/manifest.xsd)" \
		scripts/emanegenmanifests.sh "$(abspath $(MANIFEST_BUILD_DIR))"

python-wheel:
	$(INSTALL) -d "$(WHEEL_BUILD_DIR)"
	find "$(WHEEL_BUILD_DIR)" -maxdepth 1 -type f -name 'emane-*.whl' -delete
	$(PYTHON) -m pip wheel \
		--no-cache-dir --no-deps --no-build-isolation \
		--wheel-dir "$(WHEEL_BUILD_DIR)" src/python

python-stage: python-wheel
	$(INSTALL) -d "$(PYTHON_STAGE_DIR)"
	find "$(PYTHON_STAGE_DIR)" -mindepth 1 -delete
	$(PYTHON) -m pip install \
		--no-cache-dir --no-deps --no-build-isolation --no-compile --ignore-installed \
		--prefix "$(PREFIX)" --root "$(abspath $(PYTHON_STAGE_DIR))" \
		"$(PYTHON_WHEEL)"

prepare-install: manifests python-stage

check-install-artifacts:
	@set -eu; missing=0; \
	for artifact in \
		$(addprefix $(RELEASE_DIR)/,$(RUST_BINS)) \
		$(addprefix $(RELEASE_DIR)/,$(RUST_PLUGINS)); do \
		if test ! -f "$$artifact"; then echo "missing install artifact: $$artifact" >&2; missing=1; fi; \
	done; \
	test -f "$(PYTHON_STAGE_DIR)$(PREFIX)/bin/emanesh" || { echo "missing install artifacts: staged Python installation" >&2; missing=1; }; \
	test -n "$$(find "$(PYTHON_STAGE_DIR)$(PREFIX)" -type f -path '*/site-packages/emane/shell/schema/manifest.xsd' -print -quit 2>/dev/null)" || { echo "missing install artifact: Python manifest schema" >&2; missing=1; }; \
	test -f "$(MANIFEST_BUILD_DIR)/emanephy.xml" || { echo "missing install artifacts: generated manifests" >&2; missing=1; }; \
	if test "$$missing" -ne 0; then echo "run 'make prepare-install' first" >&2; exit 1; fi

# Keep compilation outside sudo: this target only installs artifacts prepared
# by `make prepare-install` in the invoking user's build environment.
install: check-install-artifacts
	+$(MAKE) install-rust install-data install-manifests install-python

install-rust:
	$(INSTALL) -d "$(DESTDIR)$(BINDIR)" "$(DESTDIR)$(LIBDIR)"
	$(INSTALL) -m 0755 $(addprefix $(RELEASE_DIR)/,$(RUST_BINS)) "$(DESTDIR)$(BINDIR)/"
	$(INSTALL) -m 0755 $(addprefix $(RELEASE_DIR)/,$(RUST_PLUGINS)) "$(DESTDIR)$(LIBDIR)/"

install-data:
	$(INSTALL) -d "$(DESTDIR)$(DTDDIR)" "$(DESTDIR)$(SCHEMADIR)"
	$(INSTALL) -m 0644 dtd/*.dtd dtd/*.ent "$(DESTDIR)$(DTDDIR)/"
	$(INSTALL) -m 0644 schema/*.xsd "$(DESTDIR)$(SCHEMADIR)/"

install-manifests:
	$(INSTALL) -d "$(DESTDIR)$(MANIFESTDIR)"
	$(INSTALL) -m 0644 $(MANIFEST_BUILD_DIR)/*.xml "$(DESTDIR)$(MANIFESTDIR)/"

install-python:
	$(INSTALL) -d "$(DESTDIR)$(PREFIX)"
	cp -R "$(PYTHON_STAGE_DIR)$(PREFIX)/." "$(DESTDIR)$(PREFIX)/"

# Check an already-installed tree, normally one staged with DESTDIR.
installcheck:
	@set -eu; \
	for program in $(RUST_BINS); do test -x "$(DESTDIR)$(BINDIR)/$$program"; done; \
	for plugin in $(RUST_PLUGINS); do test -x "$(DESTDIR)$(LIBDIR)/$$plugin"; done; \
	test -f "$(DESTDIR)$(SCHEMADIR)/manifest.xsd"; \
	test -f "$(DESTDIR)$(MANIFESTDIR)/emanephy.xml"; \
	test -n "$$(find "$(DESTDIR)$(PREFIX)" -type f -path '*/site-packages/emane/shell/schema/manifest.xsd' -print -quit)"; \
	"$(DESTDIR)$(BINDIR)/emaneinfo" --manifest ieee80211abgmaclayer >/dev/null; \
	$(PYTHON) scripts/check-manifests.py "$(DESTDIR)$(MANIFESTDIR)"; \
	PYTHONPATH="$(DESTDIR)$(PYTHON_SITE_DIR)" $(PYTHON) -c 'from pathlib import Path; from emane.shell import Manifest; paths=Path("$(DESTDIR)$(MANIFESTDIR)").glob("*.xml"); assert len([Manifest(str(path)) for path in paths]) == 17'
